mod assets;
mod daemon;
mod fsutil;
mod render;
mod routes;
mod server;
mod tree;
mod watcher;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::net::TcpListener;
use tokio::sync::{Notify, broadcast};

use crate::server::AppState;

/// ブラウザで markdown を表示するローカルビューア。
#[derive(Parser)]
#[command(
    name = "madore",
    version,
    about = "Markdown viewer served in your browser"
)]
struct Cli {
    /// 表示するルートディレクトリ（省略時はカレントディレクトリ）。
    path: Option<PathBuf>,

    /// ブラウザを自動で開かない。
    #[arg(long)]
    no_open: bool,

    /// サーバーをフォアグラウンドで常駐実行する（通常はデーモンとして自動起動される内部用）。
    #[arg(long, hide = true)]
    foreground: bool,

    /// バインドするポート（0 で自動割り当て。内部用）。
    #[arg(long, default_value_t = 0, hide = true)]
    port: u16,

    /// このルートで動いているサーバーを停止する。
    #[arg(long)]
    stop: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let root = match &cli.path {
        Some(path) => path.clone(),
        None => std::env::current_dir().context("カレントディレクトリを取得できません")?,
    };
    let root = root
        .canonicalize()
        .with_context(|| format!("パスが見つかりません: {}", root.display()))?;

    if cli.stop {
        // このルートで動いているサーバーを停止する。
        stop(root)
    } else if cli.foreground {
        // デーモンの実体。指定ポートでブロックし続ける。
        run_server(root, cli.port)
    } else {
        // ランチャー。既存サーバーに相乗りするか、無ければデーモンを起動して即終了する。
        launch(root, cli.no_open)
    }
}

/// このルートで動いているサーバーを停止する。
fn stop(root: PathBuf) -> Result<()> {
    if let Some(port) = daemon::recorded_port(&root)
        && daemon::probe(port, &root)
    {
        daemon::send_shutdown(port);
        daemon::clear_record(&root);
        println!("madore: 停止しました: {}", root.display());
    } else {
        println!("madore: 起動していません: {}", root.display());
    }
    Ok(())
}

/// フォアグラウンドで HTTP サーバーを起動してブロックする。
fn run_server(root: PathBuf, port: u16) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("tokio ランタイムを初期化できません")?;
    runtime.block_on(async move {
        let root_name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("madore")
            .to_string();

        let (tx, _rx) = broadcast::channel::<String>(64);

        let shutdown = Arc::new(Notify::new());
        let state = AppState {
            root: root.clone(),
            root_name,
            tx: tx.clone(),
            shutdown: shutdown.clone(),
        };

        // 先にポートを bind して即座に応答できるようにする。
        // ファイル監視のセットアップ（巨大リポジトリでは inotify 登録に数十秒かかりうる）を
        // 待ってから bind すると、ランチャーの起動確認がタイムアウトしてしまう。
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .context("ポートをバインドできません")?;
        let addr = listener.local_addr()?;

        // このメッセージは端末ではなくログファイルに出る（デタッチ時に stdout を回すため）。
        println!("madore: {} を http://{} で配信中", root.display(), addr);

        // ファイル監視はサーバー起動をブロックしないよう別スレッドで設定する。
        // 返ってきた Debouncer は保持し続け、シャットダウンまで監視を生かす。
        let watch_root = root.clone();
        let watcher_task = tokio::task::spawn_blocking(move || watcher::spawn(&watch_root, tx));

        axum::serve(listener, server::app(state))
            .with_graceful_shutdown(async move { shutdown.notified().await })
            .await
            .context("サーバーが停止しました")?;

        // シャットダウン後に監視を片付ける。設定に失敗していたらログに残す。
        match watcher_task.await {
            Ok(Ok(debouncer)) => drop(debouncer),
            Ok(Err(e)) => eprintln!("madore: ファイル監視を開始できません: {e}"),
            Err(e) => eprintln!("madore: ファイル監視スレッドが異常終了しました: {e}"),
        }

        Ok::<(), anyhow::Error>(())
    })
}

/// 既存サーバーがあればそれを開き、無ければデーモンを起動してから終了する。
fn launch(root: PathBuf, no_open: bool) -> Result<()> {
    // 1. このルートに対して既にサーバーが動いていれば、それを使う。
    if let Some(port) = daemon::recorded_port(&root)
        && daemon::probe(port, &root)
    {
        open_if_needed(&format!("http://127.0.0.1:{port}"), no_open);
        return Ok(());
    }

    // 2. 無ければ空きポートを決めて、デーモンを端末から切り離して起動する。
    let port = daemon::pick_free_port()?;
    daemon::spawn_detached(&root, port).context("サーバープロセスを起動できません")?;

    // 3. サーバーが応答するまで待つ。
    if !daemon::wait_until_ready(port, &root, Duration::from_secs(10)) {
        anyhow::bail!("サーバーの起動を確認できませんでした（ログ: state ディレクトリ）");
    }

    // 4. ルート→ポートを記録して、次回以降に再利用できるようにする。
    daemon::record_port(&root, port)?;

    open_if_needed(&format!("http://127.0.0.1:{port}"), no_open);
    Ok(())
}

fn open_if_needed(url: &str, no_open: bool) {
    if no_open {
        return;
    }
    if let Err(e) = open_browser(url) {
        eprintln!("ブラウザを開けませんでした ({e}). 手動で開いてください: {url}");
    }
}

/// ブラウザで URL を開く。
/// WSL では `xdg-open` にフォールバックしても実際には開かないため、
/// Windows 側の既定ブラウザを `explorer.exe` で起動する。
fn open_browser(url: &str) -> Result<()> {
    if is_wsl() {
        // explorer.exe は URL を開くと終了コードが非0になることがあるが、
        // spawn さえ成功すればブラウザは開くので、起動可否だけを見る。
        if std::process::Command::new("explorer.exe")
            .arg(url)
            .spawn()
            .is_ok()
        {
            return Ok(());
        }
    }
    webbrowser::open(url)?;
    Ok(())
}

/// WSL 上で動作しているか。
fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|v| v.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}
