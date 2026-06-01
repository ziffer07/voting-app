use std::collections::HashMap;

use anchor_client::{Cluster, anchor_lang};
use anchor_lang::{AccountDeserialize, Discriminator, declare_program, system_program};
use askama::Template;
use axum::{Form, Router, extract::Query, response::IntoResponse, routing::{get, post}};
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;

use solana_askama_kit::{
    AppError, HtmlTemplate,
    pda::{find_pda_with_id, find_pda_with_id_and_name, recover_poll_id},
    time::{format_timestamp, is_expired, parse_datetime_local},
};

// ── Your program — swap voting for sf_bc_one when ready ──────────────────────
declare_program!(voting);
use voting::{client::accounts, client::args};

// Keypair path — change this to your actual path
const KEYPAIR_PATH: &str = "/home/aryan/.config/solana/id.json";

// ── Templates ─────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "form.html")]
struct FormTemplate {}

#[derive(Template)]
#[template(path = "poll.html")]
struct PollTemplate {
    poll_name: String,
    poll_description: String,
    poll_start: String,
    poll_end: String,
    candidate1: String,
    candidate2: String,
    poll_signature: String,
    poll_id: i32,
}

#[derive(Template)]
#[template(path = "polls.html")]
struct PollsTemplate {
    polls: Vec<PollInfo>,
}

#[derive(Template)]
#[template(path = "server-error.html")]
struct ServerErrorTemplate {}

// ── View models ───────────────────────────────────────────────────────────────

struct PollInfo {
    poll_name: String,
    poll_description: String,
    poll_start: String,
    poll_end: String,
    poll_end_ts: i64,
    poll_start_ts: i64,
    address: String,
    candidates: Vec<CandidateInfo>,
    is_active: bool,
}

struct CandidateInfo {
    candidate_name: String,
    votes: u64,
}

// ── Form / query types ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PollForm {
    poll_name: String,
    poll_description: String,
    poll_start: String,
    poll_end: String,
    candidate1: String,
    candidate2: String,
}

#[derive(Deserialize)]
struct VoteForm {
    poll_address: String,
    poll_start_ts: i64,
    candidate_name: String,
}

// ── Client helper — uses your local keypair, targets Devnet ──────────────────

