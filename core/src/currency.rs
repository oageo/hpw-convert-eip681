use alloy_primitives::Address;

use crate::constants;
use crate::error::ParseError;
use crate::link::ChainId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Currency {
    Jpyc,
}

/// `master_currency_id` と（通貨, チェーン）の対応表。HashPort Wallet側では
/// 「通貨×チェーン」の組ごとにIDが振られている。HashPortリンク→EIP-681、
/// EIP-681→HashPortリンクの両方向の変換がこの1つの表だけを参照するため、
/// 片方向だけ更新されて対応がずれることはない。通貨・チェーンを追加する際は
/// この表に行を足すだけで済むようにする。
const SUPPORTED: &[(&str, Currency, ChainId)] = &[
    (
        constants::JPYC_POLYGON_MASTER_CURRENCY_ID,
        Currency::Jpyc,
        ChainId::POLYGON,
    ),
    (
        constants::JPYC_AVALANCHE_MASTER_CURRENCY_ID,
        Currency::Jpyc,
        ChainId::AVALANCHE,
    ),
    (
        constants::JPYC_ETHEREUM_MASTER_CURRENCY_ID,
        Currency::Jpyc,
        ChainId::ETHEREUM,
    ),
    (
        constants::JPYC_KAIA_MASTER_CURRENCY_ID,
        Currency::Jpyc,
        ChainId::KAIA,
    ),
];

/// 対応しているすべての通貨。コントラクトアドレスからの逆引きに使う。
const ALL_CURRENCIES: &[Currency] = &[Currency::Jpyc];

impl Currency {
    /// JPYCは全対応チェーンに同一アドレスでデプロイされているため、チェーンを
    /// 引数に取らない。チェーンごとにアドレスが異なる通貨を追加する場合は
    /// `ChainId` を受け取る形に変更すること。
    pub const fn contract_address(self) -> Address {
        match self {
            Currency::Jpyc => constants::JPYC_ADDRESS,
        }
    }

    pub const fn decimals(self) -> u8 {
        match self {
            Currency::Jpyc => constants::JPYC_DECIMALS,
        }
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            Currency::Jpyc => "JPYC",
        }
    }

    /// クエリ中の生の `master_currency_id` 値をパースし、（通貨, チェーン）の
    /// 組を返す（HashPortリンク→EIP-681方向）。
    pub fn from_raw_id(raw: &str) -> Result<(Currency, ChainId), ParseError> {
        SUPPORTED
            .iter()
            .find(|(id, _, _)| *id == raw)
            .map(|&(_, currency, chain)| (currency, chain))
            .ok_or_else(|| ParseError::UnsupportedCurrency {
                id: raw.to_string(),
            })
    }

    /// （通貨, チェーン）の組に対応する `master_currency_id` を返す
    /// （EIP-681→HashPortリンク方向）。対応表にない組は `None`。
    pub fn master_currency_id(self, chain: ChainId) -> Option<&'static str> {
        SUPPORTED
            .iter()
            .find(|&&(_, currency, c)| currency == self && c == chain)
            .map(|&(id, _, _)| id)
    }

    /// トークンのコントラクトアドレスから通貨を逆引きする（チェーンは問わない）。
    /// 対応チェーンかどうかの判定は [`Currency::master_currency_id`] で別途行う。
    /// アドレスは `Address` 同士の比較なので、元の文字列の大文字小文字には
    /// 依存しない。
    pub fn from_contract_address(address: Address) -> Option<Currency> {
        ALL_CURRENCIES
            .iter()
            .copied()
            .find(|c| c.contract_address() == address)
    }
}
