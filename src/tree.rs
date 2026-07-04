use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::fsutil::{is_excluded_name, is_markdown};

/// サイドバーに表示するファイルツリーの1ノード。
#[derive(Serialize)]
pub struct TreeNode {
    pub name: String,
    /// ルートからの相対パス（スラッシュ区切り）。ディレクトリは空文字にはしない。
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<TreeNode>,
}

/// ルート直下のツリーを構築する。ルート自身はノードにせず children のみ返す。
pub fn build_tree(root: &Path) -> Vec<TreeNode> {
    build_children(root, root)
}

fn build_children(root: &Path, dir: &Path) -> Vec<TreeNode> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut dirs: Vec<TreeNode> = Vec::new();
    let mut files: Vec<TreeNode> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(String::from) else {
            continue;
        };
        if is_excluded_name(&name) {
            continue;
        }

        // symlink を辿らない（file_type は symlink そのものを返す）。
        // これにより循環リンクでの無限再帰・スタックオーバーフローを防ぐ。
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            let children = build_children(root, &path);
            // markdownを1つも含まないディレクトリは出さない。
            if children.is_empty() {
                continue;
            }
            dirs.push(TreeNode {
                name,
                path: rel_path(root, &path),
                is_dir: true,
                children,
            });
        } else if file_type.is_file() && is_markdown(&path) {
            files.push(TreeNode {
                name,
                path: rel_path(root, &path),
                is_dir: false,
                children: Vec::new(),
            });
        }
    }

    dirs.sort_by_key(|n| n.name.to_lowercase());
    files.sort_by_key(|n| n.name.to_lowercase());
    dirs.extend(files);
    dirs
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
