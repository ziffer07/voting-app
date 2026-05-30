use std::{sync::Arc};
use anchor_client::{anchor_lang,Client, Cluster, CommitmentConfig as SolanaCommitmentConfig};  // brings anchor_lang into root scope
use anchor_lang::{AccountDeserialize, Discriminator, declare_program, system_program};
//use anyhow::Ok;
use askama::Template;
use axum::{Form, Router, extract::Query, http::StatusCode, response::{Html, IntoResponse, Response}, routing::{get, post}};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;
use solana_rpc_client::rpc_client::RpcClient;
//use solana_rpc_client_api::{config::{RpcAccountInfoConfig, RpcProgramAccountsConfig}, filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType}, response::UiAccountData};
use solana_sdk::{pubkey::Pubkey, signature::read_keypair_file};

declare_program!(voting);
use ::thiserror::Error;
use voting::{client::accounts, client::args};

//use crate::voting::accounts::PollAccount;


// ----------------------------------- Webserver code ------------------------------------------------------//
struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T> 
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", error),
            )
            .into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "form.html")]
struct FormTemplate {}

#[derive(Template)]
#[template(path="poll.html")]
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
#[template(path="server-error.html")]
struct ServerErrorTemplate{}


#[derive(Deserialize)]
struct PollForm {
    poll_name: String,
    poll_description: String,
    poll_start: String, // datetime-local gives "2025-01-01T10:00"
    poll_end: String,
    candidate1: String,
    candidate2: String,
}

#[derive(Template)]
#[template(path="polls.html")]
struct PollsTemplate {
    polls: Vec<PollInfo>,
}

struct PollInfo {
    poll_name: String,
    poll_description: String,
    poll_start: String,
    poll_end: String,
    poll_end_ts: i64,
    poll_start_ts: i64,
    address: String,
    candidates: Vec<CandidateInfo>,
}

struct CandidateInfo {
    candidate_name: String,
    votes: u64,
}


#[derive(Deserialize)]
struct VoteForm {
    poll_address: String,
    poll_start_ts: i64,
    candidate_name: String,
}


// ── Error handling ───────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Template error: {0}")]
    Template(#[from] askama::Error),
    #[error("Anchor client error: {0}")]
    AnchorClient(#[from] anchor_client::ClientError),
    #[error("Other error: {0}")]
    Other(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let html = ServerErrorTemplate {}.render().unwrap_or_default();
        (StatusCode::INTERNAL_SERVER_ERROR, Html(html)).into_response()
    }
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(show_form_handler))
        .route("/init", post(initialize_handler))
        .route("/polls", get(show_polls_handler))
        .route("/vote", get(vote_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;
    Ok(())
}


async fn show_form_handler() -> impl IntoResponse {
    HtmlTemplate(FormTemplate {})
}



async fn initialize_handler(
    Form(form): Form<PollForm>,
) -> Result<impl IntoResponse, AppError> {

    // Convert datetime-local string to Unix timestamp
    let start_ts = parse_datetime(&form.poll_start)
        .map_err(|e| AppError::Other(e.to_string())).unwrap();
    let end_ts = parse_datetime(&form.poll_end)
        .map_err(|e| AppError::Other(e.to_string())).unwrap();

    let payer = read_keypair_file("/home/aryan/.config/solana/id.json")
        .expect("Failed to read keypair file");

    let provider = Client::new_with_options(
        Cluster::Devnet,
        Arc::new(payer),
        SolanaCommitmentConfig::confirmed(),
    );

    let program = provider.program(voting::ID)?;
    let program_id = voting::ID;

    // Use timestamp as poll_id so each poll is unique
    let poll_id: i32 = (start_ts % i32::MAX as i64) as i32;

    let (poll_pda, _) = Pubkey::find_program_address(
        &[b"poll", &poll_id.to_le_bytes()],
        &program_id,
    );

    // 1. Initialize poll
    let init_poll_ix = program
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

    let poll_sig = program
        .request()
        .instruction(init_poll_ix)
        .send()
        .await.unwrap();

    // 2. Initialize candidate 1
    let (candidate1_pda, _) = Pubkey::find_program_address(
        &[&poll_id.to_le_bytes(), form.candidate1.as_bytes()],
        &program_id,
    );

    let init_c1_ix = program
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

    let c1_sig = program
        .request()
        .instruction(init_c1_ix)
        .send()
        .await.unwrap();

    // 3. Initialize candidate 2
    let (candidate2_pda, _) = Pubkey::find_program_address(
        &[&poll_id.to_le_bytes(), form.candidate2.as_bytes()],
        &program_id,
    );

    let init_c2_ix = program
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

    let c2_sig = program
        .request()
        .instruction(init_c2_ix)
        .send()
        .await.unwrap();
    

    // 4. Show the poll tile
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


fn parse_datetime(s: &str) -> Result<i64, String> {
    // datetime-local format: "2025-01-01T10:00"
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).timestamp())
        .map_err(|e| format!("Invalid date: {}", e))
}

fn format_timestamp(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| ts.to_string())
}


