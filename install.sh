#!/bin/sh
# madore インストーラ。GitHub Releases からプリビルドバイナリを取得する。
#   curl -fsSL https://raw.githubusercontent.com/ibuibu/madore/main/install.sh | sh
# インストール先は $MADORE_BIN_DIR（既定: ~/.local/bin）。
set -eu

REPO="ibuibu/madore"
BIN="madore"
DEST="${MADORE_BIN_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux)
    case "$arch" in
      x86_64 | amd64) target="x86_64-unknown-linux-musl" ;;
      *) echo "未対応のアーキテクチャ: $arch (Releases から手動で取得してください)" >&2; exit 1 ;;
    esac
    ;;
  Darwin)
    case "$arch" in
      x86_64) target="x86_64-apple-darwin" ;;
      arm64 | aarch64) target="aarch64-apple-darwin" ;;
      *) echo "未対応のアーキテクチャ: $arch" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "未対応のOS: $os (Windows は Releases から zip をダウンロードしてください)" >&2
    exit 1
    ;;
esac

asset="${BIN}-${target}.tar.gz"
url="https://github.com/${REPO}/releases/latest/download/${asset}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "ダウンロード中: $url"
curl -fsSL "$url" -o "$tmp/$asset"
tar -xzf "$tmp/$asset" -C "$tmp"

mkdir -p "$DEST"
install -m 755 "$tmp/$BIN" "$DEST/$BIN"

echo "インストール完了: $DEST/$BIN"
case ":$PATH:" in
  *":$DEST:"*) ;;
  *) echo "note: $DEST が PATH に含まれていません。シェルの設定に追加してください。" ;;
esac
