//! The inline layer: [`RichText`][R] segments ↔ the dialect's inline syntax.
//!
//! One module, both directions — emit and parse share the token table, so a
//! spelling cannot drift between them. CommonMark's inline grammar is the base;
//! the dialect departs from it where reading matters (`*bold*`, `_italic_`,
//! `__underline__`) and extends it where Waypoints has marks CommonMark lacks.
//!
//! Filled by the inline grammar task.
//!
//! [R]: crate::RichText
