import 'package:drift/drift.dart';
import '../converters.dart';
import './_mixins.dart';

@DataClassName('Block')
class Blocks extends Table with BaseColumns {
  TextColumn get type => text()();

  TextColumn get properties => text().map(const JsonConverter())();
  TextColumn get metadata => text().map(const JsonConverter()).clientDefault(() => '{}')();

  @override
  Set<Column> get primaryKey => {uuid};
}
