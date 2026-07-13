use std::fmt::Write as _;

use alloy_primitives::{Address, U256};

use crate::constants;
use crate::currency::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainId(pub u64);

impl ChainId {
    pub const POLYGON: ChainId = ChainId(constants::POLYGON_CHAIN_ID);
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
