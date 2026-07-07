use url::Url;

use crate::address;
use crate::amount;
use crate::constants;
use crate::currency::Currency;
use crate::error::ParseError;
use crate::link::{ChainId, ParsedLink};

/// HashPort Wallet決済リンクをパースし、構成要素に分解する。
///
/// パイプラインの順序: URLパース → ホスト検証 → クエリ抽出 → 通貨判定 →
/// アドレス検証 → 金額デコード → 構築。各ステップでエラーがあれば
/// その時点で返す。
pub fn parse(input: &str) -> Result<ParsedLink, ParseError> {
    let url = Url::parse(input).map_err(|e| ParseError::InvalidUrl {
        message: e.to_string(),
    })?;

    // `url::Url` はWHATWG URL仕様に従いホストを小文字化するため、この
    // 比較は特別なコードなしに大文字小文字を区別しない。
    // "link.expo2025-wallet.com.evil.example" のようなサブドメイン
    // なりすましを拒否するため、サフィックス一致ではなく完全一致とする。
    let host = url.host_str().ok_or_else(|| ParseError::UnsupportedHost {
        host: String::new(),
    })?;
    if host != constants::EXPECTED_HOST {
        return Err(ParseError::UnsupportedHost {
            host: host.to_string(),
        });
    }

    let mut to_raw = None;
    let mut currency_raw = None;
    let mut amount_raw = None;
    let mut to_name = None;
    // 同名のクエリパラメータが重複する場合（例: `to=X&to=Y`）は後勝ちと
    // なる。URL全体がユーザーに見えるものであり、エラーにするほどの
    // 攻撃面ではないため、意図的にこの挙動のままとしている。
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "to" => to_raw = Some(v.into_owned()),
            "master_currency_id" => currency_raw = Some(v.into_owned()),
            "amount" => amount_raw = Some(v.into_owned()),
            "to_name" => to_name = Some(v.into_owned()),
            _ => {}
        }
    }

    let to_raw = to_raw.ok_or(ParseError::MissingParam { name: "to" })?;
    let currency_raw = currency_raw.ok_or(ParseError::MissingParam {
        name: "master_currency_id",
    })?;

    let currency = Currency::from_raw_id(&currency_raw)?;
    let to = address::parse_address(&to_raw)?;
    // `amount` は元URLに存在しないことがあるため必須パラメータとしない。
    // 存在する場合のみ16進デコードする。
    let amount = amount_raw
        .map(|raw| amount::parse_amount_hex(&raw))
        .transpose()?;

    Ok(ParsedLink {
        to,
        currency,
        chain_id: ChainId::POLYGON,
        amount,
        to_name,
    })
}
