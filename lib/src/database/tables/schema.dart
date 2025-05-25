import 'package:drift/drift.dart';
import './_mixins.dart';
import '../converters.dart';

@DataClassName('Schema')
class Schemas extends Table with BaseColumns {
  TextColumn get name => text()();

  IntColumn get version => integer().clientDefault(() => 1)();
  BoolColumn get systemDefined => boolean().clientDefault(() => false)();

  TextColumn get definition => text().map(const JsonConverter())();
  TextColumn get display => text().map(const JsonConverter()).clientDefault(() => '{}')();
  TextColumn get indexed => text().map(const JsonConverter()).clientDefault(() => '{}')();

  @override
  Set<Column> get primaryKey => {uuid};
}
