# solana-askama-kit

> Full-stack Rust toolkit for building Solana dApps with **Axum** and **Askama**.

No more copy-pasting the same Anchor client setup, keypair loading, error handling, and timestamp utilities across every project. `solana-askama-kit` gives you a thin, opinionated layer over these patterns so you can focus on your program logic.

---

## Features

| Module | What it gives you |
|---|---|
| `SolanaClient` | Anchor + RPC client with automatic keypair resolution |
| `HtmlTemplate<T>` | Askama → Axum response bridge (one line) |
| `AppError` | Unified error type with styled HTML error page |
| `time` | `parse_datetime_local`, `format_timestamp`, `is_active`, `is_expired` |
| `pda` | `find_pda_with_id`, `find_pda_with_id_and_name`, `verify_pda`, `recover_poll_id` |
| `router` | `AppRouter` builder, `serve()`, `init_tracing()` |

---

## Quick Start

```toml
# Cargo.toml
[dependencies]
solana-askama-kit = "0.1"
```

```rust
use solana_askama_kit::{
    AppError, HtmlTemplate, SolanaClient,
    pda::find_pda_with_id,
    time::parse_datetime_local,
    router::{AppRouter, serve, init_tracing},
};
use anchor_client::Cluster;
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate { greeting: String }

async fn index() -> impl axum::response::IntoResponse {
    HtmlTemplate(IndexTemplate { greeting: "Hello Solana".into() })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let app = AppRouter::new().get("/", index).build();
    serve(app, "127.0.0.1:3000").await
}
```

---

## Keypair Resolution

`SolanaClient::new(cluster)` looks for your keypair in this order:

1. `SOLANA_KEYPAIR` environment variable (path to a JSON keypair file)
2. `~/.config/solana/id.json` (default Solana CLI location)

Override with an explicit path:

```rust
let client = SolanaClient::with_keypair_path(Cluster::Devnet, "/my/custom/keypair.json")?;
```

---

## Example: Voting dApp

See [`examples/voting/`](examples/voting/) for a full Anchor + Axum + Askama voting dApp that demonstrates every feature of the kit.

**Before** (raw boilerplate):
```rust
// Manual template bridge
impl<T: Template> IntoResponse for HtmlTemplate<T> { /* 10 lines */ }

// Manual keypair loading
let payer = read_keypair_file("/home/you/.config/solana/id.json").expect("...");
let provider = Client::new_with_options(Cluster::Devnet, Arc::new(payer), CommitmentConfig::confirmed());

// Manual PDA derivation
let (poll_pda, _) = Pubkey::find_program_address(&[b"poll", &poll_id.to_le_bytes()], &program_id);

// Manual timestamp parsing
NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
    .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).timestamp())
    .map_err(|e| format!("Invalid date: {}", e))
```

**After** (with the kit):
```rust
let client = SolanaClient::new(Cluster::Devnet)?;
let (poll_pda, _) = find_pda_with_id(b"poll", poll_id, &voting::ID);
let start_ts = parse_datetime_local(&form.poll_start).map_err(AppError::other)?;
```

---

## Project Structure

```
solana-askama-kit/
├── src/
│   ├── lib.rs          # Re-exports
│   ├── client.rs       # SolanaClient
│   ├── error.rs        # AppError + HTML error page
│   ├── response.rs     # HtmlTemplate<T>
│   ├── time.rs         # Timestamp helpers
│   ├── pda.rs          # PDA derivation helpers
│   └── router.rs       # AppRouter builder + serve() + init_tracing()
└── examples/
    └── voting/         # Full reference implementation
```

---

## Contributing

PRs welcome! Some areas to explore:

- `#[derive(AnchorForm)]` — generate Serde form structs from an Anchor IDL
- Pagination helpers for `get_program_accounts`
- Cluster selection from environment variable (`SOLANA_CLUSTER`)
- WebSocket subscription helpers

---

## License

MIT
