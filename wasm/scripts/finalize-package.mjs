// `wasm-pack build wasm --target bundler --out-dir pkg --out-name
// hpw_convert_eip681` の後処理スクリプト。
//
// RustクレートはCargoワークスペース内で `core`（`hpw-convert-eip681`）と
// 名前が衝突しないよう `hpw-convert-eip681-wasm` という名前にしているが、
// npmパッケージとしては `hpw-convert-eip681` として公開したいため、
// wasm-pack が自動生成した `pkg/package.json` のnameフィールドと
// descriptionを書き換える。また、wasm-packは `license = "MIT"`
// （SPDX識別子）からはLICENSEファイルをコピーせず、READMEも
// `wasm/README.md`（クレート直下）にしか対応していないため、
// リポジトリルートのLICENSE/README.mdをどちらも `pkg/` に同梱する
// （npmのパッケージページにREADMEを表示させるため）。
//
// さらに、throwされるエラーの型 `HpwParseError`（`wasm/types/errors.d.ts`）
// はwasm-bindgenの生成する `.d.ts` では表現できないため、`pkg/errors.d.ts`
// として同梱し、生成された型定義の末尾に型のみの再エクスポートを追記する。
// これにより利用者は `import type { HpwParseError } from "hpw-convert-eip681"`
// と書ける。型のみの追記なので、実行時のJS/wasmには一切影響しない。
//
// 実行方法: node scripts/finalize-package.mjs   (`wasm/` ディレクトリから)
import { appendFileSync, copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const wasmDir = dirname(dirname(fileURLToPath(import.meta.url)));
const pkgDir = join(wasmDir, "pkg");
const pkgJsonPath = join(pkgDir, "package.json");

const pkgJson = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
pkgJson.name = "hpw-convert-eip681";
pkgJson.description =
  "Unofficial, unaffiliated bidirectional converter: HashPort Wallet JPYC payment links <-> EIP-681 URIs. Not endorsed by or affiliated with HashPort.";
pkgJson.license = "MIT";
pkgJson.repository = {
  type: "git",
  url: "git+https://github.com/oageo/hpw-convert-eip681.git",
};
// Setで重複排除し、スクリプトを2回実行しても files が重複しないようにする
pkgJson.files = [
  ...new Set([...(pkgJson.files ?? []), "LICENSE", "README.md", "errors.d.ts"]),
];

writeFileSync(pkgJsonPath, `${JSON.stringify(pkgJson, null, 2)}\n`);
copyFileSync(join(wasmDir, "..", "LICENSE"), join(pkgDir, "LICENSE"));
copyFileSync(join(wasmDir, "..", "README.md"), join(pkgDir, "README.md"));
copyFileSync(join(wasmDir, "types", "errors.d.ts"), join(pkgDir, "errors.d.ts"));

// パッケージは `"type": "module"` なので、TypeScriptの `nodenext` 解決でも
// 通るよう拡張子付き（`./errors.js` → `errors.d.ts` に解決される）で書く。
// 既に追記済みなら何もしない（2回実行しても重複させないため）。
const typesPath = join(pkgDir, pkgJson.types);
const reexport = 'export type { HpwParseError } from "./errors.js";';
if (!readFileSync(typesPath, "utf8").includes(reexport)) {
  appendFileSync(typesPath, `\n${reexport}\n`);
}

console.log(
  `updated ${pkgJsonPath}, copied LICENSE/README.md/errors.d.ts and re-exported HpwParseError in ${pkgDir}`,
);
