<div align="center">

# madore

**A local Markdown viewer that opens in your browser**

Serves a directory over `localhost` and shows a file tree in the sidebar from the start —
even with no arguments (it uses the current directory). Saves reload automatically. A single Rust binary.

[![Release](https://img.shields.io/github/v/release/ibuibu/madore?style=flat-square)](https://github.com/ibuibu/madore/releases)
[![License](https://img.shields.io/github/license/ibuibu/madore?style=flat-square)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white)

<img src="docs/screenshot.png" width="820" alt="madore screenshot" />

</div>

## ✨ Features

- 📂 **File tree** — always shown in the sidebar; click to open
- 🔄 **Live reload** — detects saves and refreshes automatically (SSE)
- 📝 **GitHub Flavored Markdown** — tables, task lists, footnotes, GitHub Alerts
- 🎨 **Rich rendering** — syntax highlighting, KaTeX math, Mermaid diagrams
- 📦 **Single binary** — static assets are embedded; ships with no dependencies
- 🌗 **Dark / light** — follows your browser preference
- 🚀 **Returns instantly** — the server runs in the background; the command exits right away (reused on later runs)

## 📦 Installation

<details open>
<summary><b>Install script (Linux / macOS, no Rust needed)</b></summary>

```sh
curl -fsSL https://raw.githubusercontent.com/ibuibu/madore/main/install.sh | sh
```

Downloads the binary for your OS from GitHub Releases and places it in `~/.local/bin` (override with `$MADORE_BIN_DIR`). The Linux build is a fully static musl binary, so it runs on any distro.

</details>

<details>
<summary><b>Download a prebuilt binary manually</b></summary>

Grab an archive from [Releases](https://github.com/ibuibu/madore/releases) and put `madore` (`madore.exe` on Windows) somewhere on your `PATH`.

| OS | Archive |
|----|---------|
| Linux (x86_64, static musl) | `madore-x86_64-unknown-linux-musl.tar.gz` |
| macOS (Intel) | `madore-x86_64-apple-darwin.tar.gz` |
| macOS (Apple Silicon) | `madore-aarch64-apple-darwin.tar.gz` |
| Windows (x86_64) | `madore-x86_64-pc-windows-msvc.zip` |

</details>

<details>
<summary><b>Cargo (with a Rust toolchain)</b></summary>

```sh
cargo install --git https://github.com/ibuibu/madore
```

</details>

## 🚀 Usage

```sh
madore                   # open the current directory (browser opens)
madore ./docs            # open a specific directory
madore --no-open ./docs  # don't open the browser automatically
madore --stop ./docs     # stop the server for that root
```

Running `madore` detaches the server into the background and keeps serving at `http://127.0.0.1:<port>` while the command returns immediately. Opening the same root again reuses the running server (the port is recorded under `~/.local/state/madore/`).

## 🛠 How it works

| Layer | Responsibility |
|-------|----------------|
| Server (comrak) | Markdown → GFM-structured HTML |
| Client (vanilla JS) | highlight.js / KaTeX / Mermaid rendering |

The server structures the Markdown with comrak; the client handles code coloring, math, and diagrams. There is no npm / TypeScript build — vendored static assets are embedded into the binary.

## 🔧 Build from source

```sh
cargo build --release   # output: target/release/madore
cargo test              # run tests
```

## 📦 Releasing

Pushing a `vX.Y.Z` tag triggers GitHub Actions to build per-OS binaries and attach them to the release.

```sh
git tag -a v0.1.0 -m v0.1.0
git push origin v0.1.0
```

## 📄 License

[MIT](LICENSE)
