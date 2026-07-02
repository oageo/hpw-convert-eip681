use std::str::FromStr;

use alloy_primitives::Address;

use crate::error::ParseError;

/// メインのパイプラインで使う寛容なパース。大文字小文字を問わず、
/// `0x`/`0X` プレフィックスの有無も問わない。EIP-55チェックサムは
/// 強制しない。ソースURLが必ずチェックサム表記であるとは限らないため、
/// それだけを理由に拒否すべきではないため。
pub fn parse_address(raw: &str) -> Result<Address, ParseError> {
    Address::from_str(raw).map_err(|_| ParseError::InvalidAddress {
        value: raw.to_string(),
    })
}

/// オプションの厳密なEIP-55チェックサム検証。`raw` はアドレスの
/// チェックサム表記と一字一句完全に一致する必要がある。大文字のみ・
/// 小文字のみの表記は、同じアドレスを指していてもチェックサム情報を
/// 持たないため拒否される。
pub fn validate_checksum(raw: &str) -> Result<Address, ParseError> {
    Address::parse_checksummed(raw, None).map_err(|_| ParseError::InvalidAddress {
        value: raw.to_string(),
    })
}
