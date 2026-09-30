# ビルドとリリース

## 必要なもの

- Rust（MSVC ツールチェーン）
- WebView2 ランタイム（Windows 10 / 11 には通常入っている）

画面は素の HTML、CSS、JavaScript で書いており、Node.js などのビルド工程はない。

## ビルド

```sh
cd src-tauri
cargo build --release
```

`src-tauri/target/release/mira.exe` ができる。好きな場所に置いて起動する。

## テスト

```sh
cd src-tauri
cargo test
cargo clippy --all-targets
```

デバッグビルドでは、起動中にもう一度 `mira.exe --debug-popover` を実行すると、最初のプロファイルのポップアップが開く。トレイアイコンをクリックせずに画面を確かめるときに使う。

## アイコン

アプリのアイコンの元の絵は `src-tauri/icons/icon.svg` である。描き直したら次を実行する。

```sh
cd src-tauri
cargo run --example gen_icons
```

`src-tauri/icons/` と、設定画面で使う `ui/icon.png` が作り直される。

## README の画像

`docs/images/hero.png` は、実際の画面（`ui/`）に見本データを流し込んで描き出している。画面やアイコンを変えたら作り直す。

```powershell
powershell -ExecutionPolicy Bypass -File docs/images/src/render.ps1
```

- 見本データは `docs/images/src/mock.js`、画像の構成は `docs/images/src/hero.html` にある
- Microsoft Edge をヘッドレスモードで使う。画面には何も表示しない

## インストーラー

インストーラー（NSIS）を作るには Tauri CLI を使う。

```sh
cargo install tauri-cli --version "^2" --locked
cd src-tauri
cargo tauri build
```

## リリースの手順

1. バージョンを上げる
   - `src-tauri/Cargo.toml` の `version`
   - `src-tauri/tauri.conf.json` の `version`
2. リリース日を更新する
   - `src-tauri/src/state.rs` の `RELEASE_DATE`（設定画面のサイドバーに出る）
3. テストを通してから、Release ビルドを作る
