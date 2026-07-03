//! LWW application (Phase 5, #131): `apply_with_lww` + per-target dispatch.
//!
//! The `LWWTarget` enum and `AtomicOp::lww_target` live in
//! `crate::editor::ops`, next to the op definitions they classify.
