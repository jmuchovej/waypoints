enum ActivityKind {
  activity, // Generic activity
  task, // Working on task
  event, // Calendar event
  intervalWork, // Work period (Pomodoro)
  intervalRest, // Rest period (Pomodoro)
  timeBlock, // Time-block with related tasks
}

void assertStableEnumValues() {
  assert(0 == ActivityKind.activity.index);
  assert(1 == ActivityKind.task.index);
  assert(2 == ActivityKind.event.index);
  assert(3 == ActivityKind.intervalWork.index);
  assert(4 == ActivityKind.intervalRest.index);
  assert(5 == ActivityKind.timeBlock.index);
}