fn make_client() -> Result<solana_askama_kit::SolanaClient, AppError> {
    solana_askama_kit::SolanaClient::with_keypair_path(Cluster::Devnet, KEYPAIR_PATH)
        .map_err(|e| AppError::other(e.to_string()))
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(show_form_handler))
        .route("/init", post(initialize_handler))
        .route("/polls", get(show_polls_handler))
        .route("/vote", get(vote_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    println!("Listening on http://127.0.0.1:3001");
    axum::serve(listener, app).await?;
    Ok(())
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn show_form_handler() -> impl IntoResponse {
    HtmlTemplate(FormTemplate {})
}

async fn initialize_handler(
    Form(form): Form<PollForm>,
) -> Result<impl IntoResponse, AppError> {
    let start_ts = parse_datetime_local(&form.poll_start).map_err(AppError::other)?;
    let end_ts   = parse_datetime_local(&form.poll_end).map_err(AppError::other)?;

    let client  = make_client()?;
    let program = client.program::<voting::program::SfBcOne>(voting::ID)?;

    let poll_id: i32 = (start_ts % i32::MAX as i64) as i32;

    let (poll_pda, _)       = find_pda_with_id(b"poll", poll_id, &voting::ID);
    let (candidate1_pda, _) = find_pda_with_id_and_name(poll_id, &form.candidate1, &voting::ID);
    let (candidate2_pda, _) = find_pda_with_id_and_name(poll_id, &form.candidate2, &voting::ID);

    // 1. Init poll
    let poll_ix = program
        .request()
        .accounts(accounts::InitializePoll {
            signer: program.payer(),
            poll_acc: poll_pda,
            system_program: system_program::ID,
        })
        .args(args::InitializePoll {
            poll_id,
            input_name: form.poll_name.clone(),
            input_description: form.poll_description.clone(),
            input_start: start_ts,
            input_end: end_ts,
        })
        .instructions()
        .remove(0);

    let poll_sig = program.request().instruction(poll_ix).send().await?;

    // 2. Init candidate 1
    let c1_ix = program
        .request()
        .accounts(accounts::InitializeCandidate {
            signer: program.payer(),
            poll_acc: poll_pda,
            candidate_acc: candidate1_pda,
            system_program: system_program::ID,
        })
        .args(args::InitializeCandidate {
            poll_id,
            input_candidate_name: form.candidate1.clone(),
        })
        .instructions()
        .remove(0);

    program.request().instruction(c1_ix).send().await?;

    // 3. Init candidate 2
    let c2_ix = program
        .request()
        .accounts(accounts::InitializeCandidate {
            signer: program.payer(),
            poll_acc: poll_pda,
            candidate_acc: candidate2_pda,
            system_program: system_program::ID,
        })
        .args(args::InitializeCandidate {
            poll_id,
            input_candidate_name: form.candidate2.clone(),
        })
        .instructions()
        .remove(0);

    program.request().instruction(c2_ix).send().await?;

    Ok(HtmlTemplate(PollTemplate {
        poll_name: form.poll_name,
        poll_description: form.poll_description,
        poll_start: format_timestamp(start_ts),
        poll_end: format_timestamp(end_ts),
        candidate1: form.candidate1,
        candidate2: form.candidate2,
        poll_signature: poll_sig.to_string(),
        poll_id,
    }))
}

async fn show_polls_handler() -> Result<impl IntoResponse, AppError> {
    let client = make_client()?;

    let all_accounts = client
        .rpc
        .get_program_accounts(&voting::ID)
        .map_err(|e| AppError::Rpc(e.to_string()))?;

    let poll_disc = voting::accounts::PollAccount::DISCRIMINATOR;
    let cand_disc = voting::accounts::CandidateAccount::DISCRIMINATOR;

    let mut candidate_map: HashMap<Pubkey, voting::accounts::CandidateAccount> = HashMap::new();
    for (pubkey, account) in &all_accounts {
        if account.data.len() >= 8 && account.data[..8] == *cand_disc {
            let mut data: &[u8] = &account.data;
            if let Ok(c) = voting::accounts::CandidateAccount::try_deserialize(&mut data) {
                candidate_map.insert(*pubkey, c);
            }
        }
    }

    let mut polls = vec![];
    for (poll_pubkey, account) in &all_accounts {
        if account.data.len() < 8 || account.data[..8] != *poll_disc {
            continue;
        }

        let mut data: &[u8] = &account.data;
        let Ok(poll) = voting::accounts::PollAccount::try_deserialize(&mut data) else {
            continue;
        };

        let Some(poll_id) = recover_poll_id(poll_pubkey, poll.poll_start, poll.poll_end, &voting::ID)
        else {
            continue;
        };

        let mut candidates: Vec<CandidateInfo> = candidate_map
            .iter()
            .filter_map(|(cand_pubkey, candidate)| {
                let (expected, _) = find_pda_with_id_and_name(
                    poll_id,
                    &candidate.candidate_name,
                    &voting::ID,
                );
                if expected == *cand_pubkey {
                    Some(CandidateInfo {
                        candidate_name: candidate.candidate_name.clone(),
                        votes: candidate.candiate_votes as u64,
                    })
                } else {
                    None
                }
            })
            .collect();

        candidates.sort_by(|a, b| b.votes.cmp(&a.votes));

        polls.push(PollInfo {
            poll_name: poll.poll_name,
            poll_description: poll.poll_description,
            poll_start: format_timestamp(poll.poll_start),
            poll_end: format_timestamp(poll.poll_end),
            poll_end_ts: poll.poll_end,
            poll_start_ts: poll.poll_start,
            address: poll_pubkey.to_string(),
            is_active: !is_expired(poll.poll_end),
            candidates,
        });
    }

    Ok(HtmlTemplate(PollsTemplate { polls }))
}

async fn vote_handler(
    Query(form): Query<VoteForm>,
) -> Result<impl IntoResponse, AppError> {
    let client  = make_client()?;
    let program = client.program::<voting::program::SfBcOne>(voting::ID)?;

    let poll_id: i32 = (form.poll_start_ts % i32::MAX as i64) as i32;

    let (poll_pda, _) = find_pda_with_id(b"poll", poll_id, &voting::ID);
    let expected = form.poll_address.parse::<Pubkey>().map_err(AppError::from_err)?;
    solana_askama_kit::pda::verify_pda(poll_pda, expected)
        .map_err(|e| AppError::other(e.to_string()))?;

    let (candidate_pda, _) = find_pda_with_id_and_name(poll_id, &form.candidate_name, &voting::ID);

    let vote_ix = program
        .request()
        .accounts(accounts::Vote {
            signer: program.payer(),
            poll_acc: poll_pda,
            candidate_acc: candidate_pda,
        })
        .args(args::Vote {
            poll_id,
            input_candidate_name: form.candidate_name.clone(),
        })
        .instructions()
        .remove(0);

    program
        .request()
        .instruction(vote_ix)
        .send()
        .await
        .map_err(AppError::from_err)?;

    Ok(axum::response::Redirect::to("/polls"))
}





// //! Voting dApp — example showing how to use `solana-askama-kit`.
// //!
// //! Compare this to the original `main.rs`:
// //!  - No manual HtmlTemplate impl (kit provides it)
// //!  - No manual keypair loading (kit handles env var + default path)
// //!  - No manual PDA math (kit's `pda` module)
// //!  - No manual timestamp parsing (kit's `time` module)
// //!  - No boilerplate error type (kit's AppError)

// use std::collections::HashMap;

// use anchor_client::Cluster;
// use anchor_lang::{AccountDeserialize, Discriminator, system_program};
// use askama::Template;
// use axum::{Form, extract::Query, response::IntoResponse};
// use serde::Deserialize;
// use solana_sdk::pubkey::Pubkey;

// use solana_askama_kit::{
//     AppError, HtmlTemplate, SolanaClient,
//     pda::{find_pda_with_id, find_pda_with_id_and_name, recover_poll_id},
//     router::{AppRouter, init_tracing, serve},
//     time::{format_timestamp, is_expired, parse_datetime_local},
// };

// // bring your generated program types into scope
// declare_program!(voting);
// use voting::{client::accounts, client::args};

// // ── Templates ────────────────────────────────────────────────────────────────

// #[derive(Template)]
// #[template(path = "form.html")]
// struct FormTemplate {}

// #[derive(Template)]
// #[template(path = "poll.html")]
// struct PollTemplate {
//     poll_name: String,
//     poll_description: String,
//     poll_start: String,
//     poll_end: String,
//     candidate1: String,
//     candidate2: String,
//     poll_signature: String,
//     poll_id: i32,
// }

// #[derive(Template)]
// #[template(path = "polls.html")]
// struct PollsTemplate {
//     polls: Vec<PollInfo>,
// }

// // ── View models ───────────────────────────────────────────────────────────────

// struct PollInfo {
//     poll_name: String,
//     poll_description: String,
//     poll_start: String,
//     poll_end: String,
//     poll_end_ts: i64,
//     poll_start_ts: i64,
//     address: String,
//     candidates: Vec<CandidateInfo>,
//     is_active: bool,
// }

// struct CandidateInfo {
//     candidate_name: String,
//     votes: u64,
// }

// // ── Form types ────────────────────────────────────────────────────────────────

// #[derive(Deserialize)]
// struct PollForm {
//     poll_name: String,
//     poll_description: String,
//     poll_start: String,
//     poll_end: String,
//     candidate1: String,
//     candidate2: String,
// }

// #[derive(Deserialize)]
// struct VoteQuery {
//     poll_address: String,
//     poll_start_ts: i64,
//     candidate_name: String,
// }

// // ── Entry point ───────────────────────────────────────────────────────────────

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     init_tracing();

//     let app = AppRouter::new()
//         .get("/", show_form)
//         .post("/init", initialize_poll)
//         .get("/polls", show_polls)
//         .get("/vote", vote)
//         .build();

//     serve(app, "127.0.0.1:3000").await
// }

// // ── Handlers ──────────────────────────────────────────────────────────────────

// async fn show_form() -> impl IntoResponse {
//     HtmlTemplate(FormTemplate {})
// }

// async fn initialize_poll(
//     Form(form): Form<PollForm>,
// ) -> Result<impl IntoResponse, AppError> {
//     // Parse timestamps — kit helper, no manual NaiveDateTime boilerplate
//     let start_ts = parse_datetime_local(&form.poll_start).map_err(AppError::other)?;
//     let end_ts   = parse_datetime_local(&form.poll_end).map_err(AppError::other)?;

//     // Build client — kit reads SOLANA_KEYPAIR env var or ~/.config/solana/id.json
//     let client  = SolanaClient::new(Cluster::Devnet)?;
//     let program = client.program(voting::ID)?;

//     let poll_id: i32 = (start_ts % i32::MAX as i64) as i32;

//     // PDA derivation — kit helper
//     let (poll_pda, _)       = find_pda_with_id(b"poll", poll_id, &voting::ID);
//     let (candidate1_pda, _) = find_pda_with_id_and_name(poll_id, &form.candidate1, &voting::ID);
//     let (candidate2_pda, _) = find_pda_with_id_and_name(poll_id, &form.candidate2, &voting::ID);

//     // 1. Init poll
//     let poll_ix = program
//         .request()
//         .accounts(accounts::InitializePoll {
//             signer: program.payer(),
//             poll_acc: poll_pda,
//             system_program: system_program::ID,
//         })
//         .args(args::InitializePoll {
//             poll_id,
//             input_name: form.poll_name.clone(),
//             input_description: form.poll_description.clone(),
//             input_start: start_ts,
//             input_end: end_ts,
//         })
//         .instructions()
//         .remove(0);

//     let poll_sig = program.request().instruction(poll_ix).send().await?;

//     // 2. Init candidate 1
//     let c1_ix = program
//         .request()
//         .accounts(accounts::InitializeCandidate {
//             signer: program.payer(),
//             poll_acc: poll_pda,
//             candidate_acc: candidate1_pda,
//             system_program: system_program::ID,
//         })
//         .args(args::InitializeCandidate {
//             poll_id,
//             input_candidate_name: form.candidate1.clone(),
//         })
//         .instructions()
//         .remove(0);

//     program.request().instruction(c1_ix).send().await?;

//     // 3. Init candidate 2
//     let c2_ix = program
//         .request()
//         .accounts(accounts::InitializeCandidate {
//             signer: program.payer(),
//             poll_acc: poll_pda,
//             candidate_acc: candidate2_pda,
//             system_program: system_program::ID,
//         })
//         .args(args::InitializeCandidate {
//             poll_id,
//             input_candidate_name: form.candidate2.clone(),
//         })
//         .instructions()
//         .remove(0);

//     program.request().instruction(c2_ix).send().await?;

//     Ok(HtmlTemplate(PollTemplate {
//         poll_name: form.poll_name,
//         poll_description: form.poll_description,
//         poll_start: format_timestamp(start_ts),
//         poll_end: format_timestamp(end_ts),
//         candidate1: form.candidate1,
//         candidate2: form.candidate2,
//         poll_signature: poll_sig.to_string(),
//         poll_id,
//     }))
// }


// async fn show_polls() -> Result<impl IntoResponse, AppError> {
//     let client = SolanaClient::new(Cluster::Devnet)?;

//     let all_accounts = client
//         .rpc
//         .get_program_accounts(&voting::ID)
//         .map_err(|e| AppError::Rpc(e.to_string()))?;

//     let poll_disc = voting::accounts::PollAccount::DISCRIMINATOR;
//     let cand_disc = voting::accounts::CandidateAccount::DISCRIMINATOR;

//     // Collect candidates into a map
//     let mut candidate_map: HashMap<Pubkey, voting::accounts::CandidateAccount> = HashMap::new();
//     for (pubkey, account) in &all_accounts {
//         if account.data.len() >= 8 && account.data[..8] == *cand_disc {
//             let mut data: &[u8] = &account.data;
//             if let Ok(c) = voting::accounts::CandidateAccount::try_deserialize(&mut data) {
//                 candidate_map.insert(*pubkey, c);
//             }
//         }
//     }

//     let mut polls = vec![];
//     for (poll_pubkey, account) in &all_accounts {
//         if account.data.len() < 8 || account.data[..8] != *poll_disc {
//             continue;
//         }

//         let mut data: &[u8] = &account.data;
//         let Ok(poll) = voting::accounts::PollAccount::try_deserialize(&mut data) else {
//             continue;
//         };

//         // Recover the poll_id — kit helper handles both start/end fallback
//         let Some(poll_id) = recover_poll_id(poll_pubkey, poll.poll_start, poll.poll_end, &voting::ID)
//         else {
//             tracing::warn!("Could not recover poll_id for {}", poll_pubkey);
//             continue;
//         };

//         // Match candidates by PDA
//         let mut candidates: Vec<CandidateInfo> = candidate_map
//             .iter()
//             .filter_map(|(cand_pubkey, candidate)| {
//                 let (expected, _) = find_pda_with_id_and_name(poll_id, &candidate.candidate_name, &voting::ID);
//                 if expected == *cand_pubkey {
//                     Some(CandidateInfo {
//                         candidate_name: candidate.candidate_name.clone(),
//                         votes: candidate.candiate_votes as u64,
//                     })
//                 } else {
//                     None
//                 }
//             })
//             .collect();

//         // Sort by votes descending for display
//         candidates.sort_by(|a, b| b.votes.cmp(&a.votes));

//         polls.push(PollInfo {
//             poll_name: poll.poll_name,
//             poll_description: poll.poll_description,
//             poll_start: format_timestamp(poll.poll_start),
//             poll_end: format_timestamp(poll.poll_end),
//             poll_end_ts: poll.poll_end,
//             poll_start_ts: poll.poll_start,
//             address: poll_pubkey.to_string(),
//             is_active: !is_expired(poll.poll_end),
//             candidates,
//         });
//     }

//     Ok(HtmlTemplate(PollsTemplate { polls }))
// }


// async fn vote(
//     Query(form): Query<VoteQuery>,
// ) -> Result<impl IntoResponse, AppError> {
//     let client  = SolanaClient::new(Cluster::Devnet)?;
//     let program = client.program(voting::ID)?;

//     let poll_id: i32 = (form.poll_start_ts % i32::MAX as i64) as i32;

//     // Verify PDA matches the address from the form — kit helper
//     let (poll_pda, _) = find_pda_with_id(b"poll", poll_id, &voting::ID);
//     let expected = form.poll_address.parse::<Pubkey>().map_err(AppError::from_err)?;
//     solana_askama_kit::pda::verify_pda(poll_pda, expected)
//         .map_err(|e| AppError::other(e.to_string()))?;

//     let (candidate_pda, _) = find_pda_with_id_and_name(poll_id, &form.candidate_name, &voting::ID);

//     let vote_ix = program
//         .request()
//         .accounts(accounts::Vote {
//             signer: program.payer(),
//             poll_acc: poll_pda,
//             candidate_acc: candidate_pda,
//         })
//         .args(args::Vote {
//             poll_id,
//             input_candidate_name: form.candidate_name.clone(),
//         })
//         .instructions()
//         .remove(0);

//     program
//         .request()
//         .instruction(vote_ix)
//         .send()
//         .await
//         .map_err(AppError::from_err)?;

//     Ok(axum::response::Redirect::to("/polls"))
// }
