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

/// EIP-681入力用のアドレスパース。`0x`（小文字）+ 40桁の16進数のみを受け付け、
/// 大文字小文字の扱いは一般的なウォレットのEIP-55の慣行に従う:
///
/// - 16進の英字がすべて小文字、またはすべて大文字 → チェックサム情報を
///   持たない表記として受け付ける。
/// - 大文字と小文字が混在 → EIP-55チェックサムとして正しい場合のみ受け付ける。
///   混在しているのにチェックサムが合わないのは打ち間違いの兆候であり、
///   送金先を生成する用途では黙って受け入れてはならないため。
///
/// [`parse_address`] は大文字小文字を一切検査せず（alloyの `Address::from_str`
/// は単なる16進デコードで、混在表記のチェックサムを検証しない）、
/// [`validate_checksum`] は大文字のみ・小文字のみの表記を拒否するため、
/// どちらもこの規則には使えない。ENS名など `0x` で始まらない値もここで拒否する
/// （名前解決にはネットワークアクセスが必要で、本ライブラリは一切行わないため）。
pub fn parse_address_eip55_if_mixed(raw: &str) -> Result<Address, ParseError> {
    let invalid = || ParseError::InvalidAddress {
        value: raw.to_string(),
    };

    let body = raw.strip_prefix("0x").ok_or_else(invalid)?;
    if body.len() != 40 || !body.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid());
    }

    let has_lower = body.bytes().any(|b| b.is_ascii_lowercase());
    let has_upper = body.bytes().any(|b| b.is_ascii_uppercase());
    if has_lower && has_upper {
        // 大文字小文字が混在する場合のみ、厳密なEIP-55検証に委ねる。
        validate_checksum(raw)
    } else {
        parse_address(raw)
    }
}
