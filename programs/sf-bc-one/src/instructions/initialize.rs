use anchor_lang::prelude::*;

use crate::{CandidateAccount, PollAccount, error::ErrorCode};

#[derive(Accounts)]
#[instruction(poll_id: i32)]
pub struct InitializePoll<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = 8 + PollAccount::INIT_SPACE,
        seeds = [b"poll".as_ref(), poll_id.to_le_bytes().as_ref()],
        bump
    )]
    pub poll_acc: Account<'info, PollAccount>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(poll_id: i32, input_candidate: String)]
pub struct InitializeCandidate<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"poll".as_ref(), poll_id.to_le_bytes().as_ref()],
        bump
    )]
    pub poll_acc: Account<'info, PollAccount>,
    #[account(
        init,
        payer = signer,
        space = 8 + PollAccount::INIT_SPACE,
        seeds = [poll_id.to_le_bytes().as_ref(), input_candidate.as_ref()],
        bump
    )]
    pub candidate_acc: Account<'info, CandidateAccount>,
    pub system_program: Program<'info, System>
}


#[derive(Accounts)]
#[instruction(poll_id: i32, input_candidate: String)]
pub struct VoteAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"poll".as_ref(), poll_id.to_le_bytes().as_ref()],
        bump
    )]
    pub poll_acc: Account<'info, PollAccount>,
    #[account(
        mut,
        seeds = [poll_id.to_le_bytes().as_ref(), input_candidate.as_ref()],
        bump
    )]
    pub candidate_acc: Account<'info, CandidateAccount>,
}


pub fn init_poll_handler(
    ctx: Context<InitializePoll>,
    _poll_id: i32,
    input_name: String,
    input_description: String,
    input_start: i64,
    input_end: i64,
) -> Result<()> {
    msg!("Greetings from: {:?}", ctx.program_id);
    let poll_acc = &mut ctx.accounts.poll_acc;
    poll_acc.poll_name = input_name;
    poll_acc.poll_description = input_description;
    poll_acc.poll_start = input_start;
    poll_acc.poll_end = input_end;
    Ok(())
}


pub fn init_candidate_handler(
    ctx: Context<InitializeCandidate>,
    _poll_id: i32,
    input_candidate_name: String,
) -> Result<()> {
    let candidate_acc = &mut ctx.accounts.candidate_acc;
    candidate_acc.candidate_name = input_candidate_name;
    ctx.accounts.poll_acc.candidate_amount += 1;
    Ok(())
}

pub fn vote_handler(
    ctx: Context<VoteAccount>,
    _poll_id: i32,
    _input_candidate_name: String,
) -> Result<()> {
    let poll_acc = &mut ctx.accounts.poll_acc;
    let candidate_acc = &mut ctx.accounts.candidate_acc;
    let current_time = Clock::get()?.unix_timestamp;

    if current_time > poll_acc.poll_end {
        return Err(ErrorCode::VotingEnded.into());
    }

    if current_time < poll_acc.poll_start {
        return Err(ErrorCode::VotingNotStarted.into());
    }

    candidate_acc.candiate_votes += 1;

    Ok(())
}
