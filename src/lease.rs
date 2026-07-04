//! The run-level lease: one-writer-at-a-time enforcement (DESIGN § Run Lease
//! and Concurrent Writers).
//!
//! `run next --agent` issues or renews an agent-bound token; state-mutating
//! operations pass it back as `--lease`. A second live session gets
//! `lease_held`; an expired lease is cleared deterministically by the next
//! `run next`, so a crashed session never wedges the run.

use jiff::{Timestamp, ToSpan};
use serde::{Deserialize, Serialize};

use crate::ids;

/// MVP lease TTL: 10 minutes, overridable for tests via `SPECCY_LEASE_TTL_SECONDS`.
pub fn ttl_seconds() -> i64 {
    std::env::var("SPECCY_LEASE_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(600)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseState {
    pub token: String,
    pub agent: String,
    pub expires_at: Timestamp,
}

impl LeaseState {
    /// A fresh lease for `agent` with a new token.
    pub fn issue(agent: &str) -> LeaseState {
        LeaseState {
            token: ids::short_id("lease"),
            agent: agent.to_string(),
            expires_at: expiry(),
        }
    }

    /// Renew this lease: same token and agent, extended expiry (SCHEMAS: a
    /// renewal changes only `expires_at`).
    pub fn renewed(&self) -> LeaseState {
        LeaseState {
            token: self.token.clone(),
            agent: self.agent.clone(),
            expires_at: expiry(),
        }
    }

    pub fn is_expired(&self, now: Timestamp) -> bool {
        self.expires_at <= now
    }
}

fn expiry() -> Timestamp {
    let now = Timestamp::now();
    now.checked_add(ttl_seconds().seconds()).unwrap_or(now)
}
