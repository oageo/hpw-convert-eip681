//! EIP-681（ERC-681）URI → [`ParsedLink`] の逆方向パーサー。
//!
//! 受け付ける文法（EIP-681の部分集合）:
//!
//! ```text
//! ethereum:[pay-]<token>@<chain_id>/transfer?<params>
//! ```
//!
//! このパーサーの出力はHashPort決済リンクの生成に使われる。つまり送金先・
//! 金額・チェーンを決めるものなので、解釈が割れうる入力は「それらしく
//! 補完する」のではなく、すべてエラーにする方針を取る。
//!
//! チェック順（最初に見つかったエラーを返す）と、各厳格化の理由:
//!
//! 1. 文字レベルの検査: ASCIIの可視文字（0x21〜0x7E）以外を含む入力、および
//!    フラグメント（`#`）を含む入力を `InvalidEip681` とする。前後の空白を
//!    トリムしないのは、コピペ由来の混入物を黙って受け入れないため。
//!    非ASCII文字はURIとしてパーセントエンコードされているべきであり、
//!    生のまま許すとホモグリフ等の紛らわしさが残るため拒否する。
//! 2. スキーム: `ethereum:` を大文字小文字を問わず照合（RFC 3986でスキームは
//!    大文字小文字を区別しない）。続く任意の `pay-` も同様（ABNFの文字列
//!    リテラルは大文字小文字を区別しないため）。
//! 3. 構造分解: `@`・`/`・`?` の最初の出現位置でトークン・チェーンID・
//!    関数名・パラメータに分ける。パス部分はパーセントデコードしない
//!    （`%` を含めば下記の各検査で不正として弾かれる）。
//! 4. トークンアドレス: `0x` + 40桁16進のみ。大文字小文字混在ならEIP-55
//!    チェックサム必須（`InvalidAddress`）。ENS名は名前解決にネットワーク
//!    アクセスが必要になるため拒否する。対応表にないコントラクトは
//!    `UnsupportedContract`。
//! 5. チェーンID: 必須（なければ `MissingParam { name: "chain_id" }`）。
//!    EIP-681では省略時「ウォレットの現在のネットワーク」を意味するが、
//!    それは `master_currency_id` に対応付けられず、メインネット等に黙って
//!    決め打ちすると別チェーンで支払わせる危険があるため。`1*DIGIT` で
//!    符号なし・先頭ゼロなし（`0137` は8進と誤読されうるため）。構文違反は
//!    `InvalidEip681`、u64に収まらない値や（通貨, チェーン）の組が対応表に
//!    ないものは `UnsupportedChain`。
//! 6. 関数名: 厳密に `transfer`（大文字小文字を区別）。関数呼び出しが
//!    なければネイティブ通貨送金なので `UnsupportedFunction { name: "" }`。
//!    `/` の後が空の場合は構文エラー（`InvalidEip681`）として区別する。
//! 7. パラメータ: `&` で分割し、各要素を最初の `=` で分ける。キー・値とも
//!    パーセントデコードする（不正な `%` エスケープや非UTF-8は
//!    `InvalidEip681`）。`+` を空白とみなす `application/x-www-form-urlencoded`
//!    のデコードは使わない（`+` はEIP-681の数値文法で符号として意味を持つ）。
//!    空要素・`=` なしは `InvalidEip681`。同じキーの重複はデコード後の
//!    キーで判定して `DuplicateParam`（先勝ち/後勝ちがウォレットごとに
//!    異なり、曖昧さが攻撃面になるため）。`address`（必須）・`uint256`
//!    （任意）以外では、ガス関連のヒント `gas`・`gasLimit`・`gasPrice` のみ
//!    受け付けて無視する（HashPortリンクでは表現できないが、支払い内容は
//!    変えないため）。`value` はネイティブ通貨を同時に送る指定であり、
//!    捨てると支払いの意味が変わるため `UnsupportedParam`。その他の未知の
//!    キーも、意味を黙って捨てないよう `UnsupportedParam`。
//! 8. 受取人アドレス（`address=`）: 4. と同じ規則。加えて、ゼロアドレスと
//!    対応トークンのコントラクト自身は `UnsafeRecipient` とする（送金が
//!    失敗するか資金が永久に失われるだけで正当な用途がないため）。金額の
//!    検査（9.）より先に行う。
//! 9. 金額（`uint256=`）: EIP-681の数値文法の部分集合
//!    `["+"] ("0" / NZDIGIT *DIGIT) ["." 1*DIGIT] [("e"/"E") ("0" / NZDIGIT *DIGIT)]`
//!    （整数部は省略して `.5e1` としてもよい）で、値が1以上かつU256に収まる
//!    整数のみ受け付け、違反は `InvalidAmount`。整数部・指数の先頭ゼロ
//!    （`01`、`1e018`）は5. と同じく8進との誤読を避けるため拒否。値0は、
//!    0の送金でも `Transfer` イベントが出て入金と誤認されうるため拒否
//!    （金額未指定なら `uint256` を省略する）。詳細は後述の
//!    [`parse_amount_decimal`] を参照。省略時は `amount: None`。
//!
//! どのような入力に対してもパニックしない。

