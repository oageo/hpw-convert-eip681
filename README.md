# hpw-convert-eip681
![Crates.io Version](https://img.shields.io/crates/v/hpw-convert-eip681)
![Crates.io Downloads (latest version)](https://img.shields.io/crates/dv/hpw-convert-eip681)
![NPM Version](https://img.shields.io/npm/v/hpw-convert-eip681)
![NPM Downloads](https://img.shields.io/npm/dy/hpw-convert-eip681)
![GitHub License](https://img.shields.io/github/license/oageo/hpw-convert-eip681)

日本におけるとある中央集権的なステーブルコイン決済QRコード（HashPort Wallet形式の決済リンク）のURLと、EIP-681（ERC-681）形式の支払いURIとを**相互に変換する**[Rust製ライブラリ](https://crates.io/crates/hpw-convert-eip681)です。[WebAssembly版（npm）](https://www.npmjs.com/package/hpw-convert-eip681)も同梱しています。

- **HashPort Wallet決済リンク → EIP-681**: 決済リンクを解析し、EIP-681形式の支払いURIや送金先アドレス・金額に変換します。
- **EIP-681 → HashPort Wallet決済リンク**: EIP-681形式のJPYC送金URIを解析し、HashPort Wallet形式の決済リンクを生成します（[後述](#逆方向-eip-681--hashport-wallet決済リンク)の注意事項を必ずお読みください）。

## 免責事項 / 非公式ツールについて

**本プロジェクトは個人による無保証の非公式（unofficial）ツールです。** 

本ライブラリの目的は、あくまで相互運用性（interoperability）の確保です。

実装は純粋なクライアントサイドの文字列・URL解析のみを行い、**外部のサーバーへ一切ネットワークリクエストを送信しません。** 入力として受け取ったURL・URI文字列をその場でパースし、もう一方の形式の文字列やアドレス・金額に変換して返すだけです。

## 対応範囲

現時点では以下のみに対応しています（他のチェーン・通貨は今後の対応も未定です）。両方向の変換で対応範囲は同じです:

- 通貨: **JPYC** のみ
- チェーン: 下表の4チェーン

HashPort Walletの `master_currency_id` は通貨単体ではなく「通貨×チェーン」の組ごとに振られているため、この値からチェーンも決定します（逆方向ではEIP-681のchain idから `master_currency_id` を決定します）。

| `master_currency_id` | 通貨 | チェーン | chain id |
| --- | --- | --- | --- |
| `487` | JPYC | Polygon | `137` |
| `489` | JPYC | Avalanche C-Chain | `43114` |
| `490` | JPYC | Ethereum | `1` |
| `712` | JPYC | Kaia | `8217` |

JPYCのコントラクトアドレスは全チェーン共通（`0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29`）のため、EIP-681側でチェーンごとに変わるのは `@<chain id>` の部分のみです。

対応外のURL・URI・チェーン・通貨は `ParseError` （もしくはJS側のタグ付きエラー）として明示的に拒否されます。

## 対象とするURL形式

```
https://link.expo2025-wallet.com/pay?to=<address>&master_currency_id=<id>&amount=<hex>&to_name=<label>&type=<type>
```

これをEIP-681形式に変換します:

```
ethereum:<JPYCコントラクトアドレス>@<chain id>/transfer?address=<to>&uint256=<amount>
```

`amount` は元URLに存在しないことがあります。その場合 `amount` は
`None`（JS側では `undefined`）として扱われ、EIP-681出力からも
`uint256=` パラメータ自体が省かれます（`ethereum:<contract>@<chain id>/transfer?address=<to>`）。これはEIP-681仕様が金額未指定のリクエストを
許容している（受け取り側のウォレットがユーザーに入力させる）ことに
対応したものです。

`type`（例: `dynamic`）は `to_name` と同様、解釈や検証を加えず生の
文字列のまま `link_type`（JS側では `linkType`）として保持します。
取りうる値の全体像が未確認のため、真偽値等には決め打ちしていません。
パラメータが存在しない場合は `None`（JS側では `undefined`）です。

## 逆方向: EIP-681 → HashPort Wallet決済リンク

> **注意:** 生成した決済リンクが意味を持つのは、**送金先アドレスがHashPort Wallet側で受け取り先として設定済みである場合に限られます。** 本ライブラリはネットワークアクセスを一切行わないため、そのアドレスがHashPort Walletに登録されているかどうかを確認できません。生成されるのはあくまで「HashPort Walletが生成するリンクと同じ形式の文字列」であり、HashPort Wallet側がそのリンクを受け付けることや、支払いが正しく行われることは保証されません。

### 受け付ける入力

```
ethereum:[pay-]<JPYCコントラクトアドレス>@<chain id>/transfer?address=<to>[&uint256=<amount>]
```

- `uint256` は省略可能です（省略時は金額未指定のリンクになります）。EIP-681の数値表記に従い、`1000` のような整数のほか `2.014e18` のような指数表記も受け付けます（結果が非負の整数かつ `uint256` に収まる場合のみ）。
- ガス関連のヒント（`gas`・`gasLimit`・`gasPrice`）は、支払い内容を変えないため受け付けたうえで無視します。
- EIP-681のURIはHashPortリンクの `to_name`・`type` に相当する情報を持たないため、これらは別途指定します（Rustでは `ParsedLink` の `to_name`・`link_type` フィールドを設定、JSでは `parseEip681` の第2・第3引数）。

### 拒否される入力

順方向（HashPort Wallet決済リンク → EIP-681）のパースは寛容（lenient）ですが、逆方向は生成されるリンクが送金先・金額・チェーンを決めるものであるため、**解釈が割れうる入力はすべてエラーにする厳格（strict）な方針**を取っています。

| 入力 | エラー | 理由 |
| --- | --- | --- |
| chain id の省略（`ethereum:<contract>/transfer?...`） | `MissingParam` | EIP-681では「ウォレットの現在のネットワーク」を意味するが、メインネット等に決め打ちすると別チェーンで支払わせる危険があるため |
| 非対応のchain id、先頭ゼロ付きのchain id（`@0137`） | `UnsupportedChain` / `InvalidEip681` | 対応表にないチェーンは変換できない。先頭ゼロは8進と誤読されうるため |
| JPYC以外のコントラクト | `UnsupportedContract` | 非対応の通貨 |
| `transfer` 以外の関数（`approve` 等）、関数呼び出しなし | `UnsupportedFunction` | ERC-20の送金以外は表現できない |
| `address=` の欠落 | `MissingParam`（クエリが `?` のみで空の場合は `InvalidEip681`） | 受取人は `address=` でのみ指定する（`to=` 等の他のキーは下記の未知のパラメータとして扱う） |
| 同じパラメータの重複（`address=...&address=...`） | `DuplicateParam` | 先勝ち・後勝ちがウォレットごとに異なり、曖昧さが攻撃面になるため（順方向は後勝ち） |
| `value=` やその他の未知のパラメータ | `UnsupportedParam` | `value` はネイティブ通貨の同時送金を意味し、黙って捨てると支払いの意味が変わるため |
| ENS名（`example.eth`）、`0x` のないアドレス | `InvalidAddress` | ENSの名前解決にはネットワークアクセスが必要なため |
| 大文字小文字が混在しているのにEIP-55チェックサムが合わないアドレス | `InvalidAddress` | 打ち間違いの兆候であるため（すべて小文字・すべて大文字は受け付ける） |
| 負数、整数にならない値（`1.5`）、`uint256` に収まらない値、16進（`0x10`）、空 | `InvalidAmount` | 金額として一意に解釈できないため |
| 前後の空白・制御文字・非ASCII文字、フラグメント（`#`）、不正なパーセントエンコード | `InvalidEip681` | コピペ由来の混入物やホモグリフ等を黙って受け入れないため |

### 出力形式

```
https://link.expo2025-wallet.com/pay?to=<address>&master_currency_id=<id>&amount=<hex>&to_name=<label>&type=<type>
```

- パラメータの順序・表記はHashPort Walletが実際に生成するリンクに合わせています。
- `to` はEIP-55チェックサム表記です。
- `amount` は `0x` + 小文字16進64桁のゼロ埋めです（例: 100 JPYC = `0x0000000000000000000000000000000000000000000000056bc75e2d63100000`）。
- `to_name`・`type` はRFC 3986の非予約文字（`A-Z a-z 0-9 - . _ ~`）以外をすべてパーセントエンコードします。空白は `+` ではなく必ず `%20` になります。`&` や `=` も必ずエンコードされるため、`to_name` の内容が他のパラメータを注入・改変することはありません。
- `amount`・`to_name`・`type` が `None`（JS側では未指定）の場合はパラメータ自体を省略し、空文字の場合は `to_name=` のように値が空のパラメータとして出力します。

## インストール

### Rust (`core`)

```
cargo add hpw-convert-eip681
```

### npm (`wasm`)

```
npm install hpw-convert-eip681
```

## 使い方

### Rust

#### HashPort Wallet決済リンク → EIP-681

```rust
use hpw_convert_eip681::{is_supported, parse};

fn main() {
    let url = "https://link.expo2025-wallet.com/pay?to=0x1234567890123456789012345678901234567890&master_currency_id=487&amount=0x3e8&to_name=Coffee%20Shop";

    // 対応しているURLかどうかを事前に確認する
    if !is_supported(url) {
        eprintln!("unsupported link");
        return;
    }

    // パースする
    let link = parse(url).expect("valid HashPort link");

    // EIP-681形式のURIを取得する
    println!("{}", link.to_eip681());
    // => ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?address=0x1234567890123456789012345678901234567890&uint256=1000

    // アドレスと金額を個別に取得する（amountは元URLに無いこともあるため Option<U256>）
    println!("to: {}", link.address());
    match link.amount() {
        Some(amount) => println!("amount: {amount}"),
        None => println!("amount: (unspecified)"),
    }

    // まとめて取得することもできる
    let addr_amount = link.address_amount();
    println!("{} / {:?}", addr_amount.address, addr_amount.amount);

    // to_name / link_type は生のまま（None は「パラメータ自体が無い」）
    println!("to_name: {:?}, link_type: {:?}", link.to_name, link.link_type);
}
```

#### EIP-681 → HashPort Wallet決済リンク

```rust
use hpw_convert_eip681::{parse_eip681, ParseError};

fn main() -> Result<(), ParseError> {
    // 100 JPYC（= 100 × 10^18 = 1e20）をPolygon上で送金するURI
    let uri = "ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?address=0x1234567890123456789012345678901234567890&uint256=1e20";

    // パースする（to_name / link_type は常に None）
    let mut link = parse_eip681(uri)?;

    // 必要であれば表示ラベル等を設定する
    link.to_name = Some("Coffee Shop".to_string());

    // HashPort Wallet決済リンクを生成する
    println!("{}", link.to_hashport_link()?);
    // => https://link.expo2025-wallet.com/pay?to=0x1234567890123456789012345678901234567890&master_currency_id=487&amount=0x0000000000000000000000000000000000000000000000056bc75e2d63100000&to_name=Coffee%20Shop

    Ok(())
}
```

`ParsedLink` は両方向の変換で共通の中間表現なので、`parse` の結果から `to_hashport_link()` を、`parse_eip681` の結果から `to_eip681()` を呼ぶこともできます。

### JavaScript / TypeScript

#### HashPort Wallet決済リンク → EIP-681

```ts
import { isSupported, parseHashportLink } from "hpw-convert-eip681";

const url =
  "https://link.expo2025-wallet.com/pay?to=0x1234567890123456789012345678901234567890&master_currency_id=487&amount=0x3e8&to_name=Coffee%20Shop";

// 対応しているURLかどうかを事前に確認する
if (!isSupported(url)) {
  throw new Error("unsupported link");
}

// パースする（失敗時は { kind: "...", ... } 形式のエラーをthrow）
try {
  const link = parseHashportLink(url);

  // EIP-681形式のURIを取得する
  console.log(link.toEip681());
  // => ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?address=0x1234567890123456789012345678901234567890&uint256=1000

  // アドレスと金額を個別に取得する（amountは元URLに無い場合 undefined になる）
  console.log(link.to, link.amount, link.amountHex);

  // その他のフィールド（toName/linkType は元URLに無い場合 undefined）
  console.log(link.chainId, link.currencySymbol, link.currencyContract, link.toName, link.linkType);
} catch (err) {
  console.error(err.kind, err);
}
```

#### EIP-681 → HashPort Wallet決済リンク

```ts
import { isSupportedEip681, parseEip681 } from "hpw-convert-eip681";

// 100 JPYC（= 100 × 10^18 = 1e20）をPolygon上で送金するURI
const uri =
  "ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?address=0x1234567890123456789012345678901234567890&uint256=1e20";

// 対応しているURIかどうかを事前に確認する
if (!isSupportedEip681(uri)) {
  throw new Error("unsupported URI");
}

try {
  // 第2・第3引数で to_name / type を指定できる（省略時はパラメータ自体を出力しない）
  const link = parseEip681(uri, "Coffee Shop");

  // HashPort Wallet決済リンクを生成する（失敗時は { kind: "...", ... } 形式のエラーをthrow）
  console.log(link.toHashportLink());
  // => https://link.expo2025-wallet.com/pay?to=0x1234567890123456789012345678901234567890&master_currency_id=487&amount=0x0000000000000000000000000000000000000000000000056bc75e2d63100000&to_name=Coffee%20Shop
} catch (err) {
  console.error(err.kind, err);
}
```

## エラー

失敗時はRustでは `ParseError` が返り、JSでは `{ kind: "<バリアント名>", ... }` 形式の値がthrowされます。TypeScriptでは、このthrowされる値の型を `HpwParseError` としてインポートできます。

```ts
import { parseEip681, type HpwParseError } from "hpw-convert-eip681";

try {
  parseEip681(uri);
} catch (err) {
  const e = err as HpwParseError;
  if (e.kind === "UnsupportedChain") console.error(e.chain_id);
}
```

| バリアント | 主な発生箇所 |
| --- | --- |
| `InvalidUrl` / `UnsupportedHost` | 順方向: URLとして不正、ホストが `link.expo2025-wallet.com` でない |
| `UnsupportedCurrency` | 順方向: 非対応の `master_currency_id` |
| `MissingParam` | 必須パラメータ（順方向の `to`・`master_currency_id`、逆方向の chain id・`address`）がない |
| `InvalidAddress` / `InvalidAmount` | 両方向: アドレス・金額が不正 |
| `InvalidEip681` | 逆方向: EIP-681 URIとして構文的に不正 |
| `UnsupportedContract` / `UnsupportedChain` / `UnsupportedFunction` / `UnsupportedParam` / `DuplicateParam` | 逆方向: [拒否される入力](#拒否される入力)を参照（`UnsupportedChain` は `to_hashport_link()` でも発生しうる） |

`ParseError` は `#[non_exhaustive]` です。今後バリアントが追加されても互換性を壊さないよう、Rustで `match` する場合はワイルドカードアーム（`_ => ...`）が必要です。

## チェックサム検証（オプション）

`parse` / `parseHashportLink` はアドレスの大文字・小文字を厳密にはチェックしない、寛容な（lenient）パースを行います。これとは別に、EIP-55チェックサムを厳密に検証したい場合のためのオプトインAPIとして `validate_checksum` / `isValidChecksum` を用意しています（`parse_eip681` / `parseEip681` は、大文字小文字が混在するアドレスに対しては常にチェックサムを検証します）。

```rust
use hpw_convert_eip681::validate_checksum;

match validate_checksum("0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29") {
    Ok(address) => println!("valid checksum: {address}"),
    Err(e) => eprintln!("invalid checksum: {e}"),
}
```

```ts
import { isValidChecksum } from "hpw-convert-eip681";

isValidChecksum("0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29"); // => true / false
```

## ライセンス

[MIT License](./LICENSE)
