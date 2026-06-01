//! Unified application error type that renders a styled HTML error page via Axum.
//!
//! # Usage
//! ```rust,no_run
//! use solana_askama_kit::AppError;
//!
//! async fn my_handler() -> Result<impl axum::response::IntoResponse, AppError> {
//!     let result = some_fallible_op().map_err(AppError::other)?;
//!     Ok(result)
//! }
//! ```

use axum::{http::StatusCode, response::{Html, IntoResponse, Response}};

/// Unified error enum for Solana + web handlers.
///
/// All variants implement [`IntoResponse`], so handlers can use
/// `Result<_, AppError>` and Axum will serve a proper HTTP error response.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Template render error: {0}")]
    Template(#[from] askama::Error),

    #[error("Anchor client error: {0}")]
    AnchorClient(#[from] anchor_client::ClientError),

    #[error("Solana client error: {0}")]
    SolanaClient(#[from] crate::client::SolanaClientError),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// Convenience constructor for one-off string errors.
    ///
    /// ```rust,no_run
    /// use solana_askama_kit::AppError;
    /// let e = AppError::other("something went wrong");
    /// ```
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    /// Map any `Display` error into [`AppError::Other`].
    ///
    /// Useful with `.map_err(AppError::from_err)`.
    pub fn from_err<E: std::fmt::Display>(e: E) -> Self {
        Self::Other(e.to_string())
    }

    /// HTTP status code for this error variant.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Template(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::AnchorClient(_) => StatusCode::BAD_GATEWAY,
            Self::SolanaClient(_) => StatusCode::BAD_GATEWAY,
            Self::Rpc(_) => StatusCode::BAD_GATEWAY,
            Self::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let message = self.to_string();
        tracing::error!(error = %message, "Handler returned an error");
        let html = error_page_html(status.as_u16(), &message);
        (status, Html(html)).into_response()
    }
}

/// Generates a minimal, self-contained HTML error page.
/// Replace this with your own Askama template for a custom look.
fn error_page_html(code: u16, message: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Error {code}</title>
  <style>
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      background: #0f0f0f;
      color: #e5e5e5;
      font-family: 'JetBrains Mono', 'Fira Code', monospace;
    }}
    .card {{
      border: 1px solid #333;
      border-radius: 8px;
      padding: 2.5rem 3rem;
      max-width: 520px;
      width: 90%;
      background: #161616;
    }}
    .code {{
      font-size: 4rem;
      font-weight: 700;
      color: #ff4444;
      line-height: 1;
      letter-spacing: -2px;
    }}
    .label {{
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 2px;
      color: #666;
      margin-top: 0.5rem;
    }}
    .divider {{
      border: none;
      border-top: 1px solid #262626;
      margin: 1.5rem 0;
    }}
    .message {{
      font-size: 0.875rem;
      color: #aaa;
      word-break: break-word;
      line-height: 1.6;
    }}
    .back {{
      display: inline-block;
      margin-top: 1.5rem;
      font-size: 0.75rem;
      color: #555;
      text-decoration: none;
      border: 1px solid #333;
      padding: 0.4rem 1rem;
      border-radius: 4px;
      transition: color 0.2s, border-color 0.2s;
    }}
    .back:hover {{ color: #e5e5e5; border-color: #666; }}
  </style>
</head>
<body>
  <div class="card">
    <div class="code">{code}</div>
    <div class="label">Application Error</div>
    <hr class="divider" />
    <div class="message">{message}</div>
    <a class="back" href="javascript:history.back()">← Go back</a>
  </div>
</body>
</html>"#,
        code = code,
        message = html_escape(message),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
