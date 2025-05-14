use sea_orm_migration::{
  prelude::*,
  schema::*,
  sea_orm::{EnumIter,Iterable},
  sea_query::{Index},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

const INDEXES: &[(&str, Task)] = &[
    ("idx-task-name"        , Task::Name            ),
    ("idx-task-due"         , Task::DueDate         ),
    ("idx-task-start"       , Task::StartDate       ),
    ("idx-task-end"         , Task::EndDate         ),
    ("idx-task-scheduled"   , Task::ScheduledDate   ),
    ("idx-task-until"       , Task::UntilDate       ),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Task::Table)
                .if_not_exists()
                .col(pk_uuid(Task::UUID))
                .col(string(Task::Name))
                .col(string(Task::Description))
                .col(timestamp(Task::CreateDate))
                .col(timestamp(Task::ModifyDate))
                .col(timestamp(Task::DueDate))
                .col(timestamp_null(Task::StartDate))
                .col(timestamp_null(Task::EndDate))
                .col(timestamp_null(Task::ScheduledDate))
                .col(timestamp_null(Task::WaitDate))
                .col(timestamp_null(Task::UntilDate))
                .col(uuid_null(Task::Parent))
                .col(enumeration(
                  Task::Status, Alias::new("status"), Status::iter()
                ))
                .col(enumeration_null(
                  Task::EstimatedTime, Alias::new("estimated-time"), EstimatedTime::iter(),
                ))
                .col(enumeration_null(
                  Task::EstimatedEnergy, Alias::new("estimated-effort"), EstimatedEnergy::iter(),
                ))
                .col(enumeration_null(
                  Task::Priority, Alias::new("priority"), Priority::iter(),
                ))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-task-parent")
                        .from(Task::Table, Task::Parent)
                        .to(Task::Table, Task::UUID)
                )
                .to_owned(),
        ).await?;

        for index  in INDEXES.iter() {
            let (name, column) = *index;
            manager.create_index(
                Index::create().if_not_exists()
                .name(name).table(Task::Table).col(column)
                .to_owned(),
            ).await?;
        }

        // manager.create_foreign_key(
        //     ForeignKey::create()
        //         .name("fk-task-parent")
        //         .from(Task::Table, Task::Parent)
        //         .to(Task::Table, Task::UUID)
        //         .to_owned(),
        // ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_foreign_key(
            ForeignKey::drop()
                .table(Task::Table)
                .name("fk-task-parent")
                .to_owned(),
        ).await?;

        manager
            .drop_table(Table::drop().table(Task::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden, Copy, Clone)]
#[sea_orm(rename_all = "kebab-case")]
enum Task {
    Table,
    /// We need a unique identifier, this is it.
    UUID,
    /// What's the name of this task?
    Name,
    /// What's this task about?
    Description,
    /// When was this `Task` created?
    CreateDate,
    /// When was this `Task` last modified?
    ModifyDate,
    /// When did we start working on this `Task`?
    StartDate,
    /// When was this task `Completed` or `Deleted`?
    EndDate,
    /// When should we start considering working on this `Task`?
    ScheduledDate,
    /// When is the last possible date this `Task` could be **on-time**?
    DueDate,
    /// When should we start showing this `Task`?
    WaitDate,
    /// Parent Task's UUID
    Parent,
    /// If this is a recurring task, how frequently should it?
    Recurrence,
    /// If this is a recurring task, when should it stop recurring?
    UntilDate,
    /// What's the status of this `Task`?
    Status,
    /// How much **time** do you estimate this `Task` will take?
    EstimatedTime,
    /// How much **energy** do you estimate is `Task` will take?
    EstimatedEnergy,
    /// How **important** is it that this `Task` is completed ASAP?
    Priority,
}

#[derive(DeriveIden, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)", rename_all = "kebab-case")]
pub enum Status {
  /// The initial-state of a `Task`. All Tasks start this way until they're `Started`.
  Todo,

  /// We've "started" a task, but it's not complete (nor the current focus). Examples of this might be reading a book (in multiple sittings) or a task which is taking longer than you thought, but have higher priority items right now.
  Started,

  /// Something is blocking us from continuing. (e.g., waiting on a manager's response.)
  Waiting,

  /// We've been interrupted, but when we return from the interruption, we'll continue working on the task at-hand.
  Paused,

  /// This only applies to "parent" tasks -- if a child task is being worked on, this one will transition to "wip".
  #[sea_orm(string_value = "wip")]
  WorkInProgress,

  // A `Task` we need to do in the future, just waiting on the future to arrive.
  Scheduled,

