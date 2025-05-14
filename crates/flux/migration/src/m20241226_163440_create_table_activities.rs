use sea_orm_migration::{
  prelude::*,
  schema::*,
  sea_orm::{EnumIter,Iterable},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Activity::Table)
                    .if_not_exists()
                    .col(pk_uuid(Activity::UUID))
                    .col(string(Activity::Name))
                    .col(timestamp_null(Activity::StartDate))
                    .col(timestamp_null(Activity::EndDate))
                    .col(enumeration(
                        Activity::Kind, Alias::new("kind"), Kind::iter(),
                    ).default("activity"))
                    .col(enumeration(
                        Activity::Status, Alias::new("status"), Status::iter(),
                    ).default("created"))
                    .col(json_null(Activity::Tags))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Activity::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
#[sea_orm(rename_all = "kebab-case")]
enum Activity {
    Table,
    /// We need a unique identifier, this is it.
    UUID,
    /// What's the name of this `Activity`?
    Name,
    /// What kind of activity is this?
    Kind,
    /// When did we actually start on this `Activity`?
    StartDate,
    /// When did we actually finish this `Activity`?
    EndDate,
    /// What's the status of this `Activity`?
    Status,
    Tags,
}

#[derive(DeriveIden, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)", rename_all = "kebab-case")]
pub enum Kind {
  /// A generic activity kind.
  Activity,
  /// We've scheduled a task to work on during this period.
  Task,
  /// A calendar event has been scheduled during this period.
  Event,
  /// We're doing an interval-based method (e.g., Pomodoro) and are currently in a work-period.
  IntervalWork,
  /// We're doing an interval-based method (e.g., Pomodoro) and are currently in a rest-period.
  IntervalRest,
  /// We're in a present time-block and freely transitioning between related tasks in the time-block.
  TimeBlock,
}

#[derive(DeriveIden, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)", rename_all = "kebab-case")]
pub enum Status {
  /// The initial-state of an activity. All Activities start this way until they're started.
  Created,

  /// We've assigned a start-time to the activity, but the time hasn't come. The activity will remain in this state until the time comes.
  Scheduled,

  /// The active state of an activity. Only `Scheduled` and `Paused` will transition to `InProgress`. Specifically, `Scheduled` will transition to `InProgress` when the activity is first started. Otherwise, it will transition from `Paused`.
  InProgress,

  /// The activity has been [temporarily] interrupted by something. This could be a break, intermission, or an interruption of some other kind.
  /// The activity can transition back to `InProgress` when work resumes.
  Paused,

  /// The final state of an activity -- indicating it's been finished.
  /// The "end time" of this activity is recording, marking this activity's completion.
  Completed,

  /// While `Projects` can be marked as "archived" and that will hide associated activities, it's possible that you might to archive specific activities.
  Archived,

  /// In an effort to avoid accidental data-loss, we mark activities as "deleted" rather than deleting them outright. This will still hide it from view, but in a manner that allows you to retrieve it deletion was accidental.
  Deleted,
}
