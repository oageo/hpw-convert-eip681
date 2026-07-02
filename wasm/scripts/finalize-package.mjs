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
// 実行方法: node scripts/finalize-package.mjs   (`wasm/` ディレクトリから)
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const wasmDir = dirname(dirname(fileURLToPath(import.meta.url)));
const pkgDir = join(wasmDir, "pkg");
const pkgJsonPath = join(pkgDir, "package.json");

const pkgJson = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
pkgJson.name = "hpw-convert-eip681";
pkgJson.description =
  "Unofficial, unaffiliated converter: HashPort Wallet JPYC payment links -> EIP-681 URIs. Not endorsed by or affiliated with HashPort.";
pkgJson.license = "MIT";
pkgJson.repository = {
  type: "git",
  url: "git+https://github.com/oageo/hpw-convert-eip681.git",
};
pkgJson.files = [...(pkgJson.files ?? []), "LICENSE", "README.md"];

writeFileSync(pkgJsonPath, `${JSON.stringify(pkgJson, null, 2)}\n`);
copyFileSync(join(wasmDir, "..", "LICENSE"), join(pkgDir, "LICENSE"));
copyFileSync(join(wasmDir, "..", "README.md"), join(pkgDir, "README.md"));

console.log(`updated ${pkgJsonPath} and copied LICENSE/README.md into ${pkgDir}`);
