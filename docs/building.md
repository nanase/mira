# ビルドとリリース

## 必要なもの

- Rust（MSVC ツールチェーン）
- WebView2 ランタイム（Windows 10 / 11 には通常入っています）

画面は素の HTML、CSS、JavaScript で書いており、Node.js などのビルド工程はありません。

## ビルド

```sh
cd src-tauri
cargo build --release
```

`src-tauri/target/release/mira.exe` ができます。好きな場所に置いて起動してください。

## テスト

```sh
cd src-tauri
cargo test
cargo clippy --all-targets
```

デバッグビルドでは、起動中にもう一度 `mira.exe --debug-popover` を実行すると、最初のプロファイルのポップアップが開きます。トレイアイコンをクリックせずに画面を確かめるときに使います。

## アイコン

アプリのアイコンの元の絵は `src-tauri/icons/icon.svg` です。描き直したら、次を実行します。

```sh
cd src-tauri
cargo run --example gen_icons
```

`src-tauri/icons/` と、設定画面で使う `ui/icon.png` が作り直されます。

## README の画像

`docs/images/hero.png` は、実際の画面（`ui/`）に見本データを流し込んで描き出しています。画面やアイコンを変えたら、作り直してください。

```powershell
powershell -ExecutionPolicy Bypass -File docs/images/src/render.ps1
```

- 見本データは `docs/images/src/mock.js`、画像の構成は `docs/images/src/hero.html` にあります
- Microsoft Edge をヘッドレスモードで使います。画面には何も表示しません

## インストーラー

インストーラー（NSIS）を作るには、Tauri CLI を使います。

```sh
cargo install tauri-cli --version "^2" --locked
cd src-tauri
cargo tauri build
```

## リリースの手順

1. バージョンを上げます
   - `src-tauri/Cargo.toml` の `version`
   - `src-tauri/tauri.conf.json` の `version`
2. リリース日を更新します
   - `src-tauri/src/state.rs` の `RELEASE_DATE`（設定画面のサイドバーに出ます）
3. テストを通してから、配布用の Release ビルドを作ります
4. `mira.exe` だけを zip にまとめ、GitHub の Releases に載せます

配布用のビルドでは、上の「ビルド」の手順に 2 つ加えます。

- `--features tauri/custom-protocol` を付けます
  - 付けないと Tauri が開発モードでビルドされ、`src-tauri` のフルパスが exe に入ります
- `--remap-path-prefix` で、exe に入るソースのパスを置き換えます
  - 置き換えないと、ユーザー名を含むパスがパニック時のメッセージ用に残ります

```powershell
cd src-tauri
$env:RUSTFLAGS = "--remap-path-prefix=$env:USERPROFILE\.cargo=/cargo --remap-path-prefix=$env:USERPROFILE\.rustup=/rustup --remap-path-prefix=$((Resolve-Path ..).Path)=/mira"
cargo build --release --locked --features tauri/custom-protocol
Remove-Item Env:RUSTFLAGS
```
