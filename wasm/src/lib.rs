//! `hpw-convert-eip681` のWASMバインディング。
//!
//! 設計上のポイント:
//! - エラーはResultのようなオブジェクトとして返すのではなく、タグ付き
//!   `ParseError` enumの形をしたプレーンな値として（`serde_wasm_bindgen`
//!   経由で）JSにthrowする。`isSupported`/`isSupportedEip681`/
//!   `isValidChecksum` は真偽値を返す述語関数であり、throwしない。
//! - `U256` の金額は常に10進文字列として公開する（2^53を超えると精度が
//!   崩れるJSの `number` にはしない。生のBigIntとしても返さない）。加えて
//!   利便性のため `0x` プレフィックス付き16進文字列も提供する。
//! - `ParsedLink` は `serde-wasm-bindgen` に直接通さない。`alloy_primitives`
//!   自身の `U256`/`Address` に対するserde実装は16進（JSON-RPCの
//!   "quantity" 形式）でシリアライズされるため、上記の10進文字列という
//!   要件を静かに破ってしまう。代わりにgetter付きの `#[wasm_bindgen]`
//!   構造体を手書きし、各getterで望む形式に明示的に変換する。
//! - `amount` は元URLに存在しないことがあるため `Option<U256>`。JS側では
//!   その場合 `amount`/`amountHex` ともに `undefined` になる。
//! - `linkType`（元URLの `type` パラメータ、例: `"dynamic"`）は `toName`
//!   と同様、解釈を加えず生の文字列のまま公開する。取りうる値の全体像が
//!   未確認のため、bool等へは決め打ちしない。
//! - wasm内でのパニックはモジュール全体を異常終了させるため、ユーザー入力
//!   から到達しうる経路にはパニックを置かない（失敗はすべてthrowにする）。

use hpw_convert_eip681 as core_lib;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// JSに公開するパース結果。
///
/// coreの `ParsedLink` をそのまま保持し、getterで都度JS向けの表現に変換する。
/// `toHashportLink()` がcore側の `to_hashport_link` をそのまま呼べるように
/// するため、変換済みの文字列だけでなく元の値を持っておく。
#[wasm_bindgen]
pub struct ParsedLink {
    inner: core_lib::ParsedLink,
    // JS向けのチェーンID。getter呼び出し時ではなく構築時に一度だけ変換する
    // （下記 `From` 実装を参照）。
    chain_id: u32,
}

#[wasm_bindgen]
impl ParsedLink {
    /// EIP-55チェックサム表記の受取人アドレス。
    #[wasm_bindgen(getter)]
    pub fn to(&self) -> String {
        self.inner.to.to_string()
    }

    /// 10進文字列の金額（最小単位）。元に金額がなければ `undefined`。
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> Option<String> {
        self.inner.amount.map(|a| a.to_string())
    }

    /// `0x` プレフィックス付き16進文字列の金額。元に金額がなければ `undefined`。
    #[wasm_bindgen(getter, js_name = amountHex)]
    pub fn amount_hex(&self) -> Option<String> {
        self.inner.amount.map(|a| format!("{a:#x}"))
    }

    #[wasm_bindgen(getter, js_name = chainId)]
    pub fn chain_id(&self) -> u32 {
        self.chain_id
    }

    #[wasm_bindgen(getter, js_name = currencySymbol)]
    pub fn currency_symbol(&self) -> String {
        self.inner.currency.symbol().to_string()
    }

    /// チェックサム表記のトークンコントラクトアドレス。
    #[wasm_bindgen(getter, js_name = currencyContract)]
    pub fn currency_contract(&self) -> String {
        self.inner.currency.contract_address().to_string()
    }

    #[wasm_bindgen(getter, js_name = toName)]
    pub fn to_name(&self) -> Option<String> {
        self.inner.to_name.clone()
    }

    #[wasm_bindgen(getter, js_name = linkType)]
    pub fn link_type(&self) -> Option<String> {
        self.inner.link_type.clone()
    }

    #[wasm_bindgen(js_name = toEip681)]
    pub fn to_eip681(&self) -> String {
        self.inner.to_eip681()
    }

    /// HashPort Wallet決済リンクを生成する。失敗時はタグ付きエラーをthrowする。
    ///
    /// このライブラリが返す `ParsedLink` は（通貨, チェーン）の組が検証済み
    /// なので実際には失敗しないが、パニックではなくthrowにしておく。
    #[wasm_bindgen(js_name = toHashportLink)]
    pub fn to_hashport_link(&self) -> Result<String, JsValue> {
        self.inner.to_hashport_link().map_err(to_js_error)
    }
}

impl From<core_lib::ParsedLink> for ParsedLink {
    fn from(inner: core_lib::ParsedLink) -> Self {
        // チェーンIDはu32を超えうる（巨大なIDを使うEVMチェーンが存在する）。
        // `as` による黙った切り捨てではなく、収まらない場合は大きな音を
        // 立てて失敗させる。現状の対応チェーン（Ethereum 1 / Polygon 137 /
        // Kaia 8217 / Avalanche 43114）はいずれも収まり、coreのパーサーは
        // 対応表にないチェーンをエラーにするので到達しない。getterではなく
        // 構築時に変換しておくことで、万一の失敗もパース直後の一点に限られ、
        // 後からgetterを読んだ瞬間に遅れてパニックすることがない。
        let chain_id = u32::try_from(inner.chain_id.0)
            .expect("chain id exceeds u32 — widen the JS-facing chainId type");
        Self { inner, chain_id }
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

/// `uri` がこのクレートでパース可能なEIP-681 URIかどうかを返す。
/// `isSupported` のEIP-681版で、throwしない。
#[wasm_bindgen(js_name = isSupportedEip681)]
pub fn is_supported_eip681(uri: &str) -> bool {
    core_lib::parse_eip681(uri).is_ok()
}

/// EIP-681 URIをパースする。失敗時はタグ付きエラーをthrowする。
///
/// EIP-681のURIは `to_name`・`type` に相当する情報を持たないため、呼び出し側が
/// `toName`・`linkType` で与える（`toHashportLink()` で生成されるリンクの
/// `to_name`・`type` になる）。wasm-bindgenの `Option<String>` 引数は
/// `undefined`（省略）と `null` を `None`（パラメータなし）に、空文字
/// `""` を `Some("")`（値が空のパラメータとして出力）に対応付ける。
#[wasm_bindgen(js_name = parseEip681)]
pub fn parse_eip681(
    uri: &str,
    // `.d.ts` 上の引数名をJSの命名規則（camelCase）に合わせる
    #[wasm_bindgen(js_name = toName)] to_name: Option<String>,
    #[wasm_bindgen(js_name = linkType)] link_type: Option<String>,
) -> Result<ParsedLink, JsValue> {
    let mut inner = core_lib::parse_eip681(uri).map_err(to_js_error)?;
    inner.to_name = to_name;
    inner.link_type = link_type;
    Ok(ParsedLink::from(inner))
}

#[wasm_bindgen(js_name = isValidChecksum)]
pub fn is_valid_checksum(address: &str) -> bool {
    core_lib::validate_checksum(address).is_ok()
}
