use alloy_primitives::U256;
use hpw_convert_eip681::{is_supported, parse, validate_checksum, ParseError};

const CHECKSUM_ADDR: &str = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";
const JPYC_ADDR: &str = "0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29";
const ONE_TOKEN_HEX: &str = "de0b6b3a7640000";
const ONE_TOKEN_DECIMAL: &str = "1000000000000000000";

fn url(query: &str) -> String {
    format!("https://link.expo2025-wallet.com/pay?{query}")
}

#[test]
fn happy_path_with_0x_prefixed_amount_and_to_name() {
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}&to_name=Test%20Shop"
    ));
    let parsed = parse(&link).expect("should parse");

    assert_eq!(parsed.to.to_string(), CHECKSUM_ADDR);
    assert_eq!(
        parsed.amount,
        Some(U256::from_str_radix(ONE_TOKEN_HEX, 16).unwrap())
    );
    assert_eq!(parsed.to_name.as_deref(), Some("Test Shop"));
    assert_eq!(
        parsed.to_eip681(),
        format!(
            "ethereum:{JPYC_ADDR}@137/transfer?address={CHECKSUM_ADDR}&uint256={ONE_TOKEN_DECIMAL}"
        )
    );
    assert!(is_supported(&link));
}

#[test]
fn amount_param_absent_is_none_and_omitted_from_eip681() {
    // 元URLに `amount` パラメータ自体が存在しないケース。エラーにはせず、
    // `amount` を `None` として扱い、EIP-681出力からも `uint256=` を省く。
    let link = url(&format!("to={CHECKSUM_ADDR}&master_currency_id=487"));
    let parsed = parse(&link).expect("should parse");

    assert_eq!(parsed.amount, None);
    assert_eq!(
        parsed.to_eip681(),
        format!("ethereum:{JPYC_ADDR}@137/transfer?address={CHECKSUM_ADDR}")
    );
    assert!(is_supported(&link));
}

#[test]
fn amount_without_0x_prefix_matches_prefixed_form() {
    let with_prefix = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}"
    ));
    let without_prefix = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount={ONE_TOKEN_HEX}"
    ));

    let a = parse(&with_prefix).expect("should parse");
    let b = parse(&without_prefix).expect("should parse");
    assert_eq!(a.amount, b.amount);
}

#[test]
fn all_digit_amount_is_decoded_as_hex_not_decimal() {
    // 回帰テスト: プレフィックスなしで数字のみからなる値に対して素朴に
    // `U256::from_str` を使うと10進としてパースされてしまう。"1000" は
    // 10進1000ではなく16進0x1000（4096）としてデコードされなければならない。
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=1000"
    ));
    let parsed = parse(&link).expect("should parse");
    assert_eq!(parsed.amount, Some(U256::from(4096u32)));
}

#[test]
fn to_name_absent_is_none() {
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}"
    ));
    let parsed = parse(&link).expect("should parse");
    assert_eq!(parsed.to_name, None);
}

#[test]
fn to_name_present_but_empty_is_some_empty_string() {
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}&to_name="
    ));
    let parsed = parse(&link).expect("should parse");
    assert_eq!(parsed.to_name, Some(String::new()));
}

#[test]
fn unknown_query_param_is_ignored() {
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}&foo=bar"
    ));
    assert!(parse(&link).is_ok());
}

#[test]
fn type_dynamic_param_is_captured_as_link_type() {
    // HashPort Wallet側の仕様変更で、URL末尾に `type=dynamic` が付与される
    // ようになった。`to_name` と同様に、解釈を加えず生の値をそのまま
    // `link_type` に保持する（取りうる値の全体像が未確認のため、bool等に
    // 決め打ちしない）。
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}&to_name=Cafe&type=dynamic"
    ));
    let parsed = parse(&link).expect("should parse");
    assert_eq!(parsed.to.to_string(), CHECKSUM_ADDR);
    assert_eq!(
        parsed.amount,
        Some(U256::from_str_radix(ONE_TOKEN_HEX, 16).unwrap())
    );
    assert_eq!(parsed.link_type.as_deref(), Some("dynamic"));
}

#[test]
fn link_type_absent_is_none() {
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}"
    ));
    let parsed = parse(&link).expect("should parse");
    assert_eq!(parsed.link_type, None);
}

#[test]
fn uppercase_host_is_accepted_case_insensitively() {
    let link = format!(
        "https://LINK.EXPO2025-WALLET.COM/pay?to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}"
    );
    assert!(parse(&link).is_ok());
}

#[test]
fn wrong_host_is_rejected() {
    let link = format!(
        "https://evil.example.com/pay?to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}"
    );
    assert_eq!(
        parse(&link),
        Err(ParseError::UnsupportedHost {
            host: "evil.example.com".to_string()
        })
    );
    assert!(!is_supported(&link));
}

#[test]
fn suffix_spoofed_host_is_rejected_not_suffix_matched() {
    let link = format!(
        "https://link.expo2025-wallet.com.evil.com/pay?to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}"
    );
    assert!(matches!(
        parse(&link),
        Err(ParseError::UnsupportedHost { .. })
    ));
}

#[test]
fn completely_unparseable_string_is_invalid_url() {
    assert!(matches!(
        parse("not a url"),
        Err(ParseError::InvalidUrl { .. })
    ));
}

#[test]
fn missing_to_param() {
    let link = url("master_currency_id=487&amount=0x1");
    assert_eq!(parse(&link), Err(ParseError::MissingParam { name: "to" }));
}

