//! Solana client setup — wraps both the Anchor program client and the raw RPC client.
//!
//! Reads the keypair path from the `SOLANA_KEYPAIR` environment variable, falling
//! back to `~/.config/solana/id.json` when the variable is not set.
//!
//! # Example
//! ```rust,no_run
//! use solana_askama_kit::SolanaClient;
//! use anchor_client::Cluster;
//!
//! let client = SolanaClient::new(Cluster::Devnet).expect("Failed to create client");
//! let program = client.program(my_program::ID).unwrap();
//! ```

use std::sync::Arc;

use anchor_client::{Client, ClientError, Cluster, CommitmentConfig, Signer, anchor_lang};
use anchor_lang::Id;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::{Keypair, read_keypair_file}};

/// Combined Anchor + RPC client with automatic keypair resolution.
pub struct SolanaClient {
    /// Raw JSON-RPC connection (for `get_program_accounts`, balance checks, etc.)
    pub rpc: RpcClient,

    /// Anchor program client factory.
    anchor: Client<Arc<Keypair>>,

    /// Public key of the fee payer / signer.
    pub payer_pubkey: Pubkey,
}

impl SolanaClient {
    /// Build a new client for the given cluster.
    ///
    /// Keypair is resolved in this order:
    /// 1. `SOLANA_KEYPAIR` environment variable (path to a JSON keypair file)
    /// 2. `~/.config/solana/id.json`
    pub fn new(cluster: Cluster) -> Result<Self, SolanaClientError> {
        let keypair = Self::load_keypair()?;
        Self::new_with_keypair(cluster, keypair)
    }

    /// Build a client using an explicit keypair path.
    pub fn with_keypair_path(
        cluster: Cluster,
        path: &str,
    ) -> Result<Self, SolanaClientError> {
        let keypair = read_keypair_file(path)
            .map_err(|e| SolanaClientError::KeypairRead(e.to_string()))?;
        Self::new_with_keypair(cluster, keypair)
    }

    /// Build a client using an already-loaded `Keypair`.
    pub fn new_with_keypair(
        cluster: Cluster,
        keypair: Keypair,
    ) -> Result<Self, SolanaClientError> {
        let rpc_url = cluster.url().to_string();
        let payer_pubkey = keypair.pubkey();
        let payer = Arc::new(keypair);

        let rpc = RpcClient::new_with_commitment(
            rpc_url,
            CommitmentConfig::confirmed(),
        );

        let anchor = Client::new_with_options(
            cluster,
            Arc::clone(&payer),
            CommitmentConfig::confirmed(),
        );

        Ok(Self { rpc, anchor, payer_pubkey })
    }

    /// Get an Anchor [`anchor_client::Program`] handle for `program_id`.
    pub fn program<C: anchor_lang::Id>(
        &self,
        program_id: Pubkey,
    ) -> Result<anchor_client::Program<Arc<Keypair>>, ClientError> {
        self.anchor.program(program_id)
    }

    /// Get an Anchor program handle using the program's own `ID` constant.
    ///
    /// Equivalent to `client.program(MyProgram::ID)` but slightly terser.
    pub fn program_for<P: Id>(
        &self,
    ) -> Result<anchor_client::Program<Arc<Keypair>>, ClientError> {
        self.anchor.program(P::id())
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    fn load_keypair() -> Result<Keypair, SolanaClientError> {
        // 1. Env var
        if let Ok(path) = std::env::var("SOLANA_KEYPAIR") {
            return read_keypair_file(&path)
                .map_err(|e| SolanaClientError::KeypairRead(e.to_string()));
        }

        // 2. Default path
        let home = std::env::var("HOME")
            .unwrap_or_else(|_| "/root".to_string());
        let default_path = format!("{}/.config/solana/id.json", home);

        read_keypair_file(&default_path)
            .map_err(|e| SolanaClientError::KeypairRead(
                format!("Could not load keypair from {} (set SOLANA_KEYPAIR env var to override): {}", default_path, e)
            ))
    }
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum SolanaClientError {
    #[error("Failed to read keypair: {0}")]
    KeypairRead(String),

    #[error("Anchor client error: {0}")]
    Anchor(#[from] ClientError),
}
