//! HashPort Walletが発行するJPYC決済リンクとEIP-681（ERC-681）決済URIとを
//! 相互に変換する、非公式・HashPortとは無関係のライブラリ。
//!
//! - HashPortリンク → EIP-681: [`parse`] で [`ParsedLink`] に変換し、
//!   [`ParsedLink::to_eip681`] でURIを、あるいは構成要素（アドレス、金額）を
//!   取り出す。
//! - EIP-681 → HashPortリンク: [`parse_eip681`] で [`ParsedLink`] に変換し、
//!   [`ParsedLink::to_hashport_link`] でリンクを生成する。逆方向のパースは
//!   送金先・金額・チェーンを決めるものであるため、解釈が割れうる入力を
//!   すべてエラーにする厳格な方針を取る。生成したリンクは、送金先アドレスが
//!   HashPort Wallet側で設定済みである前提でのみ意味を持つ（本ライブラリは
//!   それを確認できない）。
//!
//! 純粋なクライアントサイドの文字列/URLパースのみを行い、HashPortの
//! サーバーを含め外部へは一切ネットワークリクエストを送らない。HashPort
//! Walletを持たない人が、提示されたQR/リンクを読み取り別のウォレットで
//! 支払うため（およびその逆）の相互運用ツールとして存在する。HashPortの
//! 承認・提携を受けたものではない。
//!
//! 現時点ではJPYCのみ、チェーンはEthereum（chainId 1）/ Polygon（137）/
//! Avalanche C-Chain（43114）/ Kaia（8217）に対応する。

mod address;
mod amount;
mod constants;
mod currency;
mod eip681;
mod error;
mod link;
mod parse;

pub use address::validate_checksum;
pub use currency::Currency;
pub use eip681::parse_eip681;
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
