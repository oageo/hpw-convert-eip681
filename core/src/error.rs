use thiserror::Error;

// `Deserialize` は意図的に導出しない。`MissingParam` の `&'static str`
// フィールドはデシリアライザが `'static` な借用データを返せる場合にしか
// 復元できず（例: serde_jsonで `String` からのデシリアライズは実行時
// エラーになる）、実質的な罠になるため。ワークスペース内の用途も
// エラーをJS側へ投げるためのシリアライズのみである。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind"))]
pub enum ParseError {
    #[error("could not parse as a URL: {message}")]
    InvalidUrl { message: String },

    #[error("unsupported host: {host}")]
    UnsupportedHost { host: String },

    #[error("missing required parameter: {name}")]
    MissingParam { name: &'static str },

    /// `id` はクエリ中の生の `master_currency_id` 値（数値とは限らない）。
    /// 現時点ではJPYCの "487"（Polygon）/ "489"（Avalanche）/
    /// "490"（Ethereum）/ "712"（Kaia）のみ対応している。
    #[error("unsupported currency id: {id}")]
    UnsupportedCurrency { id: String },

    #[error("invalid address: {value}")]
    InvalidAddress { value: String },

    #[error("invalid amount: {value} ({reason})")]
    InvalidAmount { value: String, reason: String },
}
