use alloy_primitives::Address;

use crate::constants;
use crate::error::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Currency {
    Jpyc,
}

impl Currency {
    pub const fn contract_address(self) -> Address {
        match self {
            Currency::Jpyc => constants::JPYC_POLYGON_ADDRESS,
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

    /// クエリ中の生の `master_currency_id` 値をパースする。将来的に通貨を
    /// 追加する際は、他の箇所に判定を散らばせず、この match アームを
    /// 増やすだけで済むようにする。
    pub fn from_raw_id(raw: &str) -> Result<Currency, ParseError> {
        match raw {
            constants::JPYC_MASTER_CURRENCY_ID => Ok(Currency::Jpyc),
            other => Err(ParseError::UnsupportedCurrency {
                id: other.to_string(),
            }),
        }
    }
}
