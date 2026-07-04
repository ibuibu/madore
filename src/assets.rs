use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// フロントエンドの静的アセット。dev ビルドでは実ファイル、release ではバイナリに埋め込まれる。
#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Assets;

/// 埋め込みアセットを Content-Type 付きで返す。
pub fn serve(path: &str) -> Response {
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
