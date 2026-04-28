pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("6Y7vUyZPsaK8MmgpxQ3aiFtxiai8ZVd3e55PFChp44Cy");

#[program]
pub mod sf_bc_one {
    use super::*;

    pub fn initialize_poll(
        ctx: Context<InitializePoll>,
        poll_id: i32,
        input_name: String,
        input_description: String,
        input_start: i64,
        input_end: i64,
    ) -> Result<()> {
        initialize::init_poll_handler(ctx, poll_id, input_name, input_description, input_start, input_end)
    }

    pub fn initialize_candidate(
        ctx: Context<InitializeCandidate>,
        poll_id: i32,
        input_candidate_name: String
    ) -> Result<()> {
        initialize::init_candidate_handler(ctx, poll_id, input_candidate_name)
    }

    pub fn vote(
        ctx: Context<VoteAccount>,
        poll_id: i32,
        input_candidate_name: String,
    ) -> Result<()> {
        initialize::vote_handler(ctx, poll_id, input_candidate_name)
    }
}


