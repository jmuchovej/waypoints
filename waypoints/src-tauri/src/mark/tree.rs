use domain::mark::Mark;
use serde::{Deserialize, Serialize};
use specta::Type;

/// A mark with its recursively expanded children.
///
/// Content children are expanded recursively; Landmark children are leaf nodes
/// with an empty `children` vec.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MarkTree {
    #[serde(flatten)]
    pub mark: Mark,
    pub children: Vec<MarkTree>,
}
