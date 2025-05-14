use crate::entities::{task, task_list};
use tempo_moments::DateTime;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatusDTO {
  Pending,
  Completed { end_date: DateTime },
  Waiting   { wait_date: DateTime },
  Recurring { recurrence: String, due_date: DateTime, until_date: Option<DateTime> },
  Schedule  { until_date: DateTime },
  Archived  { end_date: DateTime },
  Deleted   { end_date: DateTime },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatusDTO {
  Pending,
  Completed { end_date: DateTime },
  Waiting   { wait_date: DateTime },
  Recurring { recurrence: String, due_date: DateTime, until_date: Option<DateTime> },
  Schedule  { until_date: DateTime },
  Archived  { end_date: DateTime },
  Deleted   { end_date: DateTime },
}


#[derive(Debug, Serialize, Deserialize)]
struct TaskDTO {
  pub uuid: Uuid,
  pub name: String,
  pub description: String,
  pub status: TaskStatusDTO,
  pub create_date: DateTime,
  pub modify_date: DateTime,
  pub start_date: Option<DateTime>,
  pub end_date: Option<DateTime>,
  pub due_date: Option<DateTime>,
  pub wait_date: Option<DateTime>,
  pub parent: Option<Uuid>,
  pub recurrence: Option<String>,
  pub until_date: Option<DateTime>,
  pub estimated_time: Option<EstimatedTimeDTO>,
  pub estimated_energy: Option<EstimatedEnergyDTO>,
  pub priority: Option<PriorityDTO>,
}
