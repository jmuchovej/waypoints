import 'package:drift/drift.dart';
import 'package:tempo/src/database/sqlite/tables/schema.dart';
import '_mixins.dart';
import '../converters.dart';

@DataClassName('Thing')
class Things extends Table with BaseColumns {

  TextColumn get title => text().map(const AttributedTextConverter())();
  TextColumn get schema => text().references(Schemas, #uuid)();
  TextColumn get properties => text().map(const JsonConverter())();
  TextColumn get metadata => text().map(const JsonConverter()).clientDefault(() => '{}')();

  @override
  Set<Column> get primaryKey => {uuid};
}

@DataClassName('Child')
class Children extends Table {
  TextColumn get uuid => text()();

  TextColumn get parent => text().references(Things, #uuid, onDelete: KeyAction.cascade)();
  TextColumn get child => text()();
  TextColumn get type => text()();

  RealColumn get position => real()();

  IntColumn get logicalTimestamp => integer()();
  IntColumn get sequenceNumber => integer()();
  TextColumn get deviceID => text()();
  DateTimeColumn get createdAt => dateTime().clientDefault(() => DateTime.now())();

  @override
  Set<Column> get primaryKey => {uuid};

  @override
  List<Set<Column>> get uniqueKeys => [
    {parent, position},  // Each position should be unique within a parent
    // {parent, child}, // Each child should appear exactly once within a parent
  ];
}
