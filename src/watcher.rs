use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use notify_debouncer_full::notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use tokio::sync::broadcast;

use crate::fsutil::{is_excluded_name, is_markdown};

/// ルート配下を再帰監視し、markdown ファイルの変更を broadcast で通知する。
/// 返り値の `Debouncer` は監視スレッドを保持するので、生かし続けること。
pub fn spawn(
    root: &Path,
    tx: broadcast::Sender<String>,
) -> Result<Debouncer<notify_debouncer_full::notify::RecommendedWatcher, RecommendedCache>> {
    let root = root.to_path_buf();
    let watch_root = root.clone();

    let mut debouncer = new_debouncer(
        Duration::from_millis(250),
        None,
        move |result: DebounceEventResult| {
            let Ok(events) = result else {
                return;
            };
            for event in events {
                for path in &event.paths {
                    if !is_relevant(&root, path) {
                        continue;
                    }
                    let rel = path
                        .strip_prefix(&root)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    // 受信者がいなくてもエラーにしない。
                    let _ = tx.send(rel);
                }
            }
        },
    )?;

    debouncer.watch(&watch_root, RecursiveMode::Recursive)?;
    Ok(debouncer)
}

/// markdown ファイルかつ除外ディレクトリ配下でないパスだけを対象にする。
/// tree.rs と同じ `is_markdown` / `is_excluded_name` を使い、表示と監視の基準を揃える。
fn is_relevant(root: &Path, path: &Path) -> bool {
    if !is_markdown(path) {
        return false;
    }

    let rel = match path.strip_prefix(root) {
        Ok(rel) => rel.to_path_buf(),
        Err(_) => PathBuf::from(path),
    };

    for comp in rel.components() {
        if let Component::Normal(name) = comp
            && let Some(name) = name.to_str()
            && is_excluded_name(name)
        {
            return false;
        }
    }
    true
}
