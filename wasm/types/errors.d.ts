// wasm-bindgenが自動生成する`.d.ts`は関数のOk/戻り値側のみを反映し、
// 「この関数はこの形の値をthrowする」ということを表現できない。
// `parseHashportLink`（およびこのクレート内でthrowしうる他のexport）は
// 失敗時にこのタグ付きユニオン型をした素のJS値をthrowする。`.kind`で
// 分岐したい箇所では、この型を手動でインポートすること。
export type HpwParseError =
  | { kind: "InvalidUrl"; message: string }
  | { kind: "UnsupportedHost"; host: string }
  | { kind: "MissingParam"; name: string }
  | { kind: "UnsupportedCurrency"; id: string }
  | { kind: "InvalidAddress"; value: string }
  | { kind: "InvalidAmount"; value: string; reason: string };
