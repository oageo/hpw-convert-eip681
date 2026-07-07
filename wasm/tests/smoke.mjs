// `hpw-convert-eip681-wasm` パッケージのスモークテスト。
// `wasm-pack build --target nodejs --out-dir pkg-node --out-name
// hpw_convert_eip681 ...` でビルドしたものに対して実行する。
//
// 実行方法: node wasm/tests/smoke.mjs
// （importはこのファイルからの相対パスで解決されるため、どのディレクトリ
//   から実行しても動く）
import assert from "node:assert/strict";
import {
  isSupported,
  isValidChecksum,
  parseHashportLink,
} from "../pkg-node/hpw_convert_eip681.js";

const VALID_LINK =
  "https://link.expo2025-wallet.com/pay?to=0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed&master_currency_id=487&amount=0xde0b6b3a7640000&to_name=Cafe";

// isSupported
assert.equal(isSupported(VALID_LINK), true, "isSupported should accept a valid HashPort link");
assert.equal(isSupported("not a url"), false, "isSupported should reject garbage input");

// parseHashportLink の正常系
const parsed = parseHashportLink(VALID_LINK);
assert.equal(parsed.amount, "1000000000000000000", "amount should be a decimal string");
assert.equal(parsed.to, "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed", "to should be EIP-55 checksummed");
assert.equal(
  parsed.toEip681(),
  "ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?address=0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed&uint256=1000000000000000000",
  "toEip681() should produce the expected ERC-681 URI"
);

// amount パラメータが元URLに存在しない場合は amount/amountHex が undefined になり、
// EIP-681出力からも uint256= が省かれる
const withoutAmount = parseHashportLink(
  "https://link.expo2025-wallet.com/pay?to=0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed&master_currency_id=487"
);
assert.equal(withoutAmount.amount, undefined, "amount should be undefined when the param is absent");
assert.equal(withoutAmount.amountHex, undefined, "amountHex should be undefined when the param is absent");
assert.equal(
  withoutAmount.toEip681(),
  "ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?address=0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
  "toEip681() should omit uint256= when amount is absent"
);

// isValidChecksum（EIP-55チェックサム検証、throwしない述語関数）
assert.equal(
  isValidChecksum("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed"),
  true,
  "isValidChecksum should accept a correctly checksummed address"
);
assert.equal(
  isValidChecksum("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed"),
  false,
  "isValidChecksum should reject an address with a wrong checksum"
);

// parseHashportLink の異常系
let threw = false;
try {
  parseHashportLink("https://evil.example.com/pay");
} catch (err) {
  threw = true;
  assert.equal(err.kind, "UnsupportedHost", "thrown error should carry kind = UnsupportedHost");
  assert.equal(err.host, "evil.example.com", "thrown error should carry the offending host");
}
assert.equal(threw, true, "parseHashportLink should throw for an unsupported host");

console.log("wasm smoke test passed");
