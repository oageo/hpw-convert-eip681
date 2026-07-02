# hpw-convert-eip681

日本におけるとある中央集権的なステーブルコイン決済QRコード（HashPort Wallet形式の決済リンク）のURLを解析し、EIP-681（ERC-681）形式の支払いURIや送金先アドレス・金額に変換するRust製ライブラリです。WebAssembly版（npm）も同梱しています。

## 免責事項 / 非公式ツールについて

**本プロジェクトはHashPort株式会社とは無関係の非公式（unofficial）ツールです。** HashPort社による承認・提携・保証を受けたものではなく、HashPortの商標・サービスに対するいかなる権利も主張するものではありません。

本ライブラリの目的は、**HashPort Walletアプリを持っていない人が、画面に表示されたHashPort形式の決済QRコード／リンクを、別のウォレットで支払えるようにする**ための相互運用性（interoperability）の確保です。

実装は純粋なクライアントサイドの文字列・URL解析のみを行い、**HashPortのサーバーへ一切ネットワークリクエストを送信しません。** 入力として受け取ったURL文字列をその場でパースし、EIP-681形式の文字列やアドレス・金額に変換して返すだけです。

## 対応範囲

現時点では以下のみに対応しています（他のチェーン・通貨は今後の対応も未定です）:

- チェーン: **Polygon (chain id `137`)** のみ
- 通貨: **JPYC** (`master_currency_id=487`) のみ

対応外のURL・チェーン・通貨は `ParseError` （もしくはJS側のタグ付きエラー）として明示的に拒否されます。

## 対象とするURL形式

```
https://link.expo2025-wallet.com/pay?to=<address>&master_currency_id=487&amount=<hex>&to_name=<label>
```

これをEIP-681形式に変換します:

```
ethereum:<JPYCコントラクトアドレス>@137/transfer?address=<to>&uint256=<amount>
```

`amount` は元URLに存在しないことがあります。その場合 `amount` は
`None`（JS側では `undefined`）として扱われ、EIP-681出力からも
`uint256=` パラメータ自体が省かれます（`ethereum:<contract>@137/transfer?address=<to>`）。これはEIP-681仕様が金額未指定のリクエストを
許容している（受け取り側のウォレットがユーザーに入力させる）ことに
対応したものです。

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
    // => ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?address=0x1234...&uint256=1000

    // アドレスと金額を個別に取得する（amountは元URLに無いこともあるため Option<U256>）
    println!("to: {}", link.address());
    match link.amount() {
        Some(amount) => println!("amount: {amount}"),
        None => println!("amount: (unspecified)"),
    }

    // まとめて取得することもできる
    let addr_amount = link.address_amount();
    println!("{} / {:?}", addr_amount.address, addr_amount.amount);
}
```

### JavaScript / TypeScript

```ts
import { isSupported, parseHashportLink, isValidChecksum } from "hpw-convert-eip681";

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
  // => ethereum:0xE7C3D8C9a439feDe00D2600032D5dB0Be71C3c29@137/transfer?address=0x1234...&uint256=1000

  // アドレスと金額を個別に取得する（amountは元URLに無い場合 undefined になる）
  console.log(link.to, link.amount, link.amountHex);

  // その他のフィールド
  console.log(link.chainId, link.currencySymbol, link.currencyContract, link.toName);
} catch (err) {
  console.error(err.kind, err);
}
```

## チェックサム検証（オプション）

`parse` / `parseHashportLink` はアドレスの大文字・小文字を厳密にはチェックしない、寛容な（lenient）パースを行います。これとは別に、EIP-55チェックサムを厳密に検証したい場合のためのオプトインAPIとして `validate_checksum` / `isValidChecksum` を用意しています。

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
