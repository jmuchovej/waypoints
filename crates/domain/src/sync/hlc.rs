use chrono::Utc;
use derive_more::{Debug, Display};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Injectable time source for HLC operations. Production code uses
/// [`SystemClock`]; tests use a mock so HLC behavior is deterministic
/// (never depend on real wall-clock timing in tests).
pub trait Clock {
    /// Current time in milliseconds since the UNIX epoch.
    fn now_ms(&self) -> i64;
}

/// The production [`Clock`]: wall-clock time via `chrono::Utc`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        Utc::now().timestamp_millis()
    }
}

/// HLC is the total ordering primitive for sync. It combines a wall-clock millisecond with a logical counter for tie-breaking, packed into a single `i64` for SQLite-friendly storage.
/// **Layout (i64):**
///
///```text
///┌──────────────────────────────────────────────┬──────────────────┐
///│  48 bits: physical (ms since UNIX epoch)     │ 16 bits: logical │
///└──────────────────────────────────────────────┴──────────────────┘
///```
///
/// 48 bits of milliseconds gets us through year 10889. 16 bits of logical counter handles 65,536 events per millisecond per device — vastly more than a human can produce.
///
/// Because logical occupies the low 16 bits and physical is always
/// non-negative, the derived i64 `Ord` matches HLC semantics:
/// `(physical, logical)` lexicographic order. Note the top physical bit
/// doubles as the i64 sign bit, so the *usable* physical range is 47 bits
/// (through year ~6429) — beyond that, packed values go negative and
/// ordering breaks.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Display, Type,
)]
#[display("{}", self.inner())]
#[debug("HLC({}+{})", self.physical(), self.logical())]
#[serde(transparent)]
pub struct HLC(pub i64);

impl HLC {
    pub const ZERO: HLC = HLC(0);

    pub fn inner(self) -> i64 {
        self.0
    }

    pub fn physical(self) -> i64 {
        self.inner() >> 16
    }

    pub fn logical(self) -> u16 {
        (self.inner() & 0xFFFF) as u16
    }

    pub fn pack(physical_ms: i64, logical: u16) -> Self {
        HLC((physical_ms << 16) | (logical as i64))
    }

    /// Tick: produce a new HLC for a local event.
    pub fn tick(prev: HLC) -> HLC {
        Self::tick_with(prev, &SystemClock)
    }

    /// [`HLC::tick`] with an explicit clock (for tests).
    pub(crate) fn tick_with(prev: HLC, clock: &impl Clock) -> HLC {
        let physical_ms = clock.now_ms();
        if physical_ms > prev.physical() {
            HLC::pack(physical_ms, 0)
        } else {
            // Wall clock didn't advance; bump logical counter.
            HLC::pack(prev.physical(), prev.logical().wrapping_add(1))
        }
    }

    /// Receive: update local HLC after observing a remote HLC.
    /// Standard Kulkarni/Demirbas formulation.
    pub fn receive(local: HLC, remote: HLC) -> HLC {
        Self::receive_with(local, remote, &SystemClock)
    }

