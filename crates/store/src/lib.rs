pub(crate) mod error;
pub use error::{StoreError, StoreResult};

pub mod collections;
mod store;
pub use store::Store;

pub(crate) mod tools;
pub use tools::{checkpoint, open, reset};

pub mod query {
    pub mod chart;
    pub mod mark;
    pub mod sync;
    pub mod terrain;
}

#[cfg(feature = "test-utils")]
pub mod test_utils;

/// Built-in [`Chart`][C] types that provide common schemas out-of-the-box for
/// Waypoints.
///
/// These built-ins serve as:
/// - Ready-to-use types for common use-cases.
/// - Templates that users can build upon to create their own custom charts.
/// - Defaults that can be reset if users break their schemas.
///
/// The built-in system follows these principles:
/// - Each built-in chart has a fixed [`ChartID`][id] for consistency.
/// - Uses can reset to these original schemas at any time.
/// - Built-ins cover common content management and productivity needs.
///
/// [C]: domain::Chart
/// [id]: domain::ChartID
pub mod builtin {
    mod _utils;
    use _utils::{status_multi_select, tags_feature};

    pub mod book;
    pub mod bookmark;
    pub mod campaign;
    pub mod journal;
    pub mod meeting;
    pub mod note;
    pub mod objective;
    pub mod page;
    pub mod person;
}
