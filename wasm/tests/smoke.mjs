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
  isSupportedEip681,
  isValidChecksum,
  parseEip681,
  parseHashportLink,
} from "../pkg-node/hpw_convert_eip681.js";

const VALID_LINK =
  "https://link.expo2025-wallet.com/pay?to=0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed&master_currency_id=487&amount=0xde0b6b3a7640000&to_name=Cafe&type=dynamic";

// isSupported
assert.equal(isSupported(VALID_LINK), true, "isSupported should accept a valid HashPort link");
assert.equal(isSupported("not a url"), false, "isSupported should reject garbage input");

// parseHashportLink の正常系
const parsed = parseHashportLink(VALID_LINK);
assert.equal(parsed.amount, "1000000000000000000", "amount should be a decimal string");
assert.equal(parsed.to, "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed", "to should be EIP-55 checksummed");
assert.equal(parsed.linkType, "dynamic", "linkType should carry the raw `type` query value");
assert.equal(
  parsed.toEip681(),
  "ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?address=0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed&uint256=1000000000000000000",
  "toEip681() should produce the expected ERC-681 URI"
);

// master_currency_id によってチェーンが切り替わる（JPYCのコントラクトアドレスは
// 全チェーン共通なので、EIP-681出力で変わるのは @<chainId> の部分のみ）。
// HashPort Walletが実際に生成した100 JPYCの決済リンクを使う。
for (const [masterCurrencyId, chainId] of [
  ["487", 137], // Polygon
  ["489", 43114], // Avalanche C-Chain
  ["490", 1], // Ethereum
  ["712", 8217], // Kaia
]) {
  const link = parseHashportLink(
    `https://link.expo2025-wallet.com/pay?to=0x9aD4Ba3D9FB338Cd9C836cD5f222BB5fF8ab2456&master_currency_id=${masterCurrencyId}&amount=0x0000000000000000000000000000000000000000000000056bc75e2d63100000&to_name=oa&type=dynamic`
  );
  assert.equal(link.chainId, chainId, `master_currency_id=${masterCurrencyId} should map to chain ${chainId}`);
  assert.equal(link.currencySymbol, "JPYC", "currencySymbol should be JPYC on every chain");
  assert.equal(link.amount, "100000000000000000000", "zero-padded hex amount should decode to 100 JPYC");
  assert.equal(
    link.toEip681(),
    `ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@${chainId}/transfer?address=0x9aD4Ba3D9FB338Cd9C836cD5f222BB5fF8ab2456&uint256=100000000000000000000`,
    `toEip681() should target chain ${chainId}`
  );
}

// amount パラメータが元URLに存在しない場合は amount/amountHex が undefined になり、
// EIP-681出力からも uint256= が省かれる
const withoutAmount = parseHashportLink(
  "https://link.expo2025-wallet.com/pay?to=0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed&master_currency_id=487"
);
assert.equal(withoutAmount.amount, undefined, "amount should be undefined when the param is absent");
assert.equal(withoutAmount.amountHex, undefined, "amountHex should be undefined when the param is absent");
assert.equal(withoutAmount.linkType, undefined, "linkType should be undefined when the `type` param is absent");
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

// HashPortリンク → EIP-681 → HashPortリンクの往復。HashPort Walletが実際に
// 生成したリンクがバイト単位で完全に復元されることを確認する。
for (const [masterCurrencyId, chainId] of [
  ["487", 137], // Polygon
  ["489", 43114], // Avalanche C-Chain
  ["490", 1], // Ethereum
  ["712", 8217], // Kaia
]) {
  const original = `https://link.expo2025-wallet.com/pay?to=0x9aD4Ba3D9FB338Cd9C836cD5f222BB5fF8ab2456&master_currency_id=${masterCurrencyId}&amount=0x0000000000000000000000000000000000000000000000056bc75e2d63100000&to_name=oa&type=dynamic`;
  const uri = parseHashportLink(original).toEip681();
  const fromUri = parseEip681(uri, "oa", "dynamic");
  assert.equal(fromUri.chainId, chainId, `parseEip681 should keep chain ${chainId}`);
  assert.equal(fromUri.amount, "100000000000000000000", "parseEip681 amount should be a decimal string");
  assert.equal(fromUri.toName, "oa", "toName argument should populate toName");
  assert.equal(fromUri.linkType, "dynamic", "linkType argument should populate linkType");
  assert.equal(fromUri.toEip681(), uri, "toEip681() should round-trip the EIP-681 URI");
  assert.equal(
    fromUri.toHashportLink(),
    original,
    `HashPort -> EIP-681 -> HashPort should be byte-identical for master_currency_id=${masterCurrencyId}`
  );
}

const JPYC = "0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29";
const TO = "0x9aD4Ba3D9FB338Cd9C836cD5f222BB5fF8ab2456";
const EIP681_URI = `ethereum:${JPYC}@137/transfer?address=${TO}&uint256=100000000000000000000`;
const HASHPORT_BASE = `https://link.expo2025-wallet.com/pay?to=${TO}&master_currency_id=487&amount=0x0000000000000000000000000000000000000000000000056bc75e2d63100000`;

