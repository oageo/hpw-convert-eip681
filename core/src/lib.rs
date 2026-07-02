//! HashPort Walletが発行するJPYC決済リンクをパースし、EIP-681（ERC-681）
//! 決済URI、あるいはその構成要素（アドレス、金額）に変換する、非公式・
//! HashPortとは無関係のライブラリ。
//!
//! 純粋なクライアントサイドの文字列/URLパースのみを行い、HashPortの
//! サーバーへは一切ネットワークリクエストを送らない。HashPort Walletを
//! 持たない人が、提示されたQR/リンクを読み取り別のウォレットで支払うための
//! 相互運用ツールとして存在する。HashPortの承認・提携を受けたものではない。
//!
//! 現時点ではPolygon（chainId 137）とJPYCのみに対応する。

mod address;
mod amount;
mod constants;
mod currency;
mod error;
mod link;
mod parse;

pub use address::validate_checksum;
pub use currency::Currency;
pub use error::ParseError;
pub use link::{AddressAmount, ChainId, ParsedLink};
pub use parse::parse;

// downstreamのクレートが `ParsedLink` に含まれる型を扱うためだけに
// alloy-primitivesへの直接依存を必要としないよう再エクスポートする。
pub use alloy_primitives::{Address, U256};

/// `url` がこのクレートでパース可能なHashPort Wallet決済リンクかどうかを
/// 返す。[`parse`] の薄いラッパー。非対応の理由を知りたい場合は
/// `parse` を直接呼び出すこと。
pub fn is_supported(url: &str) -> bool {
    parse(url).is_ok()
}
