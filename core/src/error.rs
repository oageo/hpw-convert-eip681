use thiserror::Error;

// `Deserialize` は意図的に導出しない。`MissingParam` の `&'static str`
// フィールドはデシリアライザが `'static` な借用データを返せる場合にしか
// 復元できず（例: serde_jsonで `String` からのデシリアライズは実行時
// エラーになる）、実質的な罠になるため。ワークスペース内の用途も
// エラーをJS側へ投げるためのシリアライズのみである。
//
// `#[non_exhaustive]` により、今後バリアントを追加しても下流クレートの
// `match` を壊さない（下流ではワイルドカードアームが必須になる）。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind"))]
#[non_exhaustive]
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

    /// EIP-681 URIとして構文的に不正（スキームが `ethereum:` でない、
    /// パスが壊れている等）。`reason` は人間向けの説明。
    #[error("invalid EIP-681 URI: {reason}")]
    InvalidEip681 { reason: String },

    /// EIP-681の対象トークンコントラクトが非対応。`address` は入力中の
    /// 生の文字列。
    #[error("unsupported token contract: {address}")]
    UnsupportedContract { address: String },

    /// チェーンIDが非対応、または（通貨, チェーン）の組が対応表にない。
    /// `chain_id` は10進の文字列（JS側で `number` の精度を超えうるため
    /// 数値型にしない）。
    #[error("unsupported chain id: {chain_id}")]
    UnsupportedChain { chain_id: String },

    /// EIP-681の関数名が `transfer` 以外、もしくは関数呼び出しがない
    /// （ネイティブ通貨送金）。関数呼び出しがない場合 `name` は空文字。
    #[error("unsupported EIP-681 function: {name}")]
    UnsupportedFunction { name: String },

    /// 意味を黙って捨てることになるため受け付けないクエリパラメータ。
    #[error("unsupported parameter: {name}")]
    UnsupportedParam { name: String },

    /// 同名パラメータの重複。解釈がウォレットごとに割れうる（先勝ち/後勝ち）
    /// ため、EIP-681入力では曖昧さを残さずエラーとする。
    #[error("duplicate parameter: {name}")]
    DuplicateParam { name: String },
}