    /// [`HLC::receive`] with an explicit clock (for tests).
    pub fn receive_with(local: HLC, remote: HLC, clock: &impl Clock) -> HLC {
        let physical_ms = clock.now_ms();
        let max_physical = physical_ms.max(local.physical()).max(remote.physical());
        let logical = if max_physical == local.physical() && max_physical == remote.physical() {
            local.logical().max(remote.logical()).wrapping_add(1)
        } else if max_physical == local.physical() {
            local.logical().wrapping_add(1)
        } else if max_physical == remote.physical() {
            remote.logical().wrapping_add(1)
        } else {
            0
        };

        HLC::pack(max_physical, logical)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// A clock frozen at (or stepped through) explicit instants.
    struct MockClock(Cell<i64>);

    impl MockClock {
        fn at(ms: i64) -> Self {
            Self(Cell::new(ms))
        }

        fn set(&self, ms: i64) {
            self.0.set(ms);
        }
    }

    impl Clock for MockClock {
        fn now_ms(&self) -> i64 {
            self.0.get()
        }
    }

    #[test]
    fn pack_round_trips_physical_and_logical() {
        // Logical > 0xFFF would truncate under a 12-bit mask; use the full
        // 16-bit range to pin the layout down.
        let hlc = HLC::pack(1_735_689_600_000, 0xBEEF);
        assert_eq!(hlc.physical(), 1_735_689_600_000);
        assert_eq!(hlc.logical(), 0xBEEF);

        // Largest physical that keeps the packed i64 non-negative: the top
        // physical bit doubles as the sign bit (see type-level doc).
        let max = HLC::pack((1 << 47) - 1, u16::MAX);
        assert_eq!(max.physical(), (1 << 47) - 1);
        assert_eq!(max.logical(), u16::MAX);
        assert!(max.inner() >= 0);
    }

    #[test]
    fn ord_matches_physical_then_logical() {
        let base = 1_735_689_600_000;
        assert!(HLC::pack(base, 0) < HLC::pack(base, 1));
        assert!(HLC::pack(base, u16::MAX) < HLC::pack(base + 1, 0));
        assert!(HLC::pack(base, 5) == HLC::pack(base, 5));
        assert!(HLC::ZERO < HLC::pack(base, 0));
    }

    #[test]
    fn tick_is_strictly_increasing_across_1000_calls() {
        // Frozen clock forces every tick through the logical-bump path —
        // the worst case for monotonicity.
        let clock = MockClock::at(1_735_689_600_000);
        let mut prev = HLC::tick_with(HLC::ZERO, &clock);
        for _ in 0..1000 {
            let next = HLC::tick_with(prev, &clock);
            assert!(next > prev, "tick must be strictly increasing");
            prev = next;
        }
    }

    #[test]
    fn tick_resets_logical_when_wall_clock_advances() {
        let clock = MockClock::at(1_735_689_600_000);
        let a = HLC::tick_with(HLC::ZERO, &clock);
        let b = HLC::tick_with(a, &clock);
        assert_eq!(b.logical(), a.logical() + 1);

        clock.set(1_735_689_600_001);
        let c = HLC::tick_with(b, &clock);
        assert!(c > b);
        assert_eq!(c.physical(), 1_735_689_600_001);
        assert_eq!(c.logical(), 0);
    }

    #[test]
    fn receive_is_monotone_over_both_inputs() {
        let base = 1_735_689_600_000;
        let cases = [
            // (clock, local, remote)
            (base, HLC::pack(base, 3), HLC::pack(base, 7)), // all equal physical
            (base - 10, HLC::pack(base, 3), HLC::pack(base, 3)), // clock behind, identical HLCs
            (base, HLC::pack(base + 5, 2), HLC::pack(base, 9)), // local ahead
            (base, HLC::pack(base, 9), HLC::pack(base + 5, 2)), // remote ahead
            (base + 100, HLC::pack(base, 9), HLC::pack(base + 5, 2)), // clock ahead of both
            (base, HLC::ZERO, HLC::pack(base, 0)),          // from zero state
        ];

        for (now, local, remote) in cases {
            let clock = MockClock::at(now);
            let next = HLC::receive_with(local, remote, &clock);
            assert!(next >= local, "receive({local:?}, {remote:?}) < local");
            assert!(next >= remote, "receive({local:?}, {remote:?}) < remote");
        }
    }

    #[test]
    fn receive_dominates_identical_inputs() {
        // Two devices producing the same HLC must not stall: the result
        // strictly exceeds both so the next local event orders after.
        let base = 1_735_689_600_000;
        let clock = MockClock::at(base);
        let hlc = HLC::pack(base, 4);
        let next = HLC::receive_with(hlc, hlc, &clock);
        assert!(next > hlc);
    }
}
