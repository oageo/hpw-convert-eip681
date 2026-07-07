//! `hpw-convert-eip681` のWASMバインディング。
//!
//! 設計上のポイント:
//! - エラーはResultのようなオブジェクトとして返すのではなく、タグ付き
//!   `ParseError` enumの形をしたプレーンな値として（`serde_wasm_bindgen`
//!   経由で）JSにthrowする。`isSupported`/`isValidChecksum` は真偽値を
//!   返す述語関数であり、throwしない。
//! - `U256` の金額は常に10進文字列として公開する（2^53を超えると精度が
//!   崩れるJSの `number` にはしない。生のBigIntとしても返さない）。加えて
//!   利便性のため `0x` プレフィックス付き16進文字列も提供する。
//! - `ParsedLink` は `serde-wasm-bindgen` に直接通さない。`alloy_primitives`
//!   自身の `U256`/`Address` に対するserde実装は16進（JSON-RPCの
//!   "quantity" 形式）でシリアライズされるため、上記の10進文字列という
//!   要件を静かに破ってしまう。代わりにgetter付きの `#[wasm_bindgen]`
//!   構造体を手書きし、各フィールドを望む形式に明示的に変換する `From`
//!   実装を通して構築する。
//! - `amount` は元URLに存在しないことがあるため `Option<U256>`。JS側では
//!   その場合 `amount`/`amountHex` ともに `undefined` になる。

use hpw_convert_eip681 as core_lib;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct ParsedLink {
    to: String,                 // EIP-55チェックサム表記
    amount: Option<String>,     // 10進文字列
    amount_hex: Option<String>, // "0x"プレフィックス付き16進文字列
    chain_id: u32,
    currency_symbol: String,
    currency_contract: String, // チェックサム表記
    to_name: Option<String>,
    eip681: String, // 構築時に計算済み
}

#[wasm_bindgen]
impl ParsedLink {
    #[wasm_bindgen(getter)]
    pub fn to(&self) -> String {
        self.to.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> Option<String> {
        self.amount.clone()
    }

    #[wasm_bindgen(getter, js_name = amountHex)]
    pub fn amount_hex(&self) -> Option<String> {
        self.amount_hex.clone()
    }

    #[wasm_bindgen(getter, js_name = chainId)]
    pub fn chain_id(&self) -> u32 {
        self.chain_id
    }

    #[wasm_bindgen(getter, js_name = currencySymbol)]
    pub fn currency_symbol(&self) -> String {
        self.currency_symbol.clone()
    }

    #[wasm_bindgen(getter, js_name = currencyContract)]
    pub fn currency_contract(&self) -> String {
        self.currency_contract.clone()
    }

    #[wasm_bindgen(getter, js_name = toName)]
    pub fn to_name(&self) -> Option<String> {
        self.to_name.clone()
    }

    #[wasm_bindgen(js_name = toEip681)]
    pub fn to_eip681(&self) -> String {
        self.eip681.clone()
    }
}

impl From<core_lib::ParsedLink> for ParsedLink {
    fn from(p: core_lib::ParsedLink) -> Self {
        let eip681 = p.to_eip681();
        Self {
            to: p.to.to_string(),
            amount: p.amount.map(|a| a.to_string()),
            amount_hex: p.amount.map(|a| format!("{a:#x}")),
            // チェーンIDはu32を超えうる（巨大なIDを使うEVMチェーンが存在する）。
            // `as` による黙った切り捨てではなく、収まらない場合は大きな音を
            // 立てて失敗させる。現状はPolygon(137)のみなので到達しない。
            chain_id: u32::try_from(p.chain_id.0)
                .expect("chain id exceeds u32 — widen the JS-facing chainId type"),
            currency_symbol: p.currency.symbol().to_string(),
            currency_contract: p.currency.contract_address().to_string(),
            to_name: p.to_name,
            eip681,
        }
    }
}

fn to_js_error(e: core_lib::ParseError) -> JsValue {
    serde_wasm_bindgen::to_value(&e).unwrap_or_else(|_| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = isSupported)]
pub fn is_supported(url: &str) -> bool {
    core_lib::is_supported(url)
}

#[wasm_bindgen(js_name = parseHashportLink)]
pub fn parse_hashport_link(url: &str) -> Result<ParsedLink, JsValue> {
    core_lib::parse(url)
        .map(ParsedLink::from)
        .map_err(to_js_error)
}

#[wasm_bindgen(js_name = isValidChecksum)]
pub fn is_valid_checksum(address: &str) -> bool {
    core_lib::validate_checksum(address).is_ok()
}
