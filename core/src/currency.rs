use alloy_primitives::Address;

use crate::constants;
use crate::error::ParseError;
use crate::link::ChainId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Currency {
    Jpyc,
}

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

    /// クエリ中の生の `master_currency_id` 値をパースする。HashPort Wallet側
    /// では「通貨×チェーン」の組ごとにIDが振られているため、通貨とチェーンを
    /// 組で返す。将来的に通貨・チェーンを追加する際は、他の箇所に判定を
    /// 散らばせず、この match アームを増やすだけで済むようにする。
    pub fn from_raw_id(raw: &str) -> Result<(Currency, ChainId), ParseError> {
        match raw {
            constants::JPYC_POLYGON_MASTER_CURRENCY_ID => Ok((Currency::Jpyc, ChainId::POLYGON)),
            constants::JPYC_AVALANCHE_MASTER_CURRENCY_ID => {
                Ok((Currency::Jpyc, ChainId::AVALANCHE))
            }
            constants::JPYC_ETHEREUM_MASTER_CURRENCY_ID => Ok((Currency::Jpyc, ChainId::ETHEREUM)),
            constants::JPYC_KAIA_MASTER_CURRENCY_ID => Ok((Currency::Jpyc, ChainId::KAIA)),
            other => Err(ParseError::UnsupportedCurrency {
                id: other.to_string(),
            }),
        }
    }
}
