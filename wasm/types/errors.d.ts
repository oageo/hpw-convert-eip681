// wasm-bindgenが自動生成する`.d.ts`は関数のOk/戻り値側のみを反映し、
// 「この関数はこの形の値をthrowする」ということを表現できない。
// `parseHashportLink`・`parseEip681`・`ParsedLink.prototype.toHashportLink`
// （およびこのクレート内でthrowしうる他のexport）は失敗時にこのタグ付き
// ユニオン型をした素のJS値をthrowする。`.kind`で分岐したい箇所では、
// この型を `import type { HpwParseError } from "hpw-convert-eip681"` で
// インポートすること（npmパッケージでは `scripts/finalize-package.mjs` が
// このファイルを `errors.d.ts` として同梱し、メインの型定義から
// 再エクスポートしている）。
export type HpwParseError =
  | { kind: "InvalidUrl"; message: string }
  | { kind: "UnsupportedHost"; host: string }
  // EIP-681入力でチェーンIDがない場合は `name: "chain_id"`、
  // 受取人がない場合は `name: "address"` になる。
  | { kind: "MissingParam"; name: string }
  | { kind: "UnsupportedCurrency"; id: string }
  | { kind: "InvalidAddress"; value: string }
  | { kind: "InvalidAmount"; value: string; reason: string }
  // EIP-681 URIとして構文的に不正。`reason` は人間向けの説明。
  | { kind: "InvalidEip681"; reason: string }
  // 非対応のトークンコントラクト。`address` は入力中の生の文字列。
  | { kind: "UnsupportedContract"; address: string }
  // 非対応のチェーンID。`number` の精度を超えうるため10進文字列。
  | { kind: "UnsupportedChain"; chain_id: string }
  // `transfer` 以外の関数。関数呼び出しがない（ネイティブ通貨送金）場合は空文字。
  | { kind: "UnsupportedFunction"; name: string }
  // 意味を黙って捨てることになるため受け付けないパラメータ（例: `value`）。
  | { kind: "UnsupportedParam"; name: string }
  // EIP-681入力での同名パラメータの重複。
  | { kind: "DuplicateParam"; name: string };
