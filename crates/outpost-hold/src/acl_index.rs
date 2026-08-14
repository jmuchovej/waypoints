//! An in-memory fold of a Territory's ACL log that implements the
//! synchronous [`ACLStateProvider`] seam. A hold materializes
//! one of these from its persisted `ACLRow`s so validation — which is pure
//! and sync — can run against the folded state without touching the async
//! store.

use crate::{
    HoldError,
    protocol::{ACLRecord, ACLState, ACLStateProvider, RecordID},
};
use std::collections::BTreeMap;

/// The folded ACL log: each record's state and its log position.
#[derive(Clone, Debug, Default)]
pub struct ACLIndex {
    states: BTreeMap<RecordID, (u64, ACLState)>,
    head: Option<RecordID>,
    head_state: Option<ACLState>,
    len: u64,
}

impl ACLIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replay records in log order, folding each. Records must arrive in
    /// `position` order (the caller sorts). A fold failure here means the
    /// *stored* log is inconsistent — a bug, surfaced as [`HoldError`].
    pub fn from_ordered(records: &[ACLRecord]) -> Result<Self, HoldError> {
        let mut index = Self::new();
        for record in records {
            index.push(record)?;
        }
        Ok(index)
    }

    /// Fold one more record onto the head.
    pub fn push(&mut self, record: &ACLRecord) -> Result<(), HoldError> {
        let next = match &self.head_state {
            None => ACLState::genesis(record),
            Some(state) => state.apply(record),
        }
        .map_err(|e| HoldError::ACLFold(e.to_string()))?;
        let position = self.len;
        self.head = Some(next.head);
        self.states.insert(next.head, (position, next.clone()));
        self.head_state = Some(next);
        self.len += 1;
        Ok(())
    }

    /// The current head record, if the log is non-empty.
    pub fn head(&self) -> Option<RecordID> {
        self.head
    }

    /// The state produced by the current head.
    pub fn head_state(&self) -> Option<&ACLState> {
        self.head_state.as_ref()
    }
}

impl ACLStateProvider for ACLIndex {
    fn state_at(&self, record: RecordID) -> Option<&ACLState> {
        self.states.get(&record).map(|(_, state)| state)
    }

    fn position_of(&self, record: RecordID) -> Option<u64> {
        self.states.get(&record).map(|(position, _)| *position)
    }
}
