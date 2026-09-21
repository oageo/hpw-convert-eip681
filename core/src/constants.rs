use alloy_primitives::{address, Address};

/// HashPort Wallet決済リンクとして許可するホスト名。サブドメインなりすまし
/// を防ぐため、サフィックス一致ではなく完全一致で検証する。
pub const EXPECTED_HOST: &str = "link.expo2025-wallet.com";

// `master_currency_id` の生値。HashPort Wallet側では通貨単体ではなく
// 「通貨×チェーン」の組ごとにIDが振られているため、同じJPYCでもチェーン
// ごとに値が異なる。いずれもHashPort Walletが実際に生成した決済リンクから
// 確認した値。
pub const JPYC_POLYGON_MASTER_CURRENCY_ID: &str = "487";
pub const JPYC_AVALANCHE_MASTER_CURRENCY_ID: &str = "489";
pub const JPYC_ETHEREUM_MASTER_CURRENCY_ID: &str = "490";
pub const JPYC_KAIA_MASTER_CURRENCY_ID: &str = "712";

/// JPYCの公式コントラクトアドレス。Ethereum / Polygon / Avalanche C-Chain /
/// Kaia の全対応チェーンに同一アドレスでデプロイされている。JPYC公式GitHub
/// 組織ページ（全チェーン共通である旨とchain idの一覧）、PolygonScan上の
/// 検証済みコントラクト、およびKaia対応時のJPYC株式会社公式プレスリリースで
/// 確認済み。前払式JPYC（旧JPYC）のアドレスとは異なる点に注意。`address!`
/// マクロはコンパイル時にEIP-55チェックサムを検証する。
pub const JPYC_ADDRESS: Address = address!("0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29");

pub const JPYC_DECIMALS: u8 = 18;

pub const ETHEREUM_CHAIN_ID: u64 = 1;
pub const POLYGON_CHAIN_ID: u64 = 137;
pub const KAIA_CHAIN_ID: u64 = 8217;
pub const AVALANCHE_CHAIN_ID: u64 = 43114;
