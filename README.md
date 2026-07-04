# madore

ブラウザで Markdown を表示するローカルビューア（Rust 製）。
ローカルサーバーを立ててブラウザで開き、**引数なしでもカレントディレクトリをルートにして、サイドバーへ最初からファイルツリーを表示する**。

```sh
madore              # カレントディレクトリを開く
madore ./docs       # ディレクトリを指定
```

サーバーはバックグラウンドに常駐し、コマンドはすぐ戻る。ブラウザには常時ファイルツリーが出て、保存すると自動で表示が更新される。

## 特徴

- サイドバーに Markdown ファイルツリーを常時表示、クリックで本文を表示
- ライブリロード（ファイル保存を検知して自動更新、SSE）
- GitHub Flavored Markdown（テーブル / タスクリスト / 脚注 / GitHub Alerts）
- シンタックスハイライト（highlight.js）・数式（KaTeX）・Mermaid 図
- 静的アセットをバイナリに埋め込み、単体で配布可能
- ダーク / ライトはブラウザ設定に追従
- サーバーはバックグラウンド常駐、コマンドは即終了（同じルートの2回目以降は既存サーバーを再利用）

## インストール

### 1. インストールスクリプト（Linux / macOS・Rust 不要）

```sh
curl -fsSL https://raw.githubusercontent.com/ibuibu/madore/main/install.sh | sh
```

GitHub Releases からお使いの OS 向けバイナリを取得し、`~/.local/bin`（`$MADORE_BIN_DIR` で変更可）に配置する。`~/.local/bin` が PATH に無ければ追加すること。

### 2. プリビルドバイナリを手動で

[Releases](https://github.com/ibuibu/madore/releases) から各 OS 向けのアーカイブをダウンロードし、`madore`（Windows は `madore.exe`）を PATH の通った場所に置く。

| OS | アーカイブ |
|----|-----------|
| Linux (x86_64) | `madore-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Intel) | `madore-x86_64-apple-darwin.tar.gz` |
| macOS (Apple Silicon) | `madore-aarch64-apple-darwin.tar.gz` |
| Windows (x86_64) | `madore-x86_64-pc-windows-msvc.zip` |

### 3. Cargo（Rust ツールチェーンがある場合）

```sh
cargo install --git https://github.com/ibuibu/madore
```

## 使い方

```sh
madore                 # カレントディレクトリをルートに起動（ブラウザが開く）
madore ./docs          # ディレクトリを指定
madore --no-open ./docs  # ブラウザを自動で開かない
madore --stop ./docs     # そのルートで動いているサーバーを停止
```

`madore` を実行するとサーバーを端末から切り離してバックグラウンド起動し、空きポート `http://127.0.0.1:<port>` で配信したままコマンドは終了する。同じルートを再度開くと既存サーバーを再利用する（ポートは `~/.local/state/madore/` に記録）。

## 仕組み

| 領域 | 役割 |
|------|------|
| サーバー (comrak) | Markdown → GFM 構造の HTML 化 |
| クライアント (vanilla JS) | highlight.js / KaTeX / Mermaid の見た目付与 |

Markdown の構造化はサーバー側の comrak が行い、コードの色付け・数式・図の描画はクライアント側の JS が後処理する。npm / TypeScript のビルドは使わず、ベンダリングした静的アセットをバイナリに埋め込んでいる。

## ソースからビルド

```sh
cargo build --release   # 生成物: target/release/madore
cargo test              # テスト
```

## リリース

`vX.Y.Z` 形式のタグを push すると、GitHub Actions が各 OS 向けバイナリをビルドして Releases に添付する。

```sh
git tag v0.1.0
git push origin v0.1.0
```

## ライセンス

MIT
