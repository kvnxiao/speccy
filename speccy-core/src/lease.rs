//! The run-level lease: one-writer-at-a-time enforcement (DESIGN § Run Lease
//! and Concurrent Writers).
//!
//! `run next --agent` issues or renews an agent-bound token; state-mutating
//! operations pass it back as `--lease`. A second live session gets
//! `lease_held`; an expired lease is cleared deterministically by the next
//! `run next`, so a crashed session never wedges the run.

use crate::ids;
use jiff::Timestamp;
use jiff::ToSpan;
use serde::Deserialize;
use serde::Serialize;

/// MVP lease TTL: 10 minutes, overridable for tests via
/// `SPECCY_LEASE_TTL_SECONDS`.
#[must_use = "reads a config value that must be used to have any effect"]
pub fn ttl_seconds() -> i64 {
    std::env::var("SPECCY_LEASE_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(600)
}

/// The persisted run lease: the current writer's token, agent, and expiry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseState {
    pub token: String,
    pub agent: String,
    pub expires_at: Timestamp,
}

impl LeaseState {
    /// A fresh lease for `agent` with a new token.
    #[must_use = "constructs a lease that must be stored to take effect"]
    pub fn issue(agent: &str) -> LeaseState {
        LeaseState {
            token: ids::short_id("lease"),
            agent: agent.to_string(),
            expires_at: expiry(),
        }
    }

    /// Renew this lease: same token and agent, extended expiry (SCHEMAS: a
    /// renewal changes only `expires_at`).
    #[must_use = "constructs a renewed lease that must be stored to take effect"]
    pub fn renewed(&self) -> LeaseState {
        LeaseState {
            token: self.token.clone(),
            agent: self.agent.clone(),
            expires_at: expiry(),
        }
    }

    /// True if the lease has expired as of `now`.
    #[must_use = "checking expiry has no effect unless the result is used"]
    pub fn is_expired(&self, now: Timestamp) -> bool {
        self.expires_at <= now
    }
}

fn expiry() -> Timestamp {
    let now = Timestamp::now();
    now.checked_add(ttl_seconds().seconds()).unwrap_or(now)
}
