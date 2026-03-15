pub mod error;
mod history;
pub mod mutation;
pub mod ops;
pub mod projection;
pub mod rich_text;
pub mod service;
pub mod types;

pub use error::{EditorError, EditorResult};
pub use service::EditorService;
pub use types::{
    CreateResult, CursorState, MutationResult, RenderNode, SplitResult, UndoRedoResult,
};
