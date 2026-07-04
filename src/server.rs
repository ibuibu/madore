use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use tokio::sync::{Notify, broadcast};

use crate::routes;

/// ハンドラ間で共有する状態。
#[derive(Clone)]
pub struct AppState {
    /// 正規化済みのルートディレクトリ。
    pub root: PathBuf,
    /// ルートの表示名。
    pub root_name: String,
    /// ファイル変更通知の送信端。
    pub tx: broadcast::Sender<String>,
    /// /api/shutdown で graceful shutdown を発火させる合図。
    pub shutdown: Arc<Notify>,
}

/// ルーターを組み立てる。
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/", get(routes::index))
        .route("/api/health", get(routes::health))
        .route("/api/tree", get(routes::tree))
        .route("/api/content", get(routes::content))
        .route("/api/shutdown", post(routes::shutdown))
        .route("/events", get(routes::events))
        .route("/assets/{*file}", get(routes::asset))
        .layer(middleware::from_fn(guard_host))
        .with_state(state)
}

/// Host ヘッダを 127.0.0.1 / localhost に限定する。
/// DNS リバインディングで攻撃者ドメインからローカルサーバーへアクセスされるのを防ぐ。
async fn guard_host(req: Request, next: Next) -> Result<Response, StatusCode> {
    if let Some(host) = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
    {
        // "127.0.0.1:1234" / "[::1]:1234" / "localhost" からホスト名部分を取り出す。
        let hostname = match host.rsplit_once(':') {
            Some((h, _)) => h,
            None => host,
        };
        if matches!(hostname, "127.0.0.1" | "localhost" | "[::1]" | "::1") {
            return Ok(next.run(req).await);
        }
    }
    Err(StatusCode::FORBIDDEN)
}
