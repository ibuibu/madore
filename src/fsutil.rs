use std::path::Path;

/// markdown ファイルか（拡張子で判定）。
pub fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md") | Some("markdown")
    )
}

/// ツリー表示・監視から除外する名前（ディレクトリ/ファイル共通）。
/// ドット始まり、既知のビルド/VCS ディレクトリ、エディタ一時ファイルを弾く。
/// tree.rs と watcher.rs で同じ基準を使い、表示と監視のずれを防ぐ。
pub fn is_excluded_name(name: &str) -> bool {
    name.starts_with('.')
        || matches!(name, "node_modules" | "target")
        || name.ends_with('~')
        || name.ends_with(".swp")
}