async fn show_polls_handler() -> Result<impl IntoResponse, AppError> {
    let connection = RpcClient::new_with_commitment(
        "https://api.devnet.solana.com",
        SolanaCommitmentConfig::confirmed(),
    );

    // use solana_sdk::hash;
    // let discriminator = hash::hash("account:PollAccount".as_bytes()).to_bytes();
    // let discriminator = &discriminator[..8];

    // let config = RpcProgramAccountsConfig {
    //     filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new(
    //         0,
    //         MemcmpEncodedBytes::Bytes(discriminator.to_vec()),
    //     ))]),
    //     account_config: RpcAccountInfoConfig {
    //         commitment: Some(SolanaCommitmentConfig::confirmed()),
    //         ..Default::default()
    //     },
    //     ..Default::default()
    // };

    let program_id = voting::ID;

    let all_accounts = connection
        .get_program_accounts(&voting::ID)
        .map_err(|e| AppError::Other(e.to_string()))?;


    let poll_disc = voting::accounts::PollAccount::DISCRIMINATOR;
    let cand_disc = voting::accounts::CandidateAccount::DISCRIMINATOR;

    // collecting candidates
    let mut candidate_map: std::collections::HashMap<Pubkey, voting::accounts::CandidateAccount> = std::collections::HashMap::new();

    for (pubkey, account) in &all_accounts {
        if account.data.len() >= 8 && account.data[0..8] == *cand_disc {
            let mut data: &[u8] = &account.data;
            if let Ok(c) = voting::accounts::CandidateAccount::try_deserialize(&mut data) {
                candidate_map.insert(*pubkey, c);
            }
        }
    }

    let mut polls = vec![];
    for (poll_pubkey, account) in &all_accounts {
        if account.data.len() >= 8 && account.data[0..8] == *poll_disc {
            
            // 2. CHANGE THIS LINE: Use AccountDeserialize instead of standard deserialize
            let mut data_slice: &[u8] = &account.data;
            if let Ok(poll) = voting::accounts::PollAccount::try_deserialize(&mut data_slice) {


                let poll_id = (poll.poll_start % i32::MAX as i64) as i32;
                let poll_id_bytes = poll_id.to_le_bytes();

                let (expected_poll_pda, _bump) = Pubkey::find_program_address(
                    &[b"poll", &poll_id_bytes.as_ref()], 
                    &program_id,
                );

                let verified_poll_id_bytes = match expected_poll_pda == *poll_pubkey {
                    true => Some(poll_id_bytes),
                    false => {
                        let poll_id_alt = (poll.poll_end % i32::MAX as i64) as i32;
                        let poll_id_alt_bytes = poll_id_alt.to_le_bytes();
                        let (alt_pda, _) = Pubkey::find_program_address(
                            &[b"poll", &poll_id_alt_bytes],
                            &program_id,
                        );
                        match alt_pda == *poll_pubkey {
                            true => Some(poll_id_alt_bytes),
                            false => None,   // can't recover poll_id
                        }
                    }
                };

                 // Match candidates using recovered poll_id bytes
                let mut candidates = vec![];
                if let Some(id_bytes) = verified_poll_id_bytes {  // ← unwrap here
                    for (cand_pubkey, candidate) in &candidate_map {
                        let (expected_cand_pda, _) = Pubkey::find_program_address(
                            &[&id_bytes, candidate.candidate_name.as_bytes()],  // ← id_bytes
                            &program_id,
                        );
                        if expected_cand_pda == *cand_pubkey {
                            candidates.push(CandidateInfo {
                                candidate_name: candidate.candidate_name.clone(),
                                votes: candidate.candiate_votes as u64,
                            });
                        }
                    }
                }
                
                polls.push(PollInfo {
                    poll_name: poll.poll_name,
                    poll_description: poll.poll_description,
                    poll_start: format_timestamp(poll.poll_start),
                    poll_end: format_timestamp(poll.poll_end),
                    poll_end_ts: poll.poll_end,
                    poll_start_ts: poll.poll_start,
                    address: poll_pubkey.to_string(),
                    candidates,
                });
            }
        }
    }

    // let mut polls = vec![];
    // for (pubkey, account) in accounts {
    //     if let Ok(poll) = voting::accounts::PollAccount::deserialize(
    //         &mut &account.data[8..]
    //     ) {
    //         polls.push(PollInfo {
    //             poll_name: poll.poll_name,
    //             poll_description: poll.poll_description,
    //             poll_start: format_timestamp(poll.poll_start),
    //             poll_end: format_timestamp(poll.poll_end),
    //             address: pubkey.to_string(),
    //         });
    //     }
    // }

    
    // let mut polls = vec![];
    // for (pubkey, account) in accounts {
    //     let raw_bytes = account.data;
    //     // Anchor discriminator check
    //     if raw_bytes.len() < 8 {
    //         continue;
    //     }
    //     if let Ok(poll) = voting::accounts::PollAccount::deserialize(&mut &raw_bytes[8..]){
    //         polls.push(PollInfo {
    //             poll_name: poll.poll_name,
    //             poll_description: poll.poll_description,
    //             poll_start: format_timestamp(poll.poll_start),
    //             poll_end: format_timestamp(poll.poll_end),
    //             address: pubkey.to_string(),
    //         });
    //     }
    // }

    Ok(HtmlTemplate(PollsTemplate { polls }))
}


