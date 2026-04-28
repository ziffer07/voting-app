
use {
    anchor_lang::{InstructionData, ToAccountMetas, prelude::{Clock, system_program}, solana_program::instruction::Instruction}, 
    litesvm::LiteSVM, 
    solana_keypair::Keypair, 
    solana_message::{Message, VersionedMessage}, 
    solana_signer::Signer, 
    solana_transaction::versioned::VersionedTransaction
};

use solana_sdk::pubkey::Pubkey;

#[test]
fn test_initialize() {
    let program_id = sf_bc_one::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/sf_bc_one.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let clock = Clock {
        slot: 1000,
        epoch_start_timestamp: 0,
        epoch: 1,
        leader_schedule_epoch: 1,
        unix_timestamp: 1000,
    };
    svm.set_sysvar(&clock);

    let poll_id: i32 = 1;
    let start_time: i64 = 0;
    let end_time: i64 = 1877309408;
    let poll_name = "Who is better?".to_string();
    let poll_description = "Testing the poll to see if it works! Let go CR7".to_string();



    let (poll_pda, _bump) = Pubkey::find_program_address(
        &[b"poll", &poll_id.to_le_bytes()],
        &program_id,
    );

    // let (candidate_pda, bump) = Pubkey::find_program_address(
    //     &[&poll_id.to_le_bytes(), candidate.as_bytes()], 
    //     &program_id
    // );

    
    let instruction = Instruction::new_with_bytes(
        program_id,
        &sf_bc_one::instruction::InitializePoll {
            poll_id: poll_id,
            input_name: poll_name.clone(),
            input_description: poll_description,
            input_start: start_time,
            input_end: end_time,
        }.data(),
        sf_bc_one::accounts::InitializePoll {
            signer: payer.pubkey(),
            poll_acc: poll_pda,
            system_program: system_program::ID,
        }.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();

    let res = svm.send_transaction(tx);


    // let poll_account = svm.get_account(&poll_pda).unwrap();
    // let poll_data = &poll_account.data;
    // let poll: sf_bc_one::PollAccount = anchor_lang::AccountDeserialize::try_deserialize(
    //     &mut &poll_data[..]
    // ).unwrap();


    // println!("Poll Name {}", poll.poll_name);
    // println!("Poll desc {}", poll.poll_description);
    // println!("Poll end {}", poll.poll_end);
    // assert_eq!(poll.poll_name, poll_name);
    // assert_eq!(poll.poll_start, start_time);
    assert!(res.is_ok());
}

#[test]
fn test_candidate() {
    let program_id = sf_bc_one::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/sf_bc_one.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let clock = Clock {
        slot: 1000,
        epoch_start_timestamp: 0,
        epoch: 1,
        leader_schedule_epoch: 1,
        unix_timestamp: 1000,
    };
    svm.set_sysvar(&clock);

    let poll_id: i32 = 1;
    let start_time: i64 = 0;
    let end_time: i64 = 1877309408;
    let poll_name = "Who is better?".to_string();
    let poll_description = "Testing the poll to see if it works! Let go CR7".to_string();

    let (poll_pda, _bump) = Pubkey::find_program_address(
        &[b"poll", &poll_id.to_le_bytes()],
        &program_id,
    );
    
    let instruction = Instruction::new_with_bytes(
        program_id,
        &sf_bc_one::instruction::InitializePoll {
            poll_id: poll_id,
            input_name: poll_name.clone(),
            input_description: poll_description,
            input_start: start_time,
            input_end: end_time,
        }.data(),
        sf_bc_one::accounts::InitializePoll {
            signer: payer.pubkey(),
            poll_acc: poll_pda,
            system_program: system_program::ID,
        }.to_account_metas(None),
    );

    let candidate_name_one = "Ronaldo".to_string();
    let (candidate_pda, _bump) = Pubkey::find_program_address(
        &[&poll_id.to_le_bytes(), candidate_name_one.as_bytes()], 
        &program_id
    );

    let candidate_ix = Instruction::new_with_bytes(
        program_id,
        &sf_bc_one::instruction::InitializeCandidate {
            poll_id: poll_id,
            input_candidate_name: candidate_name_one.clone(),
        }.data(),
        sf_bc_one::accounts::InitializeCandidate {
            signer: payer.pubkey(),
            poll_acc: poll_pda,
            candidate_acc: candidate_pda,
            system_program: system_program::ID,
        }.to_account_metas(None),
    );


    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction, candidate_ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();

    let res = svm.send_transaction(tx);

    let poll_accnt = svm.get_account(&poll_pda).unwrap();
    let poll_data = &poll_accnt.data;
    let poll: sf_bc_one::PollAccount = anchor_lang::AccountDeserialize::try_deserialize(
        &mut &poll_data[..]
    ).unwrap();

    let candidate_accnt = svm .get_account(&candidate_pda).unwrap();
    let candidate_data = &candidate_accnt.data;
    let candidate: sf_bc_one::CandidateAccount = anchor_lang::AccountDeserialize::try_deserialize(
        &mut &candidate_data[..]
    ).unwrap();


    println!("Total Canididates: {}, Candidate 1 name: {}, Candidate 1 Votes: {}", poll.candidate_amount, candidate.candidate_name, candidate.candiate_votes);

    assert!(res.is_ok());
}