use alloy_primitives::U256;
use percent_encoding::percent_decode_str;

use crate::address;
use crate::currency::Currency;
use crate::error::ParseError;
use crate::link::{ChainId, ParsedLink};

const SCHEME: &str = "ethereum:";
const PAY_PREFIX: &str = "pay-";
const TRANSFER: &str = "transfer";

/// U256の最大値（2^256 - 1 ≈ 1.16 × 10^77）の10進桁数。
const U256_MAX_DECIMAL_DIGITS: usize = 78;

/// EIP-681（ERC-681）形式のERC-20送金URIをパースし、[`ParsedLink`] に
/// 変換する。[`crate::parse`] の逆方向の入口。
///
/// EIP-681のURIはHashPortリンクの `to_name`・`type` に相当する情報を
/// 持たないため、結果の `to_name`・`link_type` は常に `None` となる。
pub fn parse_eip681(input: &str) -> Result<ParsedLink, ParseError> {
    // 1. 文字レベルの検査
    if let Some(c) = input.chars().find(|c| !c.is_ascii_graphic()) {
        return Err(invalid(format!(
            "contains whitespace, control or non-ASCII character {c:?}"
        )));
    }
    if input.contains('#') {
        return Err(invalid("fragments (#) are not allowed"));
    }

    // 2. スキームと任意の `pay-` プレフィックス。`get` はバイト境界が文字境界で
    //    ない場合に `None` を返すため、スライスでパニックしない
    //    （ここでは既にASCIIのみであることを確認済みだが念のため）。
    let rest = strip_prefix_ignore_ascii_case(input, SCHEME)
        .ok_or_else(|| invalid("scheme must be \"ethereum:\""))?;
    let rest = strip_prefix_ignore_ascii_case(rest, PAY_PREFIX).unwrap_or(rest);

    // 3. 構造分解
    let parts = split_uri(rest);

    // 4. トークンコントラクト
    let token = address::parse_address_eip55_if_mixed(parts.target)?;
    let currency =
        Currency::from_contract_address(token).ok_or_else(|| ParseError::UnsupportedContract {
            address: parts.target.to_string(),
        })?;

    // 5. チェーンID
    let chain_raw = parts
        .chain_id
        .ok_or(ParseError::MissingParam { name: "chain_id" })?;
    let chain_id = parse_chain_id(chain_raw)?;
    if currency.master_currency_id(chain_id).is_none() {
        return Err(ParseError::UnsupportedChain {
            chain_id: chain_raw.to_string(),
        });
    }

    // 6. 関数名
    match parts.function {
        None => {
            return Err(ParseError::UnsupportedFunction {
                name: String::new(),
            });
        }
        Some("") => return Err(invalid("empty function name")),
        Some(TRANSFER) => {}
        Some(other) => {
            return Err(ParseError::UnsupportedFunction {
                name: other.to_string(),
            });
        }
    }

    // 7. パラメータ
    let params = parse_params(parts.query)?;

    // 8. 受取人アドレス
    let to_raw = params
        .address
        .ok_or(ParseError::MissingParam { name: "address" })?;
    let to = address::parse_address_eip55_if_mixed(&to_raw)?;
    address::check_recipient(to)?;

    // 9. 金額
    let amount = params
        .uint256
        .map(|raw| parse_amount_decimal(&raw))
        .transpose()?;

    Ok(ParsedLink {
        to,
        currency,
        chain_id,
        amount,
        to_name: None,
        link_type: None,
    })
}

fn invalid(reason: impl Into<String>) -> ParseError {
    ParseError::InvalidEip681 {
        reason: reason.into(),
    }
}

fn strip_prefix_ignore_ascii_case<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        s.get(prefix.len()..)
    } else {
        None
    }
}