// toName/linkType を省略・undefined・null にした場合は to_name/type パラメータ自体が出力されない
for (const [label, link] of [
  ["omitted", parseEip681(EIP681_URI)],
  ["undefined", parseEip681(EIP681_URI, undefined, undefined)],
  ["null", parseEip681(EIP681_URI, null, null)],
]) {
  assert.equal(link.toName, undefined, `toName should be undefined when ${label}`);
  assert.equal(link.linkType, undefined, `linkType should be undefined when ${label}`);
  assert.equal(link.toHashportLink(), HASHPORT_BASE, `toHashportLink() should omit to_name/type when ${label}`);
}

// 空文字は「値が空のパラメータ」として保たれる（省略とは区別される）
const emptyName = parseEip681(EIP681_URI, "", "");
assert.equal(emptyName.toName, "", "empty toName should stay an empty string");
assert.equal(emptyName.linkType, "", "empty linkType should stay an empty string");
assert.equal(
  emptyName.toHashportLink(),
  `${HASHPORT_BASE}&to_name=&type=`,
  "empty toName/linkType should produce present-but-empty params"
);
const emptyNameReparsed = parseHashportLink(emptyName.toHashportLink());
assert.equal(emptyNameReparsed.toName, "", "empty to_name should survive re-parsing");
assert.equal(emptyNameReparsed.linkType, "", "empty type should survive re-parsing");

// 金額なしのEIP-681 URI → amount パラメータなしのHashPortリンク
const noAmount = parseEip681(`ethereum:${JPYC}@1/transfer?address=${TO}`);
assert.equal(noAmount.amount, undefined, "amount should be undefined when uint256 is absent");
assert.equal(noAmount.amountHex, undefined, "amountHex should be undefined when uint256 is absent");
assert.equal(
  noAmount.toHashportLink(),
  `https://link.expo2025-wallet.com/pay?to=${TO}&master_currency_id=490`,
  "toHashportLink() should omit amount when absent"
);

// to_name に別パラメータを注入しようとしてもパーセントエンコードされ、
// 再パースしても元の to は変わらない
const injectedName = "a&to=0x0000000000000000000000000000000000000001";
const injectedLink = parseEip681(EIP681_URI, injectedName, "dynamic").toHashportLink();
assert.equal(
  injectedLink,
  `${HASHPORT_BASE}&to_name=a%26to%3D0x0000000000000000000000000000000000000001&type=dynamic`,
  "to_name should be percent-encoded"
);
const injectedReparsed = parseHashportLink(injectedLink);
assert.equal(injectedReparsed.to, TO, "injected to= must not override the recipient");
assert.equal(injectedReparsed.toName, injectedName, "to_name should round-trip verbatim");

// 空白は + ではなく %20 になる
const spaced = parseEip681(EIP681_URI, "Cafe Expo", "dynamic").toHashportLink();
assert.equal(spaced, `${HASHPORT_BASE}&to_name=Cafe%20Expo&type=dynamic`, "spaces should be encoded as %20");
assert.equal(parseHashportLink(spaced).toName, "Cafe Expo", "%20 should decode back to a space");

// parseEip681 の異常系（タグ付きエラーをthrowする）
function assertThrowsKind(fn, expected, message) {
  let caught;
  let threw = false;
  try {
    fn();
  } catch (err) {
    threw = true;
    caught = err;
  }
  assert.equal(threw, true, `${message}: should throw`);
  for (const [key, value] of Object.entries(expected)) {
    assert.equal(caught[key], value, `${message}: err.${key}`);
  }
}
assertThrowsKind(
  () => parseEip681(`ethereum:${JPYC}@56/transfer?address=${TO}`),
  { kind: "UnsupportedChain", chain_id: "56" },
  "unsupported chain (BSC)"
);
assertThrowsKind(
  () => parseEip681(`ethereum:${JPYC}/transfer?address=${TO}`),
  { kind: "MissingParam", name: "chain_id" },
  "missing chain id"
);
assertThrowsKind(
  () => parseEip681(`ethereum:0xdAC17F958D2ee523a2206206994597C13D831ec7@1/transfer?address=${TO}`),
  { kind: "UnsupportedContract", address: "0xdAC17F958D2ee523a2206206994597C13D831ec7" },
  "unsupported contract"
);
assertThrowsKind(
  () => parseEip681(`ethereum:${JPYC}@137/transfer?address=${TO}&address=0x0000000000000000000000000000000000000001`),
  { kind: "DuplicateParam", name: "address" },
  "duplicate address"
);
assertThrowsKind(
  () => parseEip681(`ethereum:${JPYC}@137/transfer?address=${TO}&value=1`),
  { kind: "UnsupportedParam", name: "value" },
  "value param"
);
assertThrowsKind(
  () => parseEip681(`ethereum:${JPYC}@137/transfer?address=${TO}&uint256=-1`),
  { kind: "InvalidAmount", value: "-1" },
  "negative amount"
);
assertThrowsKind(() => parseEip681("not a uri"), { kind: "InvalidEip681" }, "garbage input");

// isSupportedEip681（throwしない述語関数）
assert.equal(isSupportedEip681(EIP681_URI), true, "isSupportedEip681 should accept a valid JPYC URI");
assert.equal(
  isSupportedEip681(`ethereum:${JPYC}@56/transfer?address=${TO}`),
  false,
  "isSupportedEip681 should reject an unsupported chain"
);
assert.equal(isSupportedEip681("not a uri"), false, "isSupportedEip681 should reject garbage input");

console.log("wasm smoke test passed");
