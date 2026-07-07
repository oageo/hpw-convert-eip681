use alloy_primitives::U256;

use crate::error::ParseError;

/// URL側の `amount` クエリ値（`0x`/`0X` プレフィックスの有無が不定な、
/// 16進エンコードされたuint256）をデコードする。
///
/// `U256::from_str`（`ruint` の素の `FromStr` 実装）は `0x`/`0o`/`0b`
/// プレフィックスから基数を推測し、プレフィックスがなければ10進とみなす。
/// しかし本仕様では `amount` はプレフィックスなしの16進としても届き得るため、
/// プレフィックスなしの数字だけの値（例: "1000"）に対して素の `FromStr` を
/// 使うと、本来16進0x1000（4096）であるべき値を10進1000として誤って
/// パースしてしまう（エラーにすらならない値破壊バグになる）。そのため、
/// 必ずプレフィックスを手動で取り除いたうえで基数16を明示して
/// `from_str_radix` を呼ぶ。
pub fn parse_amount_hex(raw: &str) -> Result<U256, ParseError> {
    let trimmed = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);

    // この空文字チェックは必須である（単なるエラーメッセージ改善ではない）。
    // ruintの `from_str_radix` は空文字列に対してエラーではなく `Ok(0)` を
    // 返すため、このチェックがないと `amount=` や `amount=0x` が静かに
    // 金額0として解釈されてしまう。
    if trimmed.is_empty() {
        return Err(ParseError::InvalidAmount {
            value: raw.to_string(),
            reason: "empty amount".to_string(),
        });
    }

    U256::from_str_radix(trimmed, 16).map_err(|e| ParseError::InvalidAmount {
        value: raw.to_string(),
        reason: e.to_string(),
    })
}
