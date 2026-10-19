//! The block layer: every [`Content`][C] variant ↔ its dialect form.
//!
//! Parse runs on `pulldown-cmark` with runtime options for tables,
//! strikethrough, and math, plus the component/attribute layer the dialect adds
//! for structured blocks (callouts today; the extension point generally). Emit
//! is this crate's own serializer — there is no CommonMark serializer to borrow.
//!
//! The dialect is a strict subset: one canonical spelling per construct, with
//! well-formedness defined as `emit(parse(doc)) == doc` rather than by a
//! hand-maintained validator.
//!
//! Filled by the block grammar task.
//!
//! [C]: crate::Content
