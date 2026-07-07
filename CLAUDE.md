# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`hpw-convert-eip681` is an **unofficial, unaffiliated** parser that converts HashPort Wallet payment links into EIP-681 (ERC-681) payment URIs, or their individual components (address, amount). It performs pure client-side string/URL parsing only — it never sends any network request to HashPort's servers. Currently supports only Polygon (chain id 137) and JPYC (`master_currency_id=487`); other chains/currencies are explicitly rejected, not silently ignored.

Target URL format:
```
https://link.expo2025-wallet.com/pay?to=<address>&master_currency_id=487&amount=<hex>&to_name=<label>
```

Two-crate Cargo workspace:
- `core/` — pure Rust, published to crates.io as `hpw-convert-eip681`.
- `wasm/` — wasm-bindgen wrapper around `core` (path dependency), built with `wasm-pack` and published to npm as `hpw-convert-eip681` (the crate itself is named `hpw-convert-eip681-wasm` to avoid a name collision with `core` inside the workspace, and is renamed post-build — see below). Not published to crates.io (`publish = false`).

## Commands

All commands run from the repo root unless noted.

**Build / lint / test (core + wasm, native target):**
```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Run a single test: `cargo test -p hpw-convert-eip681 <test_name>`. Most behavior is covered by integration tests in `core/tests/parse.rs` (not unit tests in `src/`).

If `serde` feature-gated code needs checking: `cargo clippy -p hpw-convert-eip681 --all-targets --features serde -- -D warnings` (the `wasm` crate always builds `core` with `features = ["serde"]`, so this is what actually gets compiled for npm).

**wasm build (requires `wasm-pack`; install via `cargo install wasm-pack --locked`, not the `jetli/wasm-pack-action` marketplace action — that action's version resolution has been observed to install a stale `wasm-pack` build that fails to parse Cargo.toml workspace-inheritance syntax like `license.workspace = true`):**
```
wasm-pack build wasm --target nodejs --out-dir pkg-node --out-name hpw_convert_eip681
node wasm/tests/smoke.mjs
```
```
wasm-pack build wasm --target bundler --out-dir pkg --out-name hpw_convert_eip681
node wasm/scripts/finalize-package.mjs
```
**Gotcha:** the first positional argument to `wasm-pack build` (`wasm`) is the crate path, and `--out-dir` is resolved *relative to that crate path*, not the current working directory. Running `wasm-pack build wasm --out-dir wasm/pkg` from the repo root double-nests the output into `wasm/wasm/pkg`. When invoking from the repo root, always use `--out-dir pkg` (not `wasm/pkg`).

`wasm/scripts/finalize-package.mjs` is a required post-build step, not optional polish: `wasm-pack` emits `pkg/package.json` with the Rust crate name (`hpw-convert-eip681-wasm`); the script renames it to `hpw-convert-eip681`, sets description/license/repository, and copies the repo-root `LICENSE` and `README.md` into `pkg/` (neither crates.io-style `license.workspace` nor wasm-pack's own README auto-detection reaches files outside the crate directory, so this has to be done by hand). It's idempotent (dedupes `package.json`'s `files` array via `Set`), so re-running it is safe.

## Architecture

### Parsing pipeline (`core/src/`)

`parse::parse()` is the single entry point; `is_supported()` is a thin `.is_ok()` wrapper over it. Pipeline order, each step returning on first error: URL parse → exact host match (`constants::EXPECTED_HOST`, case-insensitive via `url` crate's own normalization, no suffix matching — rejects spoofing like `link.expo2025-wallet.com.evil.example`) → single-pass query extraction → currency check (`Currency::from_raw_id`) → address parse (lenient, no checksum enforced) → amount decode (hex, `0x`-prefix optional) → construct `ParsedLink`.

Module split mirrors this pipeline and the `ParseError` variant families: `constants.rs` (all magic values — JPYC contract address, chain id, host, currency id — centralized here rather than scattered, mirroring the author's other project `jpyc-payment-qr`'s `constants.ts` convention), `error.rs`, `currency.rs`, `amount.rs`, `address.rs`, `link.rs` (`ParsedLink`/`AddressAmount`/`ChainId` output types), `parse.rs` (the pipeline itself).

Non-obvious invariants worth knowing before touching this code:
- **`amount` is `Option<U256>`, not `U256`.** The source URL sometimes omits the `amount` param entirely; that's `None`, not an error. `to_eip681()` omits `&uint256=` from the output URI when absent (EIP-681 allows amount-unspecified transfer requests). A present-but-empty `amount=` *is* an error (`InvalidAmount`) — don't conflate the two.
- **Hex decoding must go through `strip_prefix("0x"/"0X")` + `U256::from_str_radix(_, 16)`, never bare `U256::from_str`.** The bare `FromStr` (backed by `ruint`) auto-detects radix and defaults to decimal for unprefixed input, so an unprefixed all-digit amount like `"1000"` would silently be misparsed as decimal 1000 instead of hex 0x1000 — a value-corruption bug, not a caught error.
- **The empty-string check in `amount.rs` is load-bearing, not cosmetic.** `ruint`'s `from_str_radix` returns `Ok(0)` for an empty string rather than erroring, so without the explicit check, `amount=` or `amount=0x` would silently parse as amount zero.
- **`Currency::from_raw_id` matches the raw `&str`, not a parsed `u32`.** This means a non-numeric `master_currency_id` value falls into the same `UnsupportedCurrency` path as an out-of-range numeric one, without a third error variant.
- **`validate_checksum` (EIP-55) is strict, not "uniform case is unambiguous."** `alloy_primitives::Address::parse_checksummed` rejects all-lowercase/all-uppercase input even though it's unambiguous about which address it names — it requires exact mixed-case checksum match. The lenient `parse_address` used by the main pipeline, by contrast, accepts any casing and even a missing `0x` prefix.
- **Duplicate query params are last-wins** (`to=X&to=Y` → `Y`), intentionally not an error — documented inline in `parse.rs` rather than enforced.
- **`ParseError` derives `Serialize` but deliberately not `Deserialize`.** The `MissingParam { name: &'static str }` variant would make `Deserialize` a runtime trap (most deserializers can't hand out `'static` borrowed data), and nothing in the workspace deserializes errors — only serializes them, to throw to JS.
- Amount is `U256` (via `alloy-primitives`), never `u128` — a JPY amount already scaled by 10^18 can exceed u128 range for large-but-legitimate values.

