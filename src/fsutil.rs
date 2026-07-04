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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn markdown_extensions() {
        assert!(is_markdown(Path::new("a.md")));
        assert!(is_markdown(Path::new("dir/b.markdown")));
        assert!(!is_markdown(Path::new("a.txt")));
        assert!(!is_markdown(Path::new("noext")));
        assert!(!is_markdown(Path::new(".env")));
    }

    #[test]
    fn excluded_names() {
        assert!(is_excluded_name(".git"));
        assert!(is_excluded_name(".hidden"));
        assert!(is_excluded_name("node_modules"));
        assert!(is_excluded_name("target"));
        assert!(is_excluded_name("foo~"));
        assert!(is_excluded_name("foo.swp"));
        assert!(!is_excluded_name("src"));
        assert!(!is_excluded_name("README.md"));
    }
}