#[test]
fn missing_master_currency_id_param() {
    let link = url(&format!("to={CHECKSUM_ADDR}&amount=0x1"));
    assert_eq!(
        parse(&link),
        Err(ParseError::MissingParam {
            name: "master_currency_id"
        })
    );
}

#[test]
fn unsupported_numeric_currency_id() {
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=1&amount=0x1"
    ));
    assert_eq!(
        parse(&link),
        Err(ParseError::UnsupportedCurrency {
            id: "1".to_string()
        })
    );
}

#[test]
fn non_numeric_currency_id() {
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=abc&amount=0x1"
    ));
    assert_eq!(
        parse(&link),
        Err(ParseError::UnsupportedCurrency {
            id: "abc".to_string()
        })
    );
}

#[test]
fn malformed_address_wrong_length() {
    // 40桁ではなく39桁の16進文字列。
    let link =
        url("to=0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAe&master_currency_id=487&amount=0x1");
    assert!(matches!(
        parse(&link),
        Err(ParseError::InvalidAddress { .. })
    ));
}

#[test]
fn address_without_0x_prefix_is_accepted() {
    // `Address::from_str` は `0x` プレフィックスなしの40桁16進文字列も
    // 受け付ける。`parse_address` は意図的にこれを許容する。
    let link = url("to=5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed&master_currency_id=487&amount=0x1");
    let parsed = parse(&link).expect("should parse");
    assert_eq!(parsed.to.to_string(), CHECKSUM_ADDR);
}

#[test]
fn malformed_address_invalid_hex_char() {
    let link =
        url("to=0xgaAeb6053F3E94C9b9A09f33669435E7Ef1BeAed&master_currency_id=487&amount=0x1");
    assert!(matches!(
        parse(&link),
        Err(ParseError::InvalidAddress { .. })
    ));
}

#[test]
fn non_hex_amount() {
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=zzzz"
    ));
    assert!(matches!(
        parse(&link),
        Err(ParseError::InvalidAmount { .. })
    ));
}

#[test]
fn empty_amount() {
    // `amount=` のようにパラメータ自体は存在するが値が空の場合は、
    // パラメータ省略（`None`）とは区別してエラーとする。
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount="
    ));
    assert!(matches!(
        parse(&link),
        Err(ParseError::InvalidAmount { .. })
    ));
}

#[test]
fn bare_0x_prefix_amount_is_rejected() {
    // `amount=0x` のようにプレフィックスのみで16進数字が続かない場合も
    // エラーとする。ruintの `from_str_radix` は空文字列を `Ok(0)` として
    // 受理してしまうため、金額0への静かな化けを防ぐ回帰テスト。
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x"
    ));
    assert!(matches!(
        parse(&link),
        Err(ParseError::InvalidAmount { .. })
    ));
}

#[test]
fn amount_larger_than_u128_is_decoded_correctly() {
    // 34桁の16進数（128ビット超）。u128::MAXを大きく超えるが切り捨てられ
    // てはならない。
    let hex = "ffffffffffffffffffffffffffffffff00";
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount={hex}"
    ));
    let parsed = parse(&link).expect("should parse");

    let expected = U256::from_str_radix(hex, 16).unwrap();
    assert_eq!(parsed.amount, Some(expected));
    assert!(parsed.amount.unwrap() > U256::from(u128::MAX));
}

#[test]
fn amount_larger_than_u256_is_rejected_not_panicking() {
    // 65桁の16進数 => 260ビットで、U256の256ビット容量を超える。
    let hex = "f".repeat(65);
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount={hex}"
    ));
    assert!(matches!(
        parse(&link),
        Err(ParseError::InvalidAmount { .. })
    ));
}

#[test]
fn address_amount_pair_matches_parsed_link_fields() {
    let link = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}"
    ));
    let parsed = parse(&link).expect("should parse");
    let pair = parsed.address_amount();
    assert_eq!(pair.address, parsed.to);
    assert_eq!(pair.amount, parsed.amount);
    assert_eq!(parsed.address(), parsed.to);
    assert_eq!(parsed.amount(), parsed.amount);
}

#[test]
fn validate_checksum_accepts_correctly_checksummed_address() {
    assert!(validate_checksum(CHECKSUM_ADDR).is_ok());
}

#[test]
fn validate_checksum_rejects_single_flipped_case_char() {
    // アドレス中の最初の16進文字の大文字小文字を反転させる。
    let tampered = CHECKSUM_ADDR.replacen('a', "A", 1);
    assert!(validate_checksum(&tampered).is_err());
}

#[test]
fn validate_checksum_rejects_all_lowercase() {
    // alloy-primitivesの `parse_checksummed` は入力がチェックサム表記と
    // 一字一句一致することを要求する。大文字小文字を統一した文字列は
    // チェックサム情報を持たないため、曖昧でないとして許容するのではなく
    // 拒否される。
    let lower = CHECKSUM_ADDR.to_lowercase();
    assert!(validate_checksum(&lower).is_err());
}

#[test]
fn is_supported_matches_parse_ok() {
    let ok = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=487&amount=0x{ONE_TOKEN_HEX}"
    ));
    let bad_host = "https://evil.example.com/pay";
    let bad_currency = url(&format!(
        "to={CHECKSUM_ADDR}&master_currency_id=1&amount=0x1"
    ));

    assert!(is_supported(&ok));
    assert!(!is_supported(bad_host));
    assert!(!is_supported(&bad_currency));
}