`alloy-primitives` (not `primitive-types`/`ethnum`) was chosen specifically because its `Address` type has EIP-55 checksum generation/validation built in, avoiding a hand-rolled Keccak-256 implementation or an extra `sha3` dependency.

### wasm boundary (`wasm/src/lib.rs`)

Three deliberate JS-interop decisions, easy to accidentally regress:
- **Errors throw as plain tagged JS values, not returned Result-likes.** `serde_wasm_bindgen::to_value(&ParseError)` produces `{"kind": "...", ...}` (internally tagged via `#[serde(tag = "kind")]` on the `core` side); documented separately in `wasm/types/errors.d.ts` since wasm-bindgen's generated `.d.ts` can't express "this throws a value of shape X."
- **`U256` amounts are always decimal strings on the JS side, never numbers or raw BigInt.** `ParsedLink` (the wasm-facing struct) is hand-written with an explicit `From<core::ParsedLink>` conversion rather than derived via `serde-wasm-bindgen`, specifically because `alloy_primitives`'s own `Serialize` impl for `U256`/`Address` emits *hex* (JSON-RPC "quantity" style) — pushing `core::ParsedLink` through `serde-wasm-bindgen` directly would silently produce hex amounts in JS.
- **`chain_id` conversion to `u32` uses `u32::try_from(..).expect(...)`, not `as u32`.** Chain ids can exceed u32 range in general; an `as` cast would silently truncate if a future chain were added. Currently unreachable (only Polygon/137 exists) but deliberately fails loudly rather than truncating.

### Release/publish workflows (`.github/workflows/`)

`ci.yml` (PRs to `main` + pushes to `main`): fmt → clippy → test → wasm-pack nodejs build → smoke test. `release.yml` (triggered on GitHub Release publish): two parallel jobs, `publish-crate` (crates.io) and `publish-npm` (npm), each re-running fmt/clippy/test as a gate before publishing, then verifying the release tag (`vX.Y.Z`) matches the workspace version before publishing.

- **crates.io publishing uses Trusted Publishing (OIDC)** via `rust-lang/crates-io-auth-action@v1` — no long-lived `CARGO_REGISTRY_TOKEN` secret. This has a bootstrap constraint: a brand-new crate cannot use Trusted Publishing for its *first* release (crates.io requires the crate to already exist to register a trusted publisher), so the very first `cargo publish` for any new crate in this repo must be done manually.
- **npm publishing uses a classic `NPM_TOKEN` secret** (no equivalent bootstrap restriction) plus `--provenance`.
- **Both crates.io and npm have immutable published versions** — there is no way to fix a wrong README/metadata on an already-published version; the fix is always a new version bump (`[workspace.package].version` in the root `Cargo.toml`, inherited by both `core` and `wasm` via `version.workspace = true`), not a re-upload.

### Conventions specific to this repo

- **All in-code comments are written in Japanese**, per the author's preference — match this when editing, including doc comments (`///`, `//!`).
- The JPYC Polygon contract address (`constants::JPYC_POLYGON_ADDRESS`) is a compile-time-checksum-validated constant (`address!` macro) — if this ever needs to change, re-verify against an authoritative source (not memory), since this project treats "don't guess contract addresses" as a hard rule.
