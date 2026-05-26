use std::rc::Rc;
use anchor_client::{Client, Cluster, CommitmentConfig, Signer};
use anchor_lang::prelude::*;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::read_keypair_file};

declare_program!(voting);
use voting::{client::accounts, client::args};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let connection = RpcClient::new_with_commitment(
        "https://api.devnet.solana.com",
        CommitmentConfig::confirmed(),
    );

    // Load existing funded devnet wallet
   let payer = read_keypair_file("/home/aryan/.config/solana/id.json")
    .expect("Failed to read keypair file");

    println!("Payer: {}", payer.pubkey());
    println!("Balance: {} lamports", connection.get_balance(&payer.pubkey())?);

    let provider = Client::new_with_options(
        Cluster::Devnet,
        Rc::new(payer),
        CommitmentConfig::confirmed(),
    );

    let program = provider.program(voting::ID)?;
    let program_id = voting::ID;

    let poll_id: i32 = 1;
    let start_time: i64 = 0;
    let end_time: i64 = 1877309408;
    let poll_name = "Who is better?".to_string();
    let poll_description = "Testing the poll to see if it works!".to_string();

    let (poll_pda, _bump) = Pubkey::find_program_address(
        &[b"poll", &poll_id.to_le_bytes()],
        &program_id,
    );

    let initialize_ix = program
        .request()
        .accounts(accounts::InitializePoll {
            signer: program.payer(),
            poll_acc: poll_pda,
            system_program: system_program::ID,
        })
        .args(args::InitializePoll {
            poll_id,
            input_name: poll_name,
            input_description: poll_description,
            input_start: start_time,
            input_end: end_time,
        })
        .instructions()
        .remove(0);

    let signature = program
        .request()
        .instruction(initialize_ix)
        .send()
        .await?;

    println!("Transaction signature: {}", signature);
    Ok(())
}




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