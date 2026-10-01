//! `parse_eip681`（EIP-681 URI → `ParsedLink`）の結合テスト。

use std::time::{Duration, Instant};

use hpw_convert_eip681::{parse, parse_eip681, Address, ChainId, Currency, ParseError, U256};

/// 全対応チェーン共通のJPYCコントラクト（EIP-55表記）。
const JPYC: &str = "0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29";
/// 受取人アドレス（EIP-55表記）。
const TO: &str = "0x9aD4Ba3D9FB338Cd9C836cD5f222BB5fF8ab2456";
/// 実際のHashPortリンクの金額（100 JPYC = 100 × 10^18）。
const AMOUNT_HEX: &str = "0x0000000000000000000000000000000000000000000000056bc75e2d63100000";

/// （`master_currency_id`, チェーンID）の実在する組。
const CHAINS: &[(&str, u64)] = &[("487", 137), ("489", 43114), ("490", 1), ("712", 8217)];

fn hashport_link(id: &str, with_amount: bool) -> String {
    let amount = if with_amount {
        format!("&amount={AMOUNT_HEX}")
    } else {
        String::new()
    };
    format!(
        "https://link.expo2025-wallet.com/pay?to={TO}&master_currency_id={id}{amount}&to_name=oa&type=dynamic"
    )
}

/// Polygon上のJPYC送金URI。`rest` は `/transfer` 以降（`?` を含む）。
fn uri(rest: &str) -> String {
    format!("ethereum:{JPYC}@137{rest}")
}

fn uri_with_amount(amount: &str) -> String {
    uri(&format!("/transfer?address={TO}&uint256={amount}"))
}

fn amount_of(amount: &str) -> Result<Option<U256>, ParseError> {
    parse_eip681(&uri_with_amount(amount)).map(|l| l.amount)
}

fn u256(s: &str) -> U256 {
    U256::from_str_radix(s, 10).unwrap()
}

fn to_addr() -> Address {
    TO.parse().unwrap()
}

fn assert_invalid_eip681(input: &str) {
    match parse_eip681(input) {
        Err(ParseError::InvalidEip681 { .. }) => {}
        other => panic!("expected InvalidEip681 for {input:?}, got {other:?}"),
    }
}

fn assert_invalid_amount(amount: &str) {
    match amount_of(amount) {
        Err(ParseError::InvalidAmount { value, .. }) => assert_eq!(value, amount),
        other => panic!("expected InvalidAmount for {amount:?}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 往復変換
// ---------------------------------------------------------------------------

#[test]
fn round_trip_all_chains_with_amount() {
    for &(id, chain) in CHAINS {
        let forward = parse(&hashport_link(id, true)).unwrap();
        let back = parse_eip681(&forward.to_eip681()).unwrap();
        assert_eq!(back.chain_id, ChainId(chain));
        assert_eq!(back.amount, Some(u256("100000000000000000000")));
        let mut expected = forward.clone();
        expected.to_name = None;
        expected.link_type = None;
        assert_eq!(back, expected, "chain {chain}");
    }
}

#[test]
fn round_trip_all_chains_without_amount() {
    for &(id, chain) in CHAINS {
        let forward = parse(&hashport_link(id, false)).unwrap();
        assert_eq!(forward.amount, None);
        let back = parse_eip681(&forward.to_eip681()).unwrap();
        assert_eq!(back.amount, None);
        let mut expected = forward.clone();
        expected.to_name = None;
        expected.link_type = None;
        assert_eq!(back, expected, "chain {chain}");
    }
}

#[test]
fn basic_fields() {
    let link = parse_eip681(&uri_with_amount("1e18")).unwrap();
    assert_eq!(link.to, to_addr());
    assert_eq!(link.currency, Currency::Jpyc);
    assert_eq!(link.chain_id, ChainId::POLYGON);
    assert_eq!(link.amount, Some(u256("1000000000000000000")));
    assert_eq!(link.to_name, None);
    assert_eq!(link.link_type, None);
}

// ---------------------------------------------------------------------------
// スキーム・文字レベルの検査
// ---------------------------------------------------------------------------

#[test]
fn scheme_case_insensitive() {
    let upper = uri_with_amount("1").replacen("ethereum:", "ETHEREUM:", 1);
    assert!(parse_eip681(&upper).is_ok());
    let mixed = uri_with_amount("1").replacen("ethereum:", "Ethereum:", 1);
    assert!(parse_eip681(&mixed).is_ok());
}

#[test]
fn pay_prefix_accepted() {
    let input = uri_with_amount("1").replacen("ethereum:", "ethereum:pay-", 1);
    assert_eq!(parse_eip681(&input).unwrap().amount, Some(U256::from(1)));
}

#[test]
fn wrong_scheme_rejected() {
    assert_invalid_eip681(&uri_with_amount("1").replacen("ethereum:", "bitcoin:", 1));
    assert_invalid_eip681(&uri_with_amount("1").replacen("ethereum:", "ethereum//", 1));
    assert_invalid_eip681(&uri_with_amount("1").replacen("ethereum:", "", 1));
    assert_invalid_eip681("");
    assert_invalid_eip681("ethereum");
}

#[test]
fn whitespace_rejected() {
    let base = uri_with_amount("1");
    assert_invalid_eip681(&format!(" {base}"));
    assert_invalid_eip681(&format!("{base} "));
    assert_invalid_eip681(&format!("{base}\n"));
    assert_invalid_eip681(&base.replacen("&uint256", "\t&uint256", 1));
    assert_invalid_eip681(&base.replacen("@137", "@ 137", 1));
}

#[test]
fn control_and_non_ascii_rejected() {
    let base = uri_with_amount("1");
    assert_invalid_eip681(&format!("{base}\u{0}"));
    assert_invalid_eip681(&format!("{base}\u{7f}"));
    assert_invalid_eip681(&base.replacen("transfer", "transfér", 1));
}

#[test]
fn fragment_rejected() {
    assert_invalid_eip681(&format!("{}#frag", uri_with_amount("1")));
    assert_invalid_eip681(&format!("{}#", uri_with_amount("1")));
}

// ---------------------------------------------------------------------------
// トークンアドレス
// ---------------------------------------------------------------------------

#[test]
fn target_lowercase_and_uppercase_accepted() {
    let lower = format!(
        "ethereum:{}@137/transfer?address={TO}",
        JPYC.to_ascii_lowercase()
    );
    assert!(parse_eip681(&lower).is_ok());
    let upper = format!(
        "ethereum:0x{}@137/transfer?address={TO}",
        JPYC[2..].to_ascii_uppercase()
    );
    assert!(parse_eip681(&upper).is_ok());
}

#[test]
fn target_bad_checksum_rejected() {
    // 先頭の `E` を `e` にしただけの混在表記（チェックサム不一致）。
    let bad = JPYC.replacen("0xE7", "0xe7", 1);
    let input = format!("ethereum:{bad}@137/transfer?address={TO}");
    assert_eq!(
        parse_eip681(&input),
        Err(ParseError::InvalidAddress { value: bad })
    );
}

#[test]
fn target_without_0x_rejected() {
    let input = format!("ethereum:{}@137/transfer?address={TO}", &JPYC[2..]);
    assert_eq!(
        parse_eip681(&input),
        Err(ParseError::InvalidAddress {
            value: JPYC[2..].to_string()
        })
    );
    let upper_x = format!("ethereum:0X{}@137/transfer?address={TO}", &JPYC[2..]);
    assert!(matches!(
        parse_eip681(&upper_x),
        Err(ParseError::InvalidAddress { .. })
    ));
}

#[test]
fn target_wrong_length_rejected() {
    let input = format!("ethereum:{}@137/transfer?address={TO}", &JPYC[..41]);
    assert!(matches!(
        parse_eip681(&input),
        Err(ParseError::InvalidAddress { .. })
    ));
}

#[test]
fn ens_name_rejected() {
    let input = format!("ethereum:jpyc.eth@137/transfer?address={TO}");
    assert_eq!(
        parse_eip681(&input),
        Err(ParseError::InvalidAddress {
            value: "jpyc.eth".to_string()
        })
    );
    let input = format!("ethereum:pay-jpyc.eth@137/transfer?address={TO}");
    assert!(matches!(
        parse_eip681(&input),
        Err(ParseError::InvalidAddress { .. })
    ));
}

#[test]
fn unsupported_contract_rejected() {
    let usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    let input = format!("ethereum:{usdc}@1/transfer?address={TO}&uint256=1");
    assert_eq!(
        parse_eip681(&input),
        Err(ParseError::UnsupportedContract {
            address: usdc.to_string()
        })
    );
}

// ---------------------------------------------------------------------------
// チェーンID
// ---------------------------------------------------------------------------

#[test]
fn all_supported_chain_ids_accepted() {
    for &(_, chain) in CHAINS {
        let input = format!("ethereum:{JPYC}@{chain}/transfer?address={TO}");
        assert_eq!(parse_eip681(&input).unwrap().chain_id, ChainId(chain));
    }
}

#[test]
fn missing_chain_id_rejected() {
    let input = format!("ethereum:{JPYC}/transfer?address={TO}&uint256=1");
    assert_eq!(
        parse_eip681(&input),
        Err(ParseError::MissingParam { name: "chain_id" })
    );
}

#[test]
fn unsupported_chain_rejected() {
    let input = format!("ethereum:{JPYC}@56/transfer?address={TO}");
    assert_eq!(
        parse_eip681(&input),
        Err(ParseError::UnsupportedChain {
            chain_id: "56".to_string()
        })
    );
    let input = format!("ethereum:{JPYC}@0/transfer?address={TO}");
    assert_eq!(
        parse_eip681(&input),
        Err(ParseError::UnsupportedChain {
            chain_id: "0".to_string()
        })
    );
}

#[test]
fn overflowing_chain_id_rejected() {
    let input = format!("ethereum:{JPYC}@99999999999999999999999/transfer?address={TO}");
    assert_eq!(
        parse_eip681(&input),
        Err(ParseError::UnsupportedChain {
            chain_id: "99999999999999999999999".to_string()
        })
    );
}

#[test]
fn malformed_chain_id_rejected() {
    for chain in [
        "0137", "00", "", "+137", "-137", "0x89", "137a", "1.0", "1e2",
    ] {
        let input = format!("ethereum:{JPYC}@{chain}/transfer?address={TO}");
        assert_invalid_eip681(&input);
    }
    // `@` の重複もチェーンIDの構文エラーになる。
    assert_invalid_eip681(&format!("ethereum:{JPYC}@137@1/transfer?address={TO}"));
}

// ---------------------------------------------------------------------------
// 関数名
// ---------------------------------------------------------------------------

#[test]
fn approve_rejected() {
    assert_eq!(
        parse_eip681(&uri(&format!("/approve?address={TO}&uint256=1"))),
        Err(ParseError::UnsupportedFunction {
            name: "approve".to_string()
        })
    );
}

#[test]
fn function_name_case_sensitive() {
    assert_eq!(
        parse_eip681(&uri(&format!("/Transfer?address={TO}"))),
        Err(ParseError::UnsupportedFunction {
            name: "Transfer".to_string()
        })
    );
}

#[test]
fn no_function_rejected() {
    // ネイティブ通貨送金の形式。
    assert_eq!(
        parse_eip681(&uri(&format!("?address={TO}&value=1"))),
        Err(ParseError::UnsupportedFunction {
            name: String::new()
        })
    );
    assert_eq!(
        parse_eip681(&uri("")),
        Err(ParseError::UnsupportedFunction {
            name: String::new()
        })
    );
}

#[test]
fn empty_function_name_rejected() {
    assert_invalid_eip681(&uri(&format!("/?address={TO}")));
}

// ---------------------------------------------------------------------------
// パラメータ
// ---------------------------------------------------------------------------

#[test]
fn missing_address_rejected() {
    assert_eq!(
        parse_eip681(&uri("/transfer?uint256=1")),
        Err(ParseError::MissingParam { name: "address" })
    );
    assert_eq!(
        parse_eip681(&uri("/transfer")),
        Err(ParseError::MissingParam { name: "address" })
    );
}

#[test]
fn value_param_rejected() {
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={TO}&uint256=1&value=1"))),
        Err(ParseError::UnsupportedParam {
            name: "value".to_string()
        })
    );
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={TO}&value="))),
        Err(ParseError::UnsupportedParam {
            name: "value".to_string()
        })
    );
}

#[test]
fn unknown_param_rejected() {
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={TO}&foo=bar"))),
        Err(ParseError::UnsupportedParam {
            name: "foo".to_string()
        })
    );
    // キーの大文字小文字は区別する。
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?Address={TO}"))),
        Err(ParseError::UnsupportedParam {
            name: "Address".to_string()
        })
    );
}

#[test]
fn duplicate_address_rejected() {
    let other = "0x0000000000000000000000000000000000000001";
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={TO}&address={other}"))),
        Err(ParseError::DuplicateParam {
            name: "address".to_string()
        })
    );
    // 同一値の重複も拒否する。
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={TO}&address={TO}"))),
        Err(ParseError::DuplicateParam {
            name: "address".to_string()
        })
    );
}

#[test]
fn duplicate_via_percent_encoded_key_rejected() {
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={TO}&addr%65ss={TO}"))),
        Err(ParseError::DuplicateParam {
            name: "address".to_string()
        })
    );
}

#[test]
fn duplicate_uint256_rejected() {
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={TO}&uint256=1&uint256=2"))),
        Err(ParseError::DuplicateParam {
            name: "uint256".to_string()
        })
    );
}

#[test]
fn duplicate_gas_param_rejected() {
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={TO}&gas=1&gas=2"))),
        Err(ParseError::DuplicateParam {
            name: "gas".to_string()
        })
    );
}

#[test]
fn gas_params_ignored() {
    let input = uri(&format!(
        "/transfer?address={TO}&gas=21000&gasLimit=60000&gasPrice=2.014e9&uint256=5"
    ));
    let link = parse_eip681(&input).unwrap();
    assert_eq!(link.to, to_addr());
    assert_eq!(link.amount, Some(U256::from(5)));
}

#[test]
fn malformed_query_rejected() {
    // 空の要素
    assert_invalid_eip681(&uri(&format!("/transfer?address={TO}&")));
    assert_invalid_eip681(&uri(&format!("/transfer?&address={TO}")));
    assert_invalid_eip681(&uri(&format!("/transfer?address={TO}&&uint256=1")));
    assert_invalid_eip681(&uri("/transfer?"));
    // `=` なし
    assert_invalid_eip681(&uri(&format!("/transfer?address={TO}&uint256")));
    // 不正なパーセントエスケープ
    assert_invalid_eip681(&uri(&format!("/transfer?address={TO}&uint256=%zz")));
    assert_invalid_eip681(&uri(&format!("/transfer?address={TO}&uint256=1%")));
    assert_invalid_eip681(&uri(&format!("/transfer?address={TO}&uint256=1%4")));
    // デコード結果が不正なUTF-8
    assert_invalid_eip681(&uri(&format!("/transfer?address={TO}&gas=%ff")));
}

#[test]
fn percent_encoded_address_decoded() {
    // `0x` の `x` をエンコード。
    let encoded = format!("0%78{}", &TO[2..]);
    let link = parse_eip681(&uri(&format!("/transfer?address={encoded}"))).unwrap();
    assert_eq!(link.to, to_addr());
    // すべての文字をエンコード。
    let fully: String = TO.bytes().map(|b| format!("%{b:02X}")).collect();
    let link = parse_eip681(&uri(&format!("/transfer?address={fully}"))).unwrap();
    assert_eq!(link.to, to_addr());
}

// ---------------------------------------------------------------------------
// 受取人アドレス
// ---------------------------------------------------------------------------

#[test]
fn recipient_lowercase_accepted() {
    let lower = TO.to_ascii_lowercase();
    let link = parse_eip681(&uri(&format!("/transfer?address={lower}"))).unwrap();
    assert_eq!(link.to, to_addr());
}

#[test]
fn recipient_uppercase_accepted() {
    let upper = format!("0x{}", TO[2..].to_ascii_uppercase());
    let link = parse_eip681(&uri(&format!("/transfer?address={upper}"))).unwrap();
    assert_eq!(link.to, to_addr());
}

#[test]
fn recipient_bad_checksum_rejected() {
    // 先頭の `a` を `A` にしただけの混在表記（チェックサム不一致）。
    let bad = TO.replacen("0x9aD4", "0x9AD4", 1);
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={bad}"))),
        Err(ParseError::InvalidAddress { value: bad })
    );
}

#[test]
fn recipient_invalid_rejected() {
    for bad in ["", "vitalik.eth", "0x1234", &TO[2..]] {
        assert_eq!(
            parse_eip681(&uri(&format!("/transfer?address={bad}"))),
            Err(ParseError::InvalidAddress {
                value: bad.to_string()
            }),
            "{bad:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 金額（uint256）
// ---------------------------------------------------------------------------

#[test]
fn amount_plain_integer() {
    assert_eq!(amount_of("100").unwrap(), Some(U256::from(100)));
    assert_eq!(amount_of("0").unwrap(), Some(U256::ZERO));
    // 先頭ゼロは10進として扱う（8進や16進として再解釈しない）。
    assert_eq!(amount_of("0100").unwrap(), Some(U256::from(100)));
}

#[test]
fn amount_scientific_notation() {
    assert_eq!(
        amount_of("1e18").unwrap(),
        Some(u256("1000000000000000000"))
    );
    assert_eq!(
        amount_of("1E18").unwrap(),
        Some(u256("1000000000000000000"))
    );
    assert_eq!(
        amount_of("2.014e18").unwrap(),
        Some(u256("2014000000000000000"))
    );
    assert_eq!(amount_of("1.50e1").unwrap(), Some(U256::from(15)));
    assert_eq!(amount_of(".5e1").unwrap(), Some(U256::from(5)));
    assert_eq!(amount_of("0.05e2").unwrap(), Some(U256::from(5)));
    assert_eq!(amount_of("5.0").unwrap(), Some(U256::from(5)));
    assert_eq!(amount_of("1000e0").unwrap(), Some(U256::from(1000)));
    assert_eq!(
        amount_of("1e000018").unwrap(),
        Some(u256("1000000000000000000"))
    );
}

#[test]
fn amount_plus_sign_accepted() {
    assert_eq!(amount_of("+5").unwrap(), Some(U256::from(5)));
    // `%2B` でエンコードされた `+` も同様。`+` を空白とみなすデコードはしない。
    assert_eq!(amount_of("%2B5").unwrap(), Some(U256::from(5)));
}

#[test]
fn amount_negative_rejected() {
    assert_invalid_amount("-5");
    assert_invalid_amount("-0");
}

#[test]
fn amount_not_integer_rejected() {
    assert_invalid_amount("1.5");
    assert_invalid_amount("0.1");
    assert_invalid_amount("2.0145e3");
}

#[test]
fn amount_malformed_rejected() {
    for bad in [
        "", "+", "e18", ".e1", "1.", "1.e5", "1e", "1E", "1e-1", "1e+1", "0x10", "0X10", "10x",
        "1,000", "1_000", "--1", "+-1", "++1", "1e1e1", "1.2.3", "abc",
    ] {
        assert_invalid_amount(bad);
    }
    // デコード後に空白を含む値も拒否する（エラーの `value` はデコード後の値）。
    assert!(matches!(
        amount_of("1%20"),
        Err(ParseError::InvalidAmount { value, .. }) if value == "1 "
    ));
}

#[test]
fn amount_absent_is_none() {
    assert_eq!(
        parse_eip681(&uri(&format!("/transfer?address={TO}")))
            .unwrap()
            .amount,
        None
    );
}

#[test]
fn amount_u256_boundary() {
    let max = U256::MAX.to_string();
    assert_eq!(max.len(), 78);
    assert_eq!(amount_of(&max).unwrap(), Some(U256::MAX));

    // 2^256（= U256::MAX + 1。末尾の桁は5なので繰り上がりはない）。
    let mut over = max.clone();
    over.pop();
    over.push('6');
    assert_invalid_amount(&over);

    // 指数表記でも同じ境界。
    let max_exp = format!("{}.{}e77", &max[..1], &max[1..]);
    assert_eq!(amount_of(&max_exp).unwrap(), Some(U256::MAX));
    let over_exp = format!("{}.{}e77", &over[..1], &over[1..]);
    assert_invalid_amount(&over_exp);

    // 桁数だけで明らかに溢れるもの。
    assert_invalid_amount("1e78");
    assert_invalid_amount("2e77");
    assert_eq!(
        amount_of("1e77").unwrap(),
        Some(U256::from(10).pow(U256::from(77)))
    );
}

#[test]
fn amount_huge_exponent_rejected_quickly() {
    let start = Instant::now();
    assert_invalid_amount("1e99999999999999999999");
    assert_invalid_amount("1e99999999999999999999999999999999999999999999999999");
    assert_invalid_amount("1.5e99999999999999999999");
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn amount_zero_mantissa_with_huge_exponent_is_zero() {
    assert_eq!(
        amount_of("0e99999999999999999999").unwrap(),
        Some(U256::ZERO)
    );
    assert_eq!(
        amount_of("0.000e99999999999999999999").unwrap(),
        Some(U256::ZERO)
    );
}

#[test]
fn amount_many_digits_bounded() {
    let start = Instant::now();

    // 10,000桁の値はU256に収まらない。
    let big = format!("1{}", "0".repeat(9_999));
    assert_eq!(big.len(), 10_000);
    assert_invalid_amount(&big);

    // 先頭ゼロが大量にあっても値自体が小さければ受け付ける。
    let padded = format!("{}42", "0".repeat(9_998));
    assert_eq!(amount_of(&padded).unwrap(), Some(U256::from(42)));

    // 小数部が長く、指数で打ち消されて整数になるもの（1e9999 / 1e9999 = 1）。
    let long_frac = format!("0.{}1e10000", "0".repeat(9_999));
    assert_eq!(amount_of(&long_frac).unwrap(), Some(U256::from(1)));

    // 小数部の末尾ゼロが大量にあっても整数として扱える。
    let trailing = format!("7.{}", "0".repeat(10_000));
    assert_eq!(amount_of(&trailing).unwrap(), Some(U256::from(7)));

    // 整数にならない長い小数。
    let non_int = format!("1.{}1", "0".repeat(9_998));
    assert_invalid_amount(&non_int);

    assert!(start.elapsed() < Duration::from_secs(2));
}

// ---------------------------------------------------------------------------
// パニックしないこと（雑多な入力）
// ---------------------------------------------------------------------------

#[test]
fn never_panics_on_odd_inputs() {
    let inputs = [
        "ethereum:",
        "ethereum:pay-",
        "ethereum:@",
        "ethereum:/",
        "ethereum:?",
        "ethereum:@/?",
        "ethereum:pay-@137/transfer?address=",
        "ethereum:0x@1/transfer",
        "ethe",
        "ethereum:pay",
        "ethereum:%",
        "ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?=",
        "ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?==",
        "ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?%",
        "日本語",
        "ethereum:日本語",
    ];
    for input in inputs {
        assert!(parse_eip681(input).is_err(), "{input:?}");
    }
}