/// スキーム（と `pay-`）を除いた残りを構造的に分解したもの。各要素は
/// 区切り文字を含まない生の文字列。`None` は区切り文字自体がなかった
/// （要素が省略された）ことを、`Some("")` は区切り文字はあるが中身が
/// 空であることを表す。
struct UriParts<'a> {
    target: &'a str,
    chain_id: Option<&'a str>,
    function: Option<&'a str>,
    query: Option<&'a str>,
}

/// `<target>[@<chain_id>][/<function>][?<query>]` を分解する。
/// 区切り文字はすべてASCIIなので、`find` が返す位置は常に文字境界である。
fn split_uri(rest: &str) -> UriParts<'_> {
    let (before_query, query) = match rest.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (rest, None),
    };
    // 関数名は `@<chain_id>` の後ろに来るため、トークン部分を切り出してから
    // 最初の `/` を探す（`/` が `@` より前にある場合、その `@` は関数名の
    // 一部として扱われ、チェーンIDは省略とみなされる）。
    let (before_function, function) = match before_query.split_once('/') {
        Some((b, f)) => (b, Some(f)),
        None => (before_query, None),
    };
    let (target, chain_id) = match before_function.split_once('@') {
        Some((t, c)) => (t, Some(c)),
        None => (before_function, None),
    };
    UriParts {
        target,
        chain_id,
        function,
        query,
    }
}

/// チェーンIDを `1*DIGIT`（符号なし・先頭ゼロなし）として解釈する。
fn parse_chain_id(raw: &str) -> Result<ChainId, ParseError> {
    if raw.is_empty() {
        return Err(invalid("empty chain id"));
    }
    if !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid(format!("chain id must be decimal digits: {raw}")));
    }
    if raw.len() > 1 && raw.starts_with('0') {
        return Err(invalid(format!(
            "chain id must not have leading zeros: {raw}"
        )));
    }
    // 数字のみであることは確認済みなので、ここで失敗するのはu64の範囲を
    // 超える場合のみ。そのようなチェーンには当然対応していない。
    raw.parse::<u64>()
        .map(ChainId)
        .map_err(|_| ParseError::UnsupportedChain {
            chain_id: raw.to_string(),
        })
}

/// パース済みのパラメータ。値はパーセントデコード済み。
#[derive(Default)]
struct Params {
    address: Option<String>,
    uint256: Option<String>,
}

fn parse_params(query: Option<&str>) -> Result<Params, ParseError> {
    let mut params = Params::default();
    let Some(query) = query else {
        return Ok(params);
    };

    // 重複検出用。パラメータ数は入力長に比例する程度なので線形探索で十分。
    let mut seen: Vec<String> = Vec::new();
    for segment in query.split('&') {
        if segment.is_empty() {
            return Err(invalid("empty query parameter"));
        }
        let (raw_key, raw_value) = segment
            .split_once('=')
            .ok_or_else(|| invalid(format!("query parameter without '=': {segment}")))?;
        let key = percent_decode_strict(raw_key)?;
        let value = percent_decode_strict(raw_value)?;

        if seen.contains(&key) {
            return Err(ParseError::DuplicateParam { name: key });
        }

        match key.as_str() {
            "address" => params.address = Some(value),
            "uint256" => params.uint256 = Some(value),
            // ガス関連のヒントは支払い内容を変えないため受け付けて無視する。
            "gas" | "gasLimit" | "gasPrice" => {}
            // `value` はネイティブ通貨の同時送金を意味し、捨てると支払いの
            // 意味が変わる。その他の未知のキーも同様に黙って捨てない。
            _ => return Err(ParseError::UnsupportedParam { name: key }),
        }
        seen.push(key);
    }
    Ok(params)
}

/// `%XX` 形式のパーセントデコードを行う。`percent_decode_str` は不正な
/// エスケープ（`%zz` や末尾の `%`）をそのまま素通しするため、曖昧さを
/// 残さないよう事前に検査してエラーにする。デコード結果は有効なUTF-8で
/// なければならない。`+` は空白に変換しない。
fn percent_decode_strict(raw: &str) -> Result<String, ParseError> {
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let ok = bytes
                .get(i + 1..i + 3)
                .is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit));
            if !ok {
                return Err(invalid(format!("malformed percent-encoding: {raw}")));
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    percent_decode_str(raw)
        .decode_utf8()
        .map(|s| s.into_owned())
        .map_err(|_| invalid(format!("percent-decoded value is not valid UTF-8: {raw}")))
}

