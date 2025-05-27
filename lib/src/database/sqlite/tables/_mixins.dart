import 'package:drift/drift.dart';

mixin BaseColumns on Table {
  TextColumn get uuid => text()();

  BoolColumn get deleted => boolean().clientDefault(() => false)();
  BoolColumn get archived => boolean().clientDefault(() => false)();

  DateTimeColumn get createdAt => dateTime().clientDefault(() => DateTime.now())();
  DateTimeColumn get updatedAt => dateTime().clientDefault(() => DateTime.now())();
}
