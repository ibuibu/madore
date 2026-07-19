use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use notify_debouncer_full::notify::event::ModifyKind;
use notify_debouncer_full::notify::{EventKind, RecursiveMode};
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
                // 内容変更と無関係なイベントは無視する。特に配信のためにファイルを
                // read すると Access(Open) が飛ぶため、これを拾うと
                // リロード→再read→再イベント… の無限ループになる。
                if !is_content_change(&event.kind) {
                    continue;
                }
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

/// ファイルの内容変更（作成・書き込み・リネーム・削除）とみなせるイベントか。
/// 読み取り(Access)や atime/権限だけの Metadata 変更は「変更」ではないので除外する。
fn is_content_change(kind: &EventKind) -> bool {
    match kind {
        // ファイルを開く/読むだけ。配信のための read もここに来るので必ず無視する。
        EventKind::Access(_) => false,
        // atime・権限など内容に無関係なメタデータのみの変更。
        EventKind::Modify(ModifyKind::Metadata(_)) => false,
        // 作成・データ書き込み・リネーム・削除・その他は再描画すべき変更とみなす。
        _ => true,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use notify_debouncer_full::notify::event::{
        AccessKind, CreateKind, DataChange, MetadataKind, RemoveKind,
    };

    #[test]
    fn ignores_read_and_metadata_events() {
        // 配信のための read で飛ぶ Access(Open) は無視する（無限ループ防止）。
        assert!(!is_content_change(&EventKind::Access(AccessKind::Open(
            notify_debouncer_full::notify::event::AccessMode::Any
        ))));
        assert!(!is_content_change(&EventKind::Access(AccessKind::Close(
            notify_debouncer_full::notify::event::AccessMode::Read
        ))));
        // atime・権限だけの変更も内容変更ではない。
        assert!(!is_content_change(&EventKind::Modify(
            ModifyKind::Metadata(MetadataKind::Any)
        )));
    }

    #[test]
    fn reports_real_changes() {
        assert!(is_content_change(&EventKind::Create(CreateKind::File)));
        assert!(is_content_change(&EventKind::Modify(ModifyKind::Data(
            DataChange::Content
        ))));
        assert!(is_content_change(&EventKind::Remove(RemoveKind::File)));
    }
}