/// `uint256=` の値を、EIP-681の数値文法で最小単位の整数としてパースする。
///
/// ```text
/// number = [ "-" / "+" ] *DIGIT [ "." 1*DIGIT ] [ ( "e" / "E" ) [ 1*DIGIT ] ]
/// ```
///
/// 上記のうち実際に受け付けるのは次の部分集合（`NZDIGIT` = `1`〜`9`）:
///
/// ```text
/// amount   = [ "+" ] mantissa [ ( "e" / "E" ) exponent ]
/// mantissa = integer [ "." 1*DIGIT ] / "." 1*DIGIT
/// integer  = "0" / NZDIGIT *DIGIT
/// exponent = "0" / NZDIGIT *DIGIT
/// ```
///
/// さらに値は 1 以上かつU256に収まる整数でなければならない。各規則の詳細:
///
/// - 先頭の `+` は許可、`-` は負数なので拒否。
/// - 仮数部（整数部と小数部の合計）に少なくとも1桁の数字が必要（`+`、`e18`、
///   `.e1` は拒否）。整数部は空でもよい（`.5e1` = 5）。
/// - 整数部・指数とも、2桁以上なら先頭ゼロを拒否する（`01`、`007`、`00.5e1`、
///   `1e018`、`1e00` は拒否。`0.5e1` = 5 や `1e0` = 1 のような単独の `0` は可）。
///   チェーンIDと同じ方針で、先頭ゼロの数値を8進として読むパーサーがあり、
///   ウォレットごとに解釈が割れうるため。小数部の桁は対象外（`1.50e1` = 15）。
/// - `.` の後には1桁以上の数字が必要（`1.` は拒否）。
/// - `e`/`E` の後には1桁以上の数字が必要（`1e` は拒否。文法上は許されるが
///   意味が曖昧なため）。指数の符号（`e-1`、`e+1`）は文法にないため拒否。
/// - `0x10` 等の16進は文法外として拒否する（決して16進として再解釈しない）。
/// - 結果はU256に収まる1以上の整数でなければならない（`2.014e18` は可、
///   `1.5` は整数にならないので拒否）。
/// - 値が0になるもの（`0`、`+0`、`0.0`、`.0`、`0e5`、
///   `0e99999999999999999999` など）は拒否する。0のERC-20送金もオンチェーンでは
///   成功して `Transfer` イベントを発行するため、イベントを監視する受取側の
///   システムが入金と誤認しうる。金額未指定を意図するなら `uint256` 自体を
///   省略すべきである。なお順方向（HashPortリンクの `amount=0x0`）は従来どおり
///   受け付ける。
/// - 空文字列は拒否する。ruintの `from_str_radix` は空文字列に `Ok(0)` を
///   返すため、ここで明示的に弾かないと金額0として通ってしまう。
///
/// 検査順は、構文（先頭ゼロを含む）→ 0の判定 → 整数性・U256範囲の判定。
///
/// DoS対策: 攻撃者が指定した巨大な指数で10^expを計算することは決してしない。
/// 仮数の有効数字（先頭・末尾のゼロを除いた桁）と実効指数だけを見て、
/// 結果の桁数がU256の最大桁数を超えるかどうかを先に判定するため、
/// `1e99999999999999999999` や数千桁の入力でも入力長に線形な時間で終わる。
/// 仮数がゼロなら指数を評価する前に0として拒否する
/// （`0e99999999999999999999` も即座に弾かれる）。
fn parse_amount_decimal(raw: &str) -> Result<U256, ParseError> {
    let err = |reason: &str| ParseError::InvalidAmount {
        value: raw.to_string(),
        reason: reason.to_string(),
    };

    if raw.is_empty() {
        return Err(err("empty amount"));
    }

    // --- 字句解析 ---
    let mut s = raw;
    if let Some(r) = s.strip_prefix('+') {
        s = r;
    } else if s.starts_with('-') {
        return Err(err("negative amount"));
    }

    let int_len = s.bytes().take_while(u8::is_ascii_digit).count();
    let (int_digits, mut s) = s.split_at(int_len);

    let mut frac_digits = "";
    if let Some(r) = s.strip_prefix('.') {
        let frac_len = r.bytes().take_while(u8::is_ascii_digit).count();
        if frac_len == 0 {
            return Err(err("'.' must be followed by at least one digit"));
        }
        (frac_digits, s) = r.split_at(frac_len);
    }

    if int_digits.is_empty() && frac_digits.is_empty() {
        return Err(err("missing digits"));
    }

    let mut exp_digits = None;
    if let Some(r) = s.strip_prefix(['e', 'E']) {
        let exp_len = r.bytes().take_while(u8::is_ascii_digit).count();
        if exp_len == 0 {
            return Err(err("exponent must be followed by at least one digit"));
        }
        let (e, r) = r.split_at(exp_len);
        exp_digits = Some(e);
        s = r;
    }

    if !s.is_empty() {
        return Err(err("unexpected character in decimal number"));
    }

    // 整数部・指数の先頭ゼロ（2桁以上で `0` 始まり）を拒否する。単独の `0` は可。
    // 8進として読むパーサーとの解釈の食い違いを避けるため（チェーンIDと同じ方針）。
    let has_leading_zero = |d: &str| d.len() > 1 && d.starts_with('0');
    if has_leading_zero(int_digits) || exp_digits.is_some_and(has_leading_zero) {
        return Err(err("leading zeros are not allowed"));
    }

    // --- 値の計算 ---
    // 仮数の全桁 = 整数部 ++ 小数部。値 = 仮数 × 10^(指数 - 小数部の桁数)。
    // 先頭ゼロは上で拒否済みなので、整数部は空・単独の `0`・非ゼロ始まりの
    // いずれか。単独の `0` は有効数字を持たない。
    let int_sig = if int_digits == "0" { "" } else { int_digits };
    let (digits_head, digits_tail) = if int_sig.is_empty() {
        // 整数部が空か `0` なら、小数部の先頭ゼロも有効数字ではない。
        let frac_zeros = frac_digits.bytes().take_while(|&b| b == b'0').count();
        ("", &frac_digits[frac_zeros..])
    } else {
        (int_sig, frac_digits)
    };
    // 有効数字が空なら、指数に関係なく値は0。指数を評価する前に拒否するので、
    // `0e99999999999999999999` のような入力も入力長に線形な時間で弾かれる。
    // 0のERC-20送金も成功して `Transfer` イベントを発行し、受取側が入金と
    // 誤認しうるため、金額未指定なら `uint256` 自体を省略させる。
    if digits_head.is_empty() && digits_tail.is_empty() {
        return Err(err(
            "amount must be greater than zero (omit uint256 for an unspecified amount)",
        ));
    }

    // 末尾のゼロを取り除き、その分を指数に繰り入れる。こうすると
    // 「有効数字 × 10^実効指数」が整数であることと実効指数が非負であることが
    // 同値になる。
    let trailing_zeros = |d: &str| d.bytes().rev().take_while(|&b| b == b'0').count();
    let (head, tail, stripped) = if digits_tail.bytes().all(|b| b == b'0') {
        let tz = trailing_zeros(digits_head);
        (
            &digits_head[..digits_head.len() - tz],
            "",
            digits_tail.len() + tz,
        )
    } else {
        let tz = trailing_zeros(digits_tail);
        (digits_head, &digits_tail[..digits_tail.len() - tz], tz)
    };
    let significant_len = head.len() + tail.len();

    // 指数を有界に評価する。指数は空でなく先頭ゼロもない（検査済み）ので、
    // `parse` が失敗するのはu64を超える場合のみ。そのような指数は、
    // 入力長（≤ isize::MAX）に由来する小数部の桁数をどう差し引いても
    // U256の桁数を大きく超え、かつ仮数は非ゼロと確認済みなので、
    // 値を計算せずに「大きすぎる」とする。
    let exponent: i128 = match exp_digits {
        None => 0,
        Some(e) => match e.parse::<u64>() {
            Ok(v) => i128::from(v),
            Err(_) => return Err(err("amount does not fit in uint256")),
        },
    };
    // usize → i128 は64ビット環境でも損失なく変換できる。
    let to_i128 = |n: usize| i128::try_from(n).unwrap_or(i128::MAX);
    let effective_exp = exponent + to_i128(stripped) - to_i128(frac_digits.len());

    if effective_exp < 0 {
        return Err(err("amount is not an integer"));
    }
    if to_i128(significant_len) + effective_exp > to_i128(U256_MAX_DECIMAL_DIGITS) {
        return Err(err("amount does not fit in uint256"));
    }

    // ここに到達した時点で、有効数字は高々78桁、実効指数は高々77なので、
    // 以下の計算はいずれも有界である。
    let mut significant = String::with_capacity(significant_len);
    significant.push_str(head);
    significant.push_str(tail);
    let mut value = U256::from_str_radix(&significant, 10)
        .map_err(|_| err("amount does not fit in uint256"))?;
    for _ in 0..effective_exp {
        value = value
            .checked_mul(U256::from(10u8))
            .ok_or_else(|| err("amount does not fit in uint256"))?;
    }
    Ok(value)
}
