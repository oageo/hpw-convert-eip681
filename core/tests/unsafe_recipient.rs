//! 送金先として確実に安全でないアドレス（ゼロアドレス、対応トークンの
//! コントラクト自身）の拒否に関するテスト。順方向（`parse`）・逆方向
//! （`parse_eip681`）・HashPortリンク生成（`to_hashport_link`）の3箇所すべてで
//! 拒否されることを確認する。

use hpw_convert_eip681::{
    is_supported, parse, parse_eip681, Address, ChainId, Currency, ParseError, ParsedLink,
    UnsafeRecipientReason, U256,
};

const ZERO: &str = "0x0000000000000000000000000000000000000000";
const JPYC_CHECKSUMMED: &str = "0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29";
const JPYC_LOWER: &str = "0xe7c3d8c9a439fede00d2600032d5db0be71c3c29";
/// `0x` は小文字のまま、16進部分のみ大文字にした表記。
const JPYC_UPPER: &str = "0xE7C3D8C9A439FEDE00D2600032D5DB0BE71C3C29";

/// HashPort Walletが実際に生成した決済リンクの値。
const REAL_TO: &str = "0x9aD4Ba3D9FB338Cd9C836cD5f222BB5fF8ab2456";
const REAL_AMOUNT: &str = "0x0000000000000000000000000000000000000000000000056bc75e2d63100000";

/// （`master_currency_id`, チェーンID）の全対応組。
const CHAINS: &[(&str, u64)] = &[("487", 137), ("489", 43114), ("490", 1), ("712", 8217)];

const ZERO_ERR: ParseError = ParseError::UnsafeRecipient {
    reason: UnsafeRecipientReason::ZeroAddress,
};
const TOKEN_ERR: ParseError = ParseError::UnsafeRecipient {
    reason: UnsafeRecipientReason::TokenContract,
};

fn hashport_link(to: &str, master_currency_id: &str) -> String {
    format!(
        "https://link.expo2025-wallet.com/pay?to={to}&master_currency_id={master_currency_id}&amount={REAL_AMOUNT}&to_name=oa&type=dynamic"
    )
}

fn eip681_uri(chain_id: u64, recipient: &str) -> String {
    format!("ethereum:{JPYC_CHECKSUMMED}@{chain_id}/transfer?address={recipient}&uint256=1")
}

fn parsed_link(to: &str, chain_id: ChainId) -> ParsedLink {
    ParsedLink {
        to: to.parse().unwrap(),
        currency: Currency::Jpyc,
        chain_id,
        amount: Some(U256::from(1u64)),
        to_name: None,
        link_type: None,
    }
}

// ---- 順方向（HashPortリンク → ParsedLink） ----

#[test]
fn forward_rejects_zero_address_on_all_chains() {
    for &(id, _) in CHAINS {
        let link = hashport_link(ZERO, id);
        assert_eq!(parse(&link), Err(ZERO_ERR), "master_currency_id={id}");
        assert!(!is_supported(&link), "master_currency_id={id}");
    }
}

#[test]
fn forward_rejects_token_contract_in_any_casing_on_all_chains() {
    for &(id, _) in CHAINS {
        for to in [JPYC_CHECKSUMMED, JPYC_LOWER, JPYC_UPPER] {
            let link = hashport_link(to, id);
            assert_eq!(
                parse(&link),
                Err(TOKEN_ERR),
                "master_currency_id={id}, to={to}"
            );
            assert!(!is_supported(&link), "master_currency_id={id}, to={to}");
        }
    }
}

#[test]
fn forward_rejects_token_contract_without_0x_prefix() {
    // 順方向の寛容なアドレスパースは `0x` なしも受け付けるため、その表記でも
    // 拒否されることを確認する。
    let link = hashport_link(&JPYC_LOWER[2..], "487");
    assert_eq!(parse(&link), Err(TOKEN_ERR));
}

#[test]
fn forward_checks_recipient_before_amount() {
    // 送金先が不正かつ金額も不正な場合、送金先のエラーが先に報告される。
    let link =
        format!("https://link.expo2025-wallet.com/pay?to={ZERO}&master_currency_id=487&amount=zz");
    assert_eq!(parse(&link), Err(ZERO_ERR));
}

// ---- 逆方向（EIP-681 URI → ParsedLink） ----

#[test]
fn reverse_rejects_zero_address_on_all_chains() {
    for &(_, chain_id) in CHAINS {
        let uri = eip681_uri(chain_id, ZERO);
        assert_eq!(parse_eip681(&uri), Err(ZERO_ERR), "chain {chain_id}");
    }
}

#[test]
fn reverse_rejects_token_contract_on_all_chains() {
    // 大文字小文字の表記違い、およびパーセントエンコードされた表記
    // （`%30` = `0`, `%78` = `x`, `%45` = `E`, `%65` = `e`）。
    let encoded_checksummed = "%30%78%45%37C3D8C9a439feDe00D2600032D5dB0Be71C3c29";
    let encoded_lower = "0xe7c3d8c9a439fede00d2600032d5db0be71c3c2%39";
    for &(_, chain_id) in CHAINS {
        for recipient in [
            JPYC_CHECKSUMMED,
            JPYC_LOWER,
            JPYC_UPPER,
            encoded_checksummed,
            encoded_lower,
        ] {
            let uri = eip681_uri(chain_id, recipient);
            assert_eq!(
                parse_eip681(&uri),
                Err(TOKEN_ERR),
                "chain {chain_id}, address={recipient}"
            );
        }
    }
}

