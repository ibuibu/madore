use std::collections::HashMap;
use std::convert::Infallible;
use std::path::Path;

use axum::Json;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use serde::Serialize;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::assets;
use crate::fsutil::is_markdown;
use crate::render::{extract_title, render_markdown};
use crate::server::AppState;
use crate::tree::{TreeNode, build_tree};

/// アプリシェル(index.html)。
pub async fn index() -> Response {
    match assets::Assets::get("index.html") {
        Some(content) => Html(content.data.into_owned()).into_response(),
        None => (StatusCode::INTERNAL_SERVER_ERROR, "index.html missing").into_response(),
    }
}

/// 埋め込みアセット配信。
pub async fn asset(AxumPath(path): AxumPath<String>) -> Response {
    assets::serve(&path)
}

/// 生存確認用。ランチャーが「このポートで動いているのが同じルートの madore か」を
/// 判定するために使う（app 名と root を返す）。
pub async fn health(State(state): State<AppState>) -> Response {
    // serde_json に依存しないよう {:?} で JSON 文字列としてエスケープする。
    let body = format!(
        "{{\"app\":\"madore\",\"root\":{:?}}}",
        state.root.to_string_lossy()
    );
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body,
    )
        .into_response()
}

/// サーバーを graceful shutdown させる（`madore --stop` から呼ばれる）。
pub async fn shutdown(State(state): State<AppState>) -> Response {
    state.shutdown.notify_one();
    (StatusCode::OK, "shutting down").into_response()
}

#[derive(Serialize)]
pub struct TreeResponse {
    root_name: String,
    nodes: Vec<TreeNode>,
}

/// ルート配下の markdown ツリー。起動直後から返せるので "not selected" にならない。
pub async fn tree(State(state): State<AppState>) -> Json<TreeResponse> {
    Json(TreeResponse {
        root_name: state.root_name.clone(),
        nodes: build_tree(&state.root),
    })
}

#[derive(Serialize)]
pub struct ContentResponse {
    html: String,
    title: String,
    path: String,
}

/// 指定 md を comrak でレンダリングして返す。パスはルート配下に限定する。
pub async fn content(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ContentResponse>, StatusCode> {
    let rel = params.get("path").ok_or(StatusCode::BAD_REQUEST)?;

    let requested = state.root.join(rel);
    let canonical = tokio::fs::canonicalize(&requested)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // パストラバーサル防止: 正規化後もルート配下であること。
    if !canonical.starts_with(&state.root) {
        return Err(StatusCode::FORBIDDEN);
    }

    // ツリーに出るのは markdown だけなので、content も markdown 以外は拒否する
    // （?path=.env のようにルート配下の任意ファイルを読ませない多層防御）。
    if !is_markdown(&canonical) {
        return Err(StatusCode::FORBIDDEN);
    }

    // サイズ上限。巨大ファイルでメモリ・CPU を食い潰さないよう先に弾く。
    const MAX_BYTES: u64 = 8 * 1024 * 1024;
    let meta = tokio::fs::metadata(&canonical)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if meta.len() > MAX_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    // 非 UTF-8 でも 404 にせず、置換文字混じりで表示する。
    let bytes = tokio::fs::read(&canonical)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let md = String::from_utf8_lossy(&bytes).into_owned();

    // comrak は同期 CPU 処理なので、非同期ワーカーをブロックしないよう spawn_blocking に載せる。
    let (html, title_opt) = tokio::task::spawn_blocking(move || {
        let title = extract_title(&md);
        let html = render_markdown(&md);
        (html, title)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let title = title_opt.unwrap_or_else(|| file_stem(&canonical));

    Ok(Json(ContentResponse {
        html,
        title,
        path: rel.clone(),
    }))
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled")
        .to_string()
}

/// ライブリロード用 SSE。変更があったファイルの相対パスを `reload` イベントで push。
pub async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).map(|msg| {
        // Lagged（チャネル溢れで取りこぼし）時は "*" を送り、
        // クライアントに全体再読込＋表示中ファイル再取得を促す。
        let data = msg.unwrap_or_else(|_| "*".to_string());
        Ok(Event::default().event("reload").data(data))
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
