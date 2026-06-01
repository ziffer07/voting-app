//! Axum router builder helpers.
//!
//! Provides a thin convenience wrapper around [`axum::Router`] for the common
//! Solana dApp pattern: a single-page HTML form app with a handful of routes.
//!
//! # Example
//! ```rust,no_run
//! use solana_askama_kit::router::AppRouter;
//!
//! let app = AppRouter::new()
//!     .get("/", show_form)
//!     .post("/init", initialize_handler)
//!     .get("/polls", show_polls)
//!     .build();
//! ```

use axum::{
    Router,
    routing::{get, post, MethodRouter},
};

/// Builder for the application [`Router`].
pub struct AppRouter {
    router: Router,
}

impl AppRouter {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self { router: Router::new() }
    }

    /// Register a GET route.
    pub fn get<H, T>(self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + 'static,
        T: Send + 'static,
    {
        Self { router: self.router.route(path, get(handler)) }
    }

    /// Register a POST route.
    pub fn post<H, T>(self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + 'static,
        T: Send + 'static,
    {
        Self { router: self.router.route(path, post(handler)) }
    }

    /// Attach any custom [`MethodRouter`] at `path`.
    pub fn route(self, path: &str, method_router: MethodRouter) -> Self {
        Self { router: self.router.route(path, method_router) }
    }

    /// Merge another router into this one.
    pub fn merge(self, other: Router) -> Self {
        Self { router: self.router.merge(other) }
    }

    /// Consume the builder and return the configured [`Router`].
    pub fn build(self) -> Router {
        self.router
    }
}

impl Default for AppRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Start a Tokio TCP listener and serve the given router.
///
/// Logs the listening address to `tracing::info!`.
///
/// # Example
/// ```rust,no_run
/// use solana_askama_kit::router::{AppRouter, serve};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let app = AppRouter::new().get("/", my_handler).build();
///     serve(app, "127.0.0.1:3000").await
/// }
/// ```
pub async fn serve(router: Router, addr: &str) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Listening on http://{}", addr);
    println!("Listening on http://{}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}

/// Initialise a `tracing_subscriber` with `RUST_LOG` env-filter support.
///
/// Call once at the top of `main` before starting the server.
///
/// ```rust,no_run
/// use solana_askama_kit::router::init_tracing;
/// init_tracing();
/// ```
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
}
