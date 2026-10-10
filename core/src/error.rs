use std::fmt;

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

    /// 送金先として確実に安全でないアドレス（ゼロアドレス、対応トークンの
    /// コントラクト自身）。いずれも送金が失敗するか資金が永久に失われるだけで
    /// 正当な用途がないため、両方向のパースとHashPortリンク生成で拒否する。
    #[error("unsafe recipient: {reason}")]
    UnsafeRecipient { reason: UnsafeRecipientReason },
}

/// [`ParseError::UnsafeRecipient`] の理由。JS側には `"zero_address"` /
/// `"token_contract"` の文字列としてシリアライズされる。
///
/// `#[non_exhaustive]` により、今後理由を追加しても下流クレートの `match` を
/// 壊さない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum UnsafeRecipientReason {
    /// ゼロアドレス（`0x0000000000000000000000000000000000000000`）。
    ZeroAddress,
    /// 対応トークンのコントラクトアドレス自身（チェーンは問わない）。
    TokenContract,
}

impl fmt::Display for UnsafeRecipientReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnsafeRecipientReason::ZeroAddress => "zero address",
            UnsafeRecipientReason::TokenContract => "token contract",
        })
    }
}
