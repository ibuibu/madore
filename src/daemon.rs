use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

/// 空きポートを取得する。取得後すぐ listener を閉じるので、ごく短時間の競合はありうるが
/// 直後に子プロセスが同じポートを bind するので実用上は問題ない。
pub fn pick_free_port() -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).context("空きポートを取得できません")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

/// サーバーのポートを記録する state ディレクトリ（XDG_STATE_HOME 準拠）。
fn state_dir() -> PathBuf {
    if let Ok(x) = std::env::var("XDG_STATE_HOME")
        && !x.is_empty()
    {
        return PathBuf::from(x).join("madore");
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.is_empty()
    {
        return PathBuf::from(home).join(".local/state/madore");
    }
    std::env::temp_dir().join("madore")
}

/// ルートごとのポート記録ファイル（ルートパスのハッシュでファイル名を決める）。
fn record_file(root: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    root.hash(&mut hasher);
    state_dir().join(format!("server-{:016x}.port", hasher.finish()))
}

/// このルートに対して記録済みのポートを返す（あれば）。
pub fn recorded_port(root: &Path) -> Option<u16> {
    fs::read_to_string(record_file(root))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// このルートのサーバーポートを記録する。
pub fn record_port(root: &Path, port: u16) -> Result<()> {
    let dir = state_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("state ディレクトリを作成できません: {}", dir.display()))?;
    fs::write(record_file(root), port.to_string())?;
    Ok(())
}

/// 指定ポートで madore サーバーが動いていて、かつ root が一致するかを確認する。
/// /api/health のレスポンスに app 名と root パスが含まれるかで判定する。
pub fn probe(port: u16, root: &Path) -> bool {
    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));

    let req = "GET /api/health HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }

    let mut body = String::new();
    if stream.read_to_string(&mut body).is_err() {
        return false;
    }

    body.contains("\"app\":\"madore\"") && body.contains(root.to_string_lossy().as_ref())
}

/// サーバーに shutdown を要求する。200 が返れば成功とみなす。
pub fn send_shutdown(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));

    let req = "POST /api/shutdown HTTP/1.0\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }

    let mut resp = String::new();
    let _ = stream.read_to_string(&mut resp);
    resp.starts_with("HTTP/1.") && resp.contains(" 200")
}

/// このルートのポート記録を削除する。
pub fn clear_record(root: &Path) {
    let _ = fs::remove_file(record_file(root));
}

/// サーバーが起動して応答するまで待つ。
pub fn wait_until_ready(port: u16, root: &Path, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if probe(port, root) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// 自分自身のバイナリを `--foreground` 付きで、端末から切り離して起動する。
/// 親（ランチャー）が終了してもこのサーバーは生き続ける。
pub fn spawn_detached(root: &Path, port: u16) -> Result<()> {
    let exe = std::env::current_exe().context("実行ファイルのパスを取得できません")?;

    let dir = state_dir();
    fs::create_dir_all(&dir)?;
    let log = fs::File::create(dir.join(format!("server-{port}.log")))
        .context("ログファイルを作成できません")?;

    let mut cmd = Command::new(exe);
    cmd.arg(root)
        .arg("--foreground")
        .arg("--port")
        .arg(port.to_string())
        .arg("--no-open")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // 新セッションを作り、制御端末・親プロセスグループから切り離す。
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
        cmd.creation_flags(0x0000_0008 | 0x0000_0200);
    }

    cmd.spawn().context("サーバープロセスを起動できません")?;
    Ok(())
}