  /// An absorbing  working on this task.
  Stopped,

  /// This only applies to "parent" tasks -- if a child task is being worked on, this one will transition to "wip".
  /// A `Task` we thought we needed to do, but either decided not to, discovered we didn't need to, or something else that caused it to be "dropped".
  Dropped,

  /// We finished! 🎉
  Complete,

  /// While `Projects` can be marked as "archived" and that will hide associated activities, it's possible that you might to archive specific activities.
  Archived,

  /// In an effort to avoid accidental data-loss, we mark activities as "deleted" rather than deleting them outright. This will still hide it from view, but in a manner that allows you to retrieve it deletion was accidental.
  Deleted,
}

/// The **time** required to complete this task. Measured in T-Shirt sizes.
/// If you know _exactly_ how long a task will take, don't both with this, use `PlannedCommence` and `PlannedComplete`. This is largely for the more abstract tasks which are relatively open-ended (thus harder to measure).
/// We make a **critical assumption** about this scheme: the average `Task` is expected to the a size `M` -- you should aim to measure all other sizes relative to this!
#[derive(DeriveIden, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)", rename_all = "kebab-case")]
pub enum EstimatedTime {
  /// Extra-Small. These tasks require the smallest amount of time. Think of these as something you could do almost automatically -- e.g. replying to an email asking a question you've answered before; fetching something you know the location of; etc.
  XS,
  /// Small. These are tasks require a pretty small amount of time. You can probably fit a few of these into an hour without much optimization.
  S,
  /// Medium. These are your average task. They can't be done too quickly, but will probably take on the order of dozens of minutes to finish.
  M,
  /// Large. These tasks are pretty big. You anticipate they'll take longer than your usual tasks `M`. Probably on the order of hours. If you find yourself heading into multiple hours (say, 3-hours+), you might want to bump this up to an `XL`.
  /// **NOTE**: That "3-hours+" is arbitrary! It's up to you to decide, but hopefully we can help you sort that out if you're having trouble. 🙂
  L,
  /// Extra-Large. These tasks are huge. You anticipate that it will take substantially longer than your usual tasks (`M`). Probably on the order of multiple hours. If you find yourself heading in to "days" territory, you probably need to break down the `Task`.
  XL,
}

/// The **energy** (think physical **or** mental) required to complete this
/// task. Measured in T-Shirt sizes. We make a **critical assumption** about this scheme: the average `Task` is expected to the a size `M` -- you should aim to measure all other sizes relative to this!
/// For example:
/// - If you find cleaning your residence to require **little** mental or physical energy, then you should rate it as `XS`.
/// - If you find setting up a password manager on your computer to require **incredible** mental energy, then you should rate it as `XL`.
/// This is a strictly monotonic scale -- in a given day, the expectation is that your quantity of tasks at each level would be `XS > S > M > L > XL`.
/// A note on this assertion -- we're not factoring in various factors that **will** alter your daily energy (e.g., psychiatric conditions, hormone levels, etc.).
#[derive(DeriveIden, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)", rename_all = "kebab-case")]
pub enum EstimatedEnergy {
  /// Extra-Small. These are tasks that you find to take little-to-no energy to complete. You could do these even on your most draining days.
  XS,
  /// Small. These are tasks that you find to take little energy to complete; but on your most draining days you might even find it challenging to complete them.
  S,
  /// Medium. These are tasks that you find to take some energy to complete. Most tasks that require sustained mental (or physical) energy are probably here. You'll probably have a difficult time doing these when drained, but you also don't need peak energy to complete this.
  M,
  /// Large. These are tasks that you find to take more energy to complete. These tasks definitely require sustained mental (or physical) energy. You'll probably have a difficult time doing more than a `L` tasks in a given day -- you don't need peak energy, but still need quite a bit.
  L,
  /// Extra-Large. These are tasks that you find to take **the most** energy to complete. You'll probably have a difficult time doing more than a few XL tasks in a given day -- **doing them requires peak energy**.
  XL,
}

/// All tasks with an urgency **have some pressure** to be completed, but pressure, of course, is ranked.
/// This is an optional field, thus doesn't need to be supplied (i.e., it defaults to `null`).
#[derive(DeriveIden, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)", rename_all = "kebab-case")]
pub enum Priority {
  /// Low. Not much pressure to complete this, but should be higher-pressure than null values.
  L,
  /// Medium. This task is regularly urgent, but it's not too pressing to finish it.
  M,
  /// High. This task needs to be completed ASAP.
  H,
}
