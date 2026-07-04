use comrak::{Options, markdown_to_html};

/// comrak を GFM 相当 + alerts + 数式($...$)拡張で設定して md→HTML する。
/// シンタックスハイライト / Mermaid / KaTeX の見た目付与はクライアント側の後処理に任せる。
pub fn render_markdown(md: &str) -> String {
    let mut options = Options::default();

    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.alerts = true;
    options.extension.math_dollars = true;
    options.extension.front_matter_delimiter = Some("---".to_string());

    // ローカルのビューアなので md 内の生 HTML もそのまま描画する。
    options.render.r#unsafe = true;

    markdown_to_html(md, &options)
}

/// 本文の最初の H1 見出しをタイトルとして抽出する。無ければ None。
/// ATX (`# 見出し`) と setext (`見出し` の次行が `===`) の両方に対応。
/// 先頭が閉じられた front matter (`---` ... `---`) ならその後ろから探す。
pub fn extract_title(md: &str) -> Option<String> {
    let lines: Vec<&str> = md.lines().collect();

    // front matter が「閉じられている」場合のみ、その後ろから探す。
    let start = front_matter_end(&lines).unwrap_or(0);
    if let Some(title) = find_h1(&lines[start..]) {
        return Some(title);
    }
    // front matter とみなした範囲に見出しが無い / 閉じられていない場合は
    // 全体からも探す（未閉じ `---` や先頭の水平線でタイトルを失わない）。
    if start != 0 {
        return find_h1(&lines);
    }
    None
}

/// 先頭が front matter (`---` ... `---`) なら閉じ行の次の行インデックスを返す。
/// 閉じられていなければ front matter とはみなさず None。
fn front_matter_end(lines: &[&str]) -> Option<usize> {
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return None;
    }
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim_end() == "---" {
            return Some(i + 1);
        }
    }
    None
}

/// 最初の H1 見出しテキストを返す。
fn find_h1(lines: &[&str]) -> Option<String> {
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();

        // ATX: `# 見出し`
        if let Some(rest) = trimmed.strip_prefix("# ") {
            return Some(rest.trim().to_string());
        }

        // setext: 通常テキスト行の次行が `===...` なら H1
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && let Some(next) = lines.get(i + 1)
        {
            let underline = next.trim();
            if !underline.is_empty() && underline.chars().all(|c| c == '=') {
                return Some(trimmed.trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atx_heading() {
        assert_eq!(extract_title("# Title\nbody").as_deref(), Some("Title"));
    }

    #[test]
    fn setext_heading() {
        assert_eq!(
            extract_title("Setext\n======\nbody").as_deref(),
            Some("Setext")
        );
    }

    #[test]
    fn skips_closed_front_matter() {
        let md = "---\ntitle: meta\n---\n# Real\nbody";
        assert_eq!(extract_title(md).as_deref(), Some("Real"));
    }

    #[test]
    fn falls_back_when_front_matter_unclosed() {
        // 閉じ `---` が無い場合でも、本文の見出しを拾えること。
        let md = "---\ntitle: meta\n\n# Real\nbody";
        assert_eq!(extract_title(md).as_deref(), Some("Real"));
    }

    #[test]
    fn none_when_no_heading() {
        assert_eq!(extract_title("just text\nmore text"), None);
    }
}
