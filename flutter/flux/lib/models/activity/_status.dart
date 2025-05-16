enum ActivityStatus {
  created, // Initial state
  scheduled, // Start time assigned but not started
  active, // Currently active
  paused, // Temporarily interrupted
  completed, // Finished
}

void assertStableEnumValues() {
  assert(0 == ActivityStatus.created.index);
  assert(1 == ActivityStatus.scheduled.index);
  assert(2 == ActivityStatus.active.index);
  assert(3 == ActivityStatus.paused.index);
  assert(4 == ActivityStatus.completed.index);
}
