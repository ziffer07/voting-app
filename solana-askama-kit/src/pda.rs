//! Program Derived Address (PDA) helpers.
//!
//! These utilities reduce boilerplate around `Pubkey::find_program_address`
//! and provide typed seed builders for common patterns.

use solana_sdk::pubkey::Pubkey;

/// Derive a PDA from a string prefix and a `i32` ID.
///
/// This is the pattern used for poll accounts: seed = `[b"poll", poll_id.to_le_bytes()]`.
///
/// # Example
/// ```rust,no_run
/// use solana_askama_kit::pda::find_pda_with_id;
/// let (pda, bump) = find_pda_with_id(b"poll", 42, &program_id);
/// ```
pub fn find_pda_with_id(prefix: &[u8], id: i32, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[prefix, &id.to_le_bytes()], program_id)
}

/// Derive a PDA from a `i32` ID and a string name.
///
/// This is the pattern used for candidate accounts: seed = `[poll_id.to_le_bytes(), name.as_bytes()]`.
///
/// # Example
/// ```rust,no_run
/// use solana_askama_kit::pda::find_pda_with_id_and_name;
/// let (pda, bump) = find_pda_with_id_and_name(42, "Alice", &program_id);
/// ```
pub fn find_pda_with_id_and_name(id: i32, name: &str, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&id.to_le_bytes(), name.as_bytes()], program_id)
}

/// Verify that a derived PDA matches an expected `Pubkey`.
///
/// Returns `Ok(pubkey)` on match, `Err(mismatch message)` otherwise.
///
/// # Example
/// ```rust,no_run
/// use solana_askama_kit::pda::{find_pda_with_id, verify_pda};
/// let (expected, _) = find_pda_with_id(b"poll", 42, &program_id);
/// verify_pda(expected, user_supplied_pubkey)?;
/// ```
pub fn verify_pda(expected: Pubkey, actual: Pubkey) -> Result<Pubkey, PdaError> {
    if expected == actual {
        Ok(actual)
    } else {
        Err(PdaError::Mismatch { expected, actual })
    }
}

/// Try to recover a poll `i32` ID from a known PDA address using the stored
/// `poll_start` timestamp, falling back to `poll_end`.
///
/// This is needed when fetching on-chain accounts that did not store their
/// seed ID explicitly (a known limitation of the voting example program).
///
/// Returns `None` if neither timestamp produces a matching PDA.
pub fn recover_poll_id(
    poll_pubkey: &Pubkey,
    poll_start: i64,
    poll_end: i64,
    program_id: &Pubkey,
) -> Option<i32> {
    let candidate_id = (poll_start % i32::MAX as i64) as i32;
    let (derived, _) =
        Pubkey::find_program_address(&[b"poll", &candidate_id.to_le_bytes()], program_id);
    if derived == *poll_pubkey {
        return Some(candidate_id);
    }

    let fallback_id = (poll_end % i32::MAX as i64) as i32;
    let (derived_fallback, _) =
        Pubkey::find_program_address(&[b"poll", &fallback_id.to_le_bytes()], program_id);
    if derived_fallback == *poll_pubkey {
        return Some(fallback_id);
    }

    None
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PdaError {
    #[error("PDA mismatch: expected {expected}, got {actual}")]
    Mismatch {
        expected: Pubkey,
        actual: Pubkey,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn find_and_verify_roundtrip() {
        let program_id = Pubkey::new_unique();
        let (pda, _) = find_pda_with_id(b"poll", 99, &program_id);
        assert!(verify_pda(pda, pda).is_ok());
    }

    #[test]
    fn verify_mismatch_errors() {
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        assert!(verify_pda(a, b).is_err());
    }
}
