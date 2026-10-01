use std::fmt::Write as _;

use alloy_primitives::{Address, U256};
use percent_encoding::{utf8_percent_encode, AsciiSet, PercentEncode, NON_ALPHANUMERIC};

use crate::amount;
use crate::constants;
use crate::currency::Currency;
use crate::error::ParseError;

/// HashPortリンクのクエリ値（`to_name` / `type`）でパーセントエンコードする
/// 文字の集合。RFC 3986の非予約文字（`A-Z a-z 0-9 - . _ ~`）以外の
/// すべてのバイトを `%XX`（大文字16進）にエンコードする。
///
/// これはセキュリティ上重要である: `&` `=` `#` `%` `+` `?` `/` 改行・NUL・
/// 非ASCII等をすべてエスケープするため、`to_name` の内容（例:
/// `x&to=0x攻撃者のアドレス`）が他のパラメータを注入・改変することはできない。
/// 空白は `+` ではなく必ず `%20` にする。HashPort側がフォームエンコーディング
/// （`+` を空白とみなす）と `decodeURIComponent`（`+` をそのまま残す）の
/// どちらでデコードするか不明だが、`%20` はどちらでも空白に戻る一方、`+` は
/// 後者では空白に戻らないため。
const QUERY_VALUE_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// クエリ値を [`QUERY_VALUE_ENCODE_SET`] に従ってパーセントエンコードする。
fn encode_query_value(value: &str) -> PercentEncode<'_> {
    utf8_percent_encode(value, QUERY_VALUE_ENCODE_SET)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainId(pub u64);

impl ChainId {
    pub const ETHEREUM: ChainId = ChainId(constants::ETHEREUM_CHAIN_ID);
    pub const POLYGON: ChainId = ChainId(constants::POLYGON_CHAIN_ID);
    pub const KAIA: ChainId = ChainId(constants::KAIA_CHAIN_ID);
    pub const AVALANCHE: ChainId = ChainId(constants::AVALANCHE_CHAIN_ID);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLink {
    pub to: Address,
    pub currency: Currency,
    pub chain_id: ChainId,
    /// 最小単位（10^decimals倍済み）の金額。URL側の16進 `amount` を
    /// デコードしたもの。`amount` パラメータ自体が省略されている元URLも
    /// あるため、`None` はパラメータが存在しなかったことを表す。
    pub amount: Option<U256>,
    /// `to_name` の生の（非検証・非加工の）表示ラベル。パラメータが
    /// 全く存在しない場合のみ `None`。存在するが空文字の場合は
    /// `Some(String::new())`。
    pub to_name: Option<String>,
    /// `type` クエリパラメータの生の（非検証・非加工の）値（例:
    /// `"dynamic"`）。HashPort Wallet側の仕様変更で付与されるように
    /// なったものだが、取りうる値の全体像が未確認のため `to_name` と
    /// 同様に解釈を加えず生の文字列のまま保持する。`None` はパラメータが
    /// 全く存在しなかったことを表す。
    pub link_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressAmount {
    pub address: Address,
    pub amount: Option<U256>,
}

impl ParsedLink {
    /// EIP-681（ERC-681）形式の決済URIを生成する。`U256` の `Display` は
    /// 10進表記であり、これはERC-681仕様が `uint256=` の値として要求する
    /// 形式そのものである（URL側の `amount` は16進で符号化されているが、
    /// ここでは10進に変換して出力する）。金額が省略されている場合は
    /// `uint256=` パラメータ自体を省略する（EIP-681は金額未指定の
    /// リクエストを許容しており、ウォレット側でユーザーに入力させる想定）。
    pub fn to_eip681(&self) -> String {
        let contract = self.currency.contract_address();
        let chain_id = self.chain_id.0;
        let mut uri = format!(
            "ethereum:{contract}@{chain_id}/transfer?address={}",
            self.to
        );
        if let Some(amount) = self.amount {
            // `String` への `write!` は失敗しない。
            let _ = write!(uri, "&uint256={amount}");
        }
        uri
    }

    /// HashPort Wallet決済リンク
    /// （`https://link.expo2025-wallet.com/pay?to=...&master_currency_id=...`）
    /// を生成する。[`ParsedLink::to_eip681`] の逆方向。
    ///
    /// パラメータの順序・表記はHashPort Walletが実際に生成するリンクに
    /// 合わせる（`to` → `master_currency_id` → `amount` → `to_name` →
    /// `type`）。`to` はEIP-55チェックサム表記、`amount` は `0x` + 小文字
    /// 16進64桁のゼロ埋め。`amount` / `to_name` / `type` は `None` なら
    /// パラメータ自体を省略し、`Some("")` なら `to_name=` のように値が空の
    /// パラメータとして出力する（[`crate::parse`] で往復させても区別が保たれる）。
    ///
    /// フィールドが `pub` であり任意の組を構築できるため、（通貨, チェーン）の
    /// 組が対応表にない場合は `ParseError::UnsupportedChain` を返す。
    pub fn to_hashport_link(&self) -> Result<String, ParseError> {
        let master_currency_id =
            self.currency
                .master_currency_id(self.chain_id)
                .ok_or_else(|| ParseError::UnsupportedChain {
                    chain_id: self.chain_id.0.to_string(),
                })?;

        let mut link = format!(
            "{}://{}{}?to={}&master_currency_id={master_currency_id}",
            constants::HASHPORT_LINK_SCHEME,
            constants::EXPECTED_HOST,
            constants::HASHPORT_LINK_PATH,
            self.to,
        );
        // `String` への `write!` は失敗しない。
        if let Some(amount) = self.amount {
            let _ = write!(link, "&amount={}", amount::encode_amount_hex(amount));
        }
        if let Some(to_name) = &self.to_name {
            let _ = write!(link, "&to_name={}", encode_query_value(to_name));
        }
        if let Some(link_type) = &self.link_type {
            let _ = write!(link, "&type={}", encode_query_value(link_type));
        }
        Ok(link)
    }

    pub fn address(&self) -> Address {
        self.to
    }

    pub fn amount(&self) -> Option<U256> {
        self.amount
    }

    pub fn address_amount(&self) -> AddressAmount {
        AddressAmount {
            address: self.to,
            amount: self.amount,
        }
    }
}