async fn vote_handler(
    Query(form): Query<VoteForm>,
) -> Result<impl IntoResponse, AppError> {
    let payer = read_keypair_file("/home/aryan/.config/solana/id.json")
        .expect("Failed to read keypair file");

    let provider = Client::new_with_options(
        Cluster::Devnet,
        Arc::new(payer),
        SolanaCommitmentConfig::confirmed(),
    );

    let program = provider.program(voting::ID)?;
    let program_id = voting::ID;

    // Recover poll_id from poll_start_ts — same formula as initialize_handler
    let poll_id: i32 = (form.poll_start_ts % i32::MAX as i64) as i32;

    let (poll_pda, _) = Pubkey::find_program_address(
        &[b"poll", &poll_id.to_le_bytes()],
        &program_id,
    );

    // Verify recovered PDA matches the one sent from the form
    let expected_address = form.poll_address.parse::<Pubkey>()
        .map_err(|e| AppError::Other(e.to_string()))?;

    if poll_pda != expected_address {
        return Err(AppError::Other("Poll PDA mismatch".to_string()));
    }

    let (candidate_pda, _) = Pubkey::find_program_address(
        &[&poll_id.to_le_bytes(), form.candidate_name.as_bytes()],
        &program_id,
    );

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

    let signature = program
        .request()
        .instruction(vote_ix)
        .send()
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;


    // Redirect back to polls page after voting
    Ok(axum::response::Redirect::to("/polls"))
}



// async fn show_poll_handler() -> impl IntoResponse {
//     let payer = read_keypair_file("/home/aryan/.config/solana/id.json")
//          .expect("Failed to read keypair file");

//     let provider = Client::new_with_options(
//         Cluster::Devnet,
//         Arc::new(payer), // Rc::new(payer) doesn't work for axum tokio
//         CommitmentConfig::confirmed(),
//     );

//     let program = provider.program(voting::ID).unwrap();
//     let program_id = voting::ID;

//     let poll_id:i32 = 2;

//     let (poll_pda, _bump) = Pubkey::find_program_address(
//         &[b"poll", &poll_id.to_le_bytes()],
//         &program_id,
//     );

//     let poll: PollAccount = program.account::<PollAccount>(poll_pda).await.unwrap();

//     let template = PollTemplate {
//         poll_name: poll.poll_name,
//         poll_description: poll.poll_description,
//         poll_start: poll.poll_start.to_string(),
//         poll_end: poll.poll_end.to_string(),
//     };

//     HtmlTemplate(template)
    
// }

















// The code below works isude tokio::main block
// let connection = RpcClient::new_with_commitment(
//         "https://api.devnet.solana.com",
//         CommitmentConfig::confirmed(),
//     );

//     // Load existing funded devnet wallet
//    let payer = read_keypair_file("/home/aryan/.config/solana/id.json")
//     .expect("Failed to read keypair file");

//     println!("Payer: {}", payer.pubkey());
//     println!("Balance: {} lamports", connection.get_balance(&payer.pubkey())?);

//     let provider = Client::new_with_options(
//         Cluster::Devnet,
//         Rc::new(payer),
//         CommitmentConfig::confirmed(),
//     );

//     let program = provider.program(voting::ID)?;
//     let program_id = voting::ID;

//     let poll_id: i32 = 2;
//     let start_time: i64 = 0;
//     let end_time: i64 = 1877309408;
//     let poll_name = "Who is better?".to_string();
//     let poll_description = "Testing the poll to see if it works!".to_string();

