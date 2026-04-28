use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct PollAccount {
    #[max_len(32)]
    pub poll_name: String,
    #[max_len(280)]
    pub poll_description: String,
    pub poll_start: i64,
    pub poll_end: i64,
    pub candidate_amount: i32,
}


#[account]
#[derive(InitSpace)]
pub struct CandidateAccount {
    #[max_len(50)]
    pub candidate_name: String,
    pub candiate_votes: i32,
}

