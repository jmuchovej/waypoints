mod chart_orm;
mod mark_orm;

pub use chart_orm::{
    ActiveModel as ActiveChart, Column as ChartColumn, Entity as Charts, Model as ChartModel,
};
pub use mark_orm::{
    ActiveModel as ActiveMark, Column as MarkColumn, Entity as Marks, Model as MarkModel,
};
