//! # solana-askama-kit
//!
//! A full-stack Rust toolkit for building Solana dApps with Axum + Askama.
//!
//! ## Features
//! - [`SolanaClient`] — zero-boilerplate Anchor + RPC client setup
//! - [`HtmlTemplate`] — Askama → Axum response bridge
//! - [`AppError`] — unified error type with HTML error responses
//! - [`time`] — datetime parsing and formatting utilities
//! - [`pda`] — PDA seed derivation helpers
//!
//! ## Quick Start
//! ```rust,no_run
//! use solana_askama_kit::{SolanaClient, HtmlTemplate, AppError};
//! use askama::Template;
//!
//! #[derive(Template)]
//! #[template(path = "index.html")]
//! struct IndexTemplate { title: String }
//!
//! async fn index() -> Result<impl axum::response::IntoResponse, AppError> {
//!     Ok(HtmlTemplate(IndexTemplate { title: "Hello Solana".into() }))
//! }
//! ```

pub mod client;
pub mod error;
pub mod response;
pub mod time;
pub mod pda;
pub mod router;

// Flat re-exports for ergonomic `use solana_askama_kit::*`
pub use client::SolanaClient;
pub use error::AppError;
pub use response::HtmlTemplate;
