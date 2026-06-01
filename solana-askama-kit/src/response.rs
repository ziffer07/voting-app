//! Axum response bridge for Askama templates.
//!
//! Wrap any [`askama::Template`] in [`HtmlTemplate`] and return it from an
//! Axum handler — the template is rendered and served as `text/html`.
//!
//! # Example
//! ```rust,no_run
//! use askama::Template;
//! use solana_askama_kit::HtmlTemplate;
//!
//! #[derive(Template)]
//! #[template(path = "index.html")]
//! struct IndexTemplate { greeting: String }
//!
//! async fn index() -> impl axum::response::IntoResponse {
//!     HtmlTemplate(IndexTemplate { greeting: "Hello".into() })
//! }
//! ```

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

/// Newtype wrapper that renders an Askama template as an Axum HTML response.
pub struct HtmlTemplate<T>(pub T);

impl<T: Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                tracing::error!("Template render failed: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!(
                        "<h1>Template Error</h1><pre>{}</pre>",
                        html_escape(&err.to_string())
                    )),
                )
                    .into_response()
            }
        }
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
