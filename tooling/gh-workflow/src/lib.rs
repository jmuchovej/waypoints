extern crate proc_macro;

/// Fills each `None` field of `$this` from `$other`, which is destructured
/// exhaustively: a field added to the struct fails to compile until it is
/// listed. Fields named after `rest` are bound for the caller to merge.
macro_rules! fill_none {
    ($this:ident, $other:ident { $($field:ident),* $(,)? } $(rest { $($extra:ident),* $(,)? })?) => {
        let Self { $($field,)* $($($extra,)*)? } = $other;
        $(
            if $this.$field.is_none() {
                $this.$field = $field;
            }
        )*
    };
}

mod artifacts;
mod cargo;
mod concurrency;
mod container;
mod defaults;
mod env;
mod environment;
pub mod error;
mod event;
mod expression;
mod fluent;
mod job;
mod outputs;
mod permissions;
mod secrets;
mod step;
mod strategy;

pub mod ctx;
mod rust_flag;
pub mod toolchain;
pub(crate) mod workflow;

pub use artifacts::*;
pub use cargo::*;
pub use concurrency::*;
pub use container::*;
pub use defaults::*;
pub use env::*;
pub use environment::*;
pub use event::*;
pub use expression::*;
pub use fluent::*;
pub use job::*;
pub use outputs::*;
pub use permissions::*;
pub use rust_flag::*;
pub use secrets::*;
pub use step::*;
pub use strategy::*;
pub use workflow::*;

pub(crate) fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

mod private {
    pub trait Sealed {}
}
