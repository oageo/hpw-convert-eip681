use alloy_primitives::U256;

use crate::error::ParseError;

/// URL側の `amount` クエリ値（`0x`/`0X` プレフィックスの有無が不定な、
/// 16進エンコードされたuint256）をデコードする。
///
/// `U256::from_str`（`ruint` の素の `FromStr` 実装）は `0x`/`0o`/`0b`
/// プレフィックスから基数を推測し、プレフィックスがなければ10進とみなす。
/// しかし本仕様では `amount` はプレフィックスなしの16進としても届き得るため、
/// プレフィックスなしの数字だけの値（例: "1000"）に対して素の `FromStr` を
/// 使うと、本来16進0x1000（4096）であるべき値を10進1000として誤って
/// パースしてしまう（エラーにすらならない値破壊バグになる）。そのため、
/// 必ずプレフィックスを手動で取り除いたうえで基数16を明示して
/// `from_str_radix` を呼ぶ。
pub fn parse_amount_hex(raw: &str) -> Result<U256, ParseError> {
    let trimmed = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);

    // この空文字チェックは必須である（単なるエラーメッセージ改善ではない）。
    // ruintの `from_str_radix` は空文字列に対してエラーではなく `Ok(0)` を
    // 返すため、このチェックがないと `amount=` や `amount=0x` が静かに
    // 金額0として解釈されてしまう。
    if trimmed.is_empty() {
        return Err(ParseError::InvalidAmount {
            value: raw.to_string(),
            reason: "empty amount".to_string(),
        });
    }

    // 16進数字以外を事前に弾くこのチェックも必須である。ruintの
    // `from_str_radix` は桁区切りとして `_` を読み飛ばすため、これがないと
    // `amount=_` や `amount=0x__` が上の空文字チェックをすり抜けて金額0と
    // 解釈され、`amount=0x1_000` も黙って受理されてしまう。
    if !trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ParseError::InvalidAmount {
            value: raw.to_string(),
            reason: "amount must consist of hex digits only".to_string(),
        });
    }

    U256::from_str_radix(trimmed, 16).map_err(|e| ParseError::InvalidAmount {
        value: raw.to_string(),
        reason: e.to_string(),
    })
}

/// `U256` の金額を、HashPort Wallet決済リンクの `amount` クエリ値の形式
/// （`0x` + 小文字16進64桁、ゼロ埋め）にエンコードする。
/// [`parse_amount_hex`] の逆変換であり、HashPort Walletが実際に生成する
/// リンクと同じ固定長の表記に合わせている。
///
/// `{:#066x}` の幅66は `0x` プレフィックス2文字を含む。ruintの `LowerHex`
/// 実装は `Formatter::pad_integral` を経由するため、標準の整数型と同様に
/// `#`（プレフィックス付与）と `0`（プレフィックスの後ろへのゼロ埋め）が
/// 正しく効く。`U256` の最大値はちょうど64桁なので、幅を超えることはない。
pub fn encode_amount_hex(amount: U256) -> String {
    format!("{amount:#066x}")
}