//     let (poll_pda, _bump) = Pubkey::find_program_address(
//         &[b"poll", &poll_id.to_le_bytes()],
//         &program_id,
//     );

//     let initialize_ix = program
//         .request()
//         .accounts(accounts::InitializePoll {
//             signer: program.payer(),
//             poll_acc: poll_pda,
//             system_program: system_program::ID,
//         })
//         .args(args::InitializePoll {
//             poll_id,
//             input_name: poll_name,
//             input_description: poll_description,
//             input_start: start_time,
//             input_end: end_time,
//         })
//         .instructions()
//         .remove(0);

//     let signature = program
//         .request()
//         .instruction(initialize_ix)
//         .send()
//         .await?;

//     println!("Transaction signature: {}", signature);

//     println!("\nFetch poll account data");
//     let poll_account: PollAccount = program.account::<PollAccount>(poll_pda).await?;
//     println!("   Poll name: {}", poll_account.poll_name);
//     Ok(())




// Code below is for localnet which I tried to run for devnet and didn't work


// use std::{rc::Rc, sync::Arc};

// use anchor_client::{Client, Cluster, CommitmentConfig, Signer};
// use anchor_lang::{prelude::*};
// use anyhow::Ok;
// use solana_rpc_client::rpc_client::RpcClient;


// declare_program!(voting);
// use solana_sdk::{native_token::LAMPORTS_PER_SOL, signature::{read_keypair_file}, pubkey::Pubkey};
// use voting::{accounts::PollAccount, client::accounts, client::args};



// #[tokio::main]

// async fn main() -> anyhow::Result<()> {
//     let connection = RpcClient::new_with_commitment(
//         "https://api.devnet.solana.com", 
//         CommitmentConfig::confirmed()
//     );


//     let home = std::env::var("HOME")?;
//     let payer_path = format!("{}/.config/solana/id.json", home);
//     let payer = read_keypair_file(&payer_path)
//         .expect(&format!("Failed to read keypair file at {}", payer_path));

//     //let payer = Keypair::new();
//     //let poll = Keypair::new();

//     println!("Generated Keypairs:");
//     println!("Payer: {}", payer.pubkey());
//     //println!("Poll: {}", poll.pubkey());

//     println!("\nRequesting 1 SOL airdrop to payer");
//     let airdrop_signature = connection.request_airdrop(&payer.pubkey(), LAMPORTS_PER_SOL)?;
 
//     // Wait for airdrop confirmation
//     while !connection.confirm_transaction(&airdrop_signature)? {
//         std::thread::sleep(std::time::Duration::from_millis(100));
//     }
//     println!("Airdrop confirmed!");

//     // Clone BEFORE moving payer into Rc
//     //let payer_clone = Keypair::new_from_array(*payer.secret_bytes());

//     // Use Arc so it can be shared with 'static lifetime
//     let payer = Arc::new(payer);
//     //let payer_clone = Arc::clone(&payer);

//     // Create program client
//     let provider = Client::new_with_options(
//         Cluster::Devnet,
//         Rc::new(payer),
//         CommitmentConfig::confirmed(),
//     );
//     let program = provider.program(voting::ID)?;
//     let program_id = voting::ID;
 
//     // Build and send instructions
//     println!("\nSend transaction with initialize and increment instructions");

//     let poll_id: i32 = 1;
//     let start_time: i64 = 0;
//     let end_time: i64 = 1877309408;
//     let poll_name = "Who is better?".to_string();
//     let poll_description = "Testing the poll to see if it works! Let go CR7".to_string();

//     let (poll_pda, _bump) = Pubkey::find_program_address(
//         &[b"poll", &poll_id.to_le_bytes()],
//         &program_id,
//     );

//     let initialize_ix = program
//         .request()
//         .accounts(accounts::InitializePoll{
//             signer: program.payer(),
//             poll_acc: poll_pda,
//             system_program: system_program::ID,
//         })
//         .args(args::InitializePoll{
//             poll_id: poll_id,
//             input_name: poll_name,
//             input_description: poll_description,
//             input_start: start_time,
//             input_end: end_time,
//         })
//         .instructions()
//         .remove(0);

//     let signature = program
//         .request()
//         .instruction(initialize_ix)
//         //.signer(payer_clone)
//         .send()
//         .await?;

//     println!("   Transaction confirmed: {}", signature);
 
//     println!("\nFetch poll account data");
//     let poll_account: PollAccount = program.account::<PollAccount>(poll_pda).await?;
//     println!("   Poll name: {}", poll_account.poll_name);

//     Ok(())
// }