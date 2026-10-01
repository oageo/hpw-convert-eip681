use hpw_convert_eip681::{parse, Address, ChainId, Currency, ParseError, ParsedLink, U256};

/// HashPort Walletが実際に生成した決済リンクのクエリ部分（`master_currency_id`
/// 以外）。チェーンごとに `master_currency_id` だけが異なる。
const REAL_TO: &str = "0x9aD4Ba3D9FB338Cd9C836cD5f222BB5fF8ab2456";
const REAL_AMOUNT: &str = "0x0000000000000000000000000000000000000000000000056bc75e2d63100000";

fn real_link(master_currency_id: &str) -> String {
    format!(
        "https://link.expo2025-wallet.com/pay?to={REAL_TO}&master_currency_id={master_currency_id}&amount={REAL_AMOUNT}&to_name=oa&type=dynamic"
    )
}

fn to_addr() -> Address {
    REAL_TO.parse().unwrap()
}

/// 100 JPYC（100 * 10^18）。
fn hundred_tokens() -> U256 {
    U256::from(100u64) * U256::from(10u64).pow(U256::from(18u64))
}

fn link(
    chain_id: ChainId,
    amount: Option<U256>,
    to_name: Option<&str>,
    link_type: Option<&str>,
) -> ParsedLink {
    ParsedLink {
        to: to_addr(),
        currency: Currency::Jpyc,
        chain_id,
        amount,
        to_name: to_name.map(str::to_string),
        link_type: link_type.map(str::to_string),
    }
}

/// クエリ文字列から `name=` で始まるパラメータの生の（デコード前の）値を
/// 取り出す。パラメータが存在しなければ `None`。
fn raw_param<'a>(link: &'a str, name: &str) -> Option<&'a str> {
    let query = link.split_once('?').unwrap().1;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
}

#[test]
fn real_links_round_trip_byte_identical() {
    for id in ["487", "489", "490", "712"] {
        let original = real_link(id);
        let parsed = parse(&original).expect("should parse");
        assert_eq!(
            parsed.to_hashport_link().expect("should encode"),
            original,
            "master_currency_id={id}"
        );
    }
}

#[test]
fn real_link_decodes_to_expected_fields() {
    let parsed = parse(&real_link("490")).unwrap();
    assert_eq!(
        parsed,
        link(
            ChainId::ETHEREUM,
            Some(hundred_tokens()),
            Some("oa"),
            Some("dynamic")
        )
    );
}

#[test]
fn round_trip_through_parse_for_varied_links() {
    let names: &[Option<&str>] = &[
        None,
        Some(""),
        Some("Test Shop"),
        Some("  leading and trailing  "),
        Some("a&b"),
        Some("a=b"),
        Some("a#b"),
        Some("100%"),
        Some("%20"),
        Some("%zz"),
        Some("a+b"),
        Some("a?b"),
        Some("a/b\\c"),
        Some("テスト商店"),
        Some("お店🍣🎌"),
        Some("line1\nline2\r\n"),
        Some("tab\there"),
        Some("nul\0byte"),
        Some("-._~"),
        Some("!*'();:@$,[]\"<>{}|^`"),
    ];
    let types: &[Option<&str>] = &[None, Some(""), Some("dynamic"), Some("static & weird=1")];
    let amounts = [
        None,
        Some(U256::ZERO),
        Some(U256::from(1u64)),
        Some(hundred_tokens()),
        Some(U256::MAX),
    ];
    let chains = [
        ChainId::POLYGON,
        ChainId::AVALANCHE,
        ChainId::ETHEREUM,
        ChainId::KAIA,
    ];

    for &chain in &chains {
        for &amount in &amounts {
            for &name in names {
                for &ty in types {
                    let p = link(chain, amount, name, ty);
                    let encoded = p.to_hashport_link().expect("should encode");
                    assert_eq!(parse(&encoded), Ok(p.clone()), "encoded: {encoded}");
                }
            }
        }
    }
}

#[test]
fn to_name_cannot_inject_other_params() {
    let evil = "x&to=0x0000000000000000000000000000000000000001&master_currency_id=1";
    let p = link(
        ChainId::POLYGON,
        Some(hundred_tokens()),
        Some(evil),
        Some(evil),
    );
    let encoded = p.to_hashport_link().unwrap();

    let reparsed = parse(&encoded).expect("should parse");
    assert_eq!(reparsed.to, to_addr());
    assert_eq!(reparsed.chain_id, ChainId::POLYGON);
    assert_eq!(reparsed.currency, Currency::Jpyc);
    assert_eq!(reparsed.amount, Some(hundred_tokens()));
    assert_eq!(reparsed.to_name.as_deref(), Some(evil));
    assert_eq!(reparsed.link_type.as_deref(), Some(evil));

    // 生のクエリ上でも、各パラメータがちょうど1回ずつしか現れないこと。
    let query = encoded.split_once('?').unwrap().1;
    let keys: Vec<&str> = query
        .split('&')
        .map(|pair| pair.split_once('=').unwrap().0)
        .collect();
    assert_eq!(
        keys,
        ["to", "master_currency_id", "amount", "to_name", "type"]
    );
    assert!(!encoded.contains('#'));
}

#[test]
fn space_is_encoded_as_percent_20_never_plus() {
    let p = link(ChainId::POLYGON, None, Some("Test Shop a+b"), Some("a b"));
    let encoded = p.to_hashport_link().unwrap();
    assert_eq!(raw_param(&encoded, "to_name"), Some("Test%20Shop%20a%2Bb"));
    assert_eq!(raw_param(&encoded, "type"), Some("a%20b"));
    assert!(!encoded.contains('+'));
}

#[test]
fn only_unreserved_characters_are_left_unencoded() {
    let p = link(
        ChainId::POLYGON,
        None,
        Some("AZaz09-._~ &=#%+?/\n\0あ"),
        None,
    );
    let encoded = p.to_hashport_link().unwrap();
    assert_eq!(
        raw_param(&encoded, "to_name"),
        Some("AZaz09-._~%20%26%3D%23%25%2B%3F%2F%0A%00%E3%81%82")
    );
}

#[test]
fn optional_params_none_are_omitted_and_empty_are_present() {
    let none = link(ChainId::POLYGON, None, None, None)
        .to_hashport_link()
        .unwrap();
    assert_eq!(
        none,
        format!("https://link.expo2025-wallet.com/pay?to={REAL_TO}&master_currency_id=487")
    );

    let empty = link(ChainId::POLYGON, None, Some(""), Some(""))
        .to_hashport_link()
        .unwrap();
    assert_eq!(
        empty,
        format!(
            "https://link.expo2025-wallet.com/pay?to={REAL_TO}&master_currency_id=487&to_name=&type="
        )
    );
}

#[test]
fn to_is_eip55_checksummed() {
    let lower: Address = REAL_TO.to_lowercase().parse().unwrap();
    let mut p = link(ChainId::POLYGON, None, None, None);
    p.to = lower;
    let encoded = p.to_hashport_link().unwrap();
    assert_eq!(raw_param(&encoded, "to"), Some(REAL_TO));
}

#[test]
fn unsupported_chain_is_rejected() {
    let p = link(ChainId(56), Some(hundred_tokens()), Some("oa"), None);
    match p.to_hashport_link() {
        Err(ParseError::UnsupportedChain { chain_id }) => assert_eq!(chain_id, "56"),
        other => panic!("expected UnsupportedChain, got {other:?}"),
    }

    // u64の最大値でもパニックせず10進文字列で返ること。
    let p = link(ChainId(u64::MAX), None, None, None);
    assert_eq!(
        p.to_hashport_link(),
        Err(ParseError::UnsupportedChain {
            chain_id: u64::MAX.to_string()
        })
    );
}

#[test]
fn amount_zero_is_64_zero_digits() {
    let encoded = link(ChainId::POLYGON, Some(U256::ZERO), None, None)
        .to_hashport_link()
        .unwrap();
    assert_eq!(
        raw_param(&encoded, "amount"),
        Some(format!("0x{}", "0".repeat(64)).as_str())
    );
}

#[test]
fn amount_max_is_64_f_digits() {
    let encoded = link(ChainId::POLYGON, Some(U256::MAX), None, None)
        .to_hashport_link()
        .unwrap();
    assert_eq!(
        raw_param(&encoded, "amount"),
        Some(format!("0x{}", "f".repeat(64)).as_str())
    );
}

#[test]
fn amount_small_and_mid_values_are_zero_padded_lowercase() {
    // u64 / u128 の範囲内の値（ruintが標準整数型の書式化に委譲する経路）と、
    // u128を超える値（ruint独自の書式化経路）の両方で幅が守られること。
    let cases = [
        (U256::from(1u64), format!("0x{}1", "0".repeat(63))),
        (
            U256::from(u128::MAX),
            format!("0x{}{}", "0".repeat(32), "f".repeat(32)),
        ),
        (
            U256::from(u128::MAX) + U256::from(1u64),
            format!("0x{}1{}", "0".repeat(31), "0".repeat(32)),
        ),
        (hundred_tokens(), REAL_AMOUNT.to_string()),
    ];
    for (amount, expected) in cases {
        let encoded = link(ChainId::POLYGON, Some(amount), None, None)
            .to_hashport_link()
            .unwrap();
        assert_eq!(raw_param(&encoded, "amount"), Some(expected.as_str()));
    }
}
