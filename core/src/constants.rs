use alloy_primitives::{address, Address};

/// HashPort Wallet決済リンクとして許可するホスト名。サブドメインなりすまし
/// を防ぐため、サフィックス一致ではなく完全一致で検証する。
pub const EXPECTED_HOST: &str = "link.expo2025-wallet.com";

/// JPYCを表す `master_currency_id` の生値。現時点ではこの値のみ対応する。
pub const JPYC_MASTER_CURRENCY_ID: &str = "487";

/// JPYCのPolygon上の公式コントラクトアドレス。Ethereum / Avalanche C-Chain
/// にも同一アドレスでCREATE2デプロイされている。JPYC公式GitHub組織ページと
/// PolygonScan上の検証済みコントラクトで確認済み。`address!`マクロは
/// コンパイル時にEIP-55チェックサムを検証する。
pub const JPYC_POLYGON_ADDRESS: Address = address!("0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29");

pub const JPYC_DECIMALS: u8 = 18;

pub const POLYGON_CHAIN_ID: u64 = 137;
