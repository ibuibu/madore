# madore

ブラウザで Markdown を表示するローカルビューア（Rust製）。
ローカルサーバーを立ててブラウザで開き、
**引数なしでもカレントディレクトリをルートにして、サイドバーへ最初からファイルツリーを表示する**。

## 特徴

- サイドバーに Markdown ファイルツリーを常時表示、クリックで本文を表示
- ライブリロード（ファイル保存を検知して自動更新、SSE）
- GitHub Flavored Markdown（テーブル / タスクリスト / 脚注 / GitHub Alerts）
- シンタックスハイライト（highlight.js）
- 数式（KaTeX）・Mermaid 図
- 静的アセットをバイナリに埋め込み、単体で配布可能
- ダーク / ライトはブラウザ設定に追従
- サーバーはバックグラウンドに常駐し、コマンドはすぐ終了する（同じルートの2回目以降は既存サーバーを再利用）

## 使い方

```sh
# カレントディレクトリをルートに起動（ブラウザが開き、コマンドはすぐ戻る）
madore

# ディレクトリを指定
madore ./docs

# ブラウザを自動で開かない
madore --no-open ./docs

# そのルートで動いているサーバーを停止
madore --stop ./docs
```

`madore` を実行するとサーバーを端末から切り離して（`setsid`）バックグラウンド起動し、
空きポート `http://127.0.0.1:<port>` で配信したままコマンドは終了する。
同じルートを再度開くと既存サーバーを再利用する（ポートは `~/.local/state/madore/` に記録）。

## ビルド

```sh
cargo build --release
# 生成物: target/release/madore
```

## 構成

| 領域 | 役割 |
|---|---|
| サーバー (comrak) | Markdown → GFM 構造の HTML 化 |
| クライアント (vanilla JS) | highlight.js / KaTeX / Mermaid の見た目付与 |

- `src/` … CLI・axum サーバー・comrak レンダリング・notify 監視・ツリー構築
- `assets/` … `index.html` / `app.js` / `app.css` と `vendor/`（埋め込み対象）