#[test]
fn reverse_rejects_with_pay_prefix_and_without_amount() {
    let uri = format!("ethereum:pay-{JPYC_CHECKSUMMED}@137/transfer?address={JPYC_LOWER}");
    assert_eq!(parse_eip681(&uri), Err(TOKEN_ERR));
    let uri = format!("ethereum:{JPYC_CHECKSUMMED}@1/transfer?address={ZERO}");
    assert_eq!(parse_eip681(&uri), Err(ZERO_ERR));
}

#[test]
fn reverse_checks_recipient_before_amount() {
    // 手順8（受取人）は手順9（金額）より先に検査される。
    for amount in ["-1", "0x10", "abc", "1.5"] {
        let uri =
            format!("ethereum:{JPYC_CHECKSUMMED}@137/transfer?address={ZERO}&uint256={amount}");
        assert_eq!(parse_eip681(&uri), Err(ZERO_ERR), "uint256={amount}");
        let uri = format!(
            "ethereum:{JPYC_CHECKSUMMED}@137/transfer?address={JPYC_LOWER}&uint256={amount}"
        );
        assert_eq!(parse_eip681(&uri), Err(TOKEN_ERR), "uint256={amount}");
    }
}

#[test]
fn reverse_reports_invalid_address_before_unsafe_recipient() {
    // チェックサムの合わない混在表記は、安全性の判定以前に `InvalidAddress`。
    let bad_checksum = "0xe7C3D8C9a439feDe00D2600032D5dB0Be71C3c29";
    let uri = eip681_uri(137, bad_checksum);
    assert_eq!(
        parse_eip681(&uri),
        Err(ParseError::InvalidAddress {
            value: bad_checksum.to_string()
        })
    );
}

// ---- HashPortリンク生成（ParsedLink → HashPortリンク） ----

#[test]
fn to_hashport_link_rejects_unsafe_recipients_on_all_chains() {
    for chain_id in [
        ChainId::POLYGON,
        ChainId::AVALANCHE,
        ChainId::ETHEREUM,
        ChainId::KAIA,
    ] {
        assert_eq!(
            parsed_link(ZERO, chain_id).to_hashport_link(),
            Err(ZERO_ERR),
            "{chain_id:?}"
        );
        assert_eq!(
            parsed_link(JPYC_CHECKSUMMED, chain_id).to_hashport_link(),
            Err(TOKEN_ERR),
            "{chain_id:?}"
        );
    }
    // `Address::ZERO` から直接構築した場合も同様。
    let mut link = parsed_link(REAL_TO, ChainId::POLYGON);
    link.to = Address::ZERO;
    assert_eq!(link.to_hashport_link(), Err(ZERO_ERR));
    link.to = Currency::Jpyc.contract_address();
    assert_eq!(link.to_hashport_link(), Err(TOKEN_ERR));
}

#[test]
fn to_hashport_link_checks_chain_before_recipient() {
    // 対応表にないチェーンの場合は、送金先の検査より先に `UnsupportedChain`。
    let link = parsed_link(ZERO, ChainId(56));
    assert_eq!(
        link.to_hashport_link(),
        Err(ParseError::UnsupportedChain {
            chain_id: "56".to_string()
        })
    );
}

#[test]
fn near_miss_addresses_are_still_accepted() {
    // ゼロアドレス・JPYCコントラクトと1文字だけ違うアドレスは通常どおり受け付ける。
    let near_zero = "0x0000000000000000000000000000000000000001";
    let near_jpyc = "0xe7c3d8c9a439fede00d2600032d5db0be71c3c28";
    for near in [near_zero, near_jpyc] {
        let link = parsed_link(near, ChainId::POLYGON);
        let hashport = link
            .to_hashport_link()
            .unwrap_or_else(|e| panic!("{near}: {e}"));
        assert_eq!(parse(&hashport).unwrap().to, link.to, "{near}");
        let uri = link.to_eip681();
        assert_eq!(parse_eip681(&uri).unwrap().to, link.to, "{near}");
    }
}

#[test]
fn to_eip681_stays_infallible_for_unsafe_recipients() {
    // `to_eip681()` は設計上infallibleのまま（検査はパース側と
    // `to_hashport_link()` で行う）。
    let uri = parsed_link(ZERO, ChainId::POLYGON).to_eip681();
    assert_eq!(
        uri,
        format!("ethereum:{JPYC_CHECKSUMMED}@137/transfer?address={ZERO}&uint256=1")
    );
}

// ---- 実リンクの回帰確認 ----

#[test]
fn real_links_still_parse_and_round_trip() {
    for &(id, chain_id) in CHAINS {
        let original = hashport_link(REAL_TO, id);
        assert!(is_supported(&original), "master_currency_id={id}");
        let parsed = parse(&original).unwrap();
        assert_eq!(parsed.chain_id, ChainId(chain_id));
        assert_eq!(parsed.to_hashport_link().unwrap(), original);

        let mut reversed = parse_eip681(&parsed.to_eip681()).unwrap();
        reversed.to_name = Some("oa".to_string());
        reversed.link_type = Some("dynamic".to_string());
        assert_eq!(reversed.to_hashport_link().unwrap(), original);
    }
}

// ---- 表示 ----

#[test]
fn display_strings() {
    assert_eq!(
        UnsafeRecipientReason::ZeroAddress.to_string(),
        "zero address"
    );
    assert_eq!(
        UnsafeRecipientReason::TokenContract.to_string(),
        "token contract"
    );
    assert_eq!(ZERO_ERR.to_string(), "unsafe recipient: zero address");
    assert_eq!(TOKEN_ERR.to_string(), "unsafe recipient: token contract");
}
