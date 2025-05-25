// dart format width=80
// ignore_for_file: type=lint
import 'package:drift/drift.dart' as i0;
import 'package:tempo/src/database/tables/schema.drift.dart' as i1;
import 'package:tempo/src/database/tables/thing.drift.dart' as i2;
import 'package:tempo/src/database/tables/block.drift.dart' as i3;

abstract class $TempoDatabase extends i0.GeneratedDatabase {
  $TempoDatabase(i0.QueryExecutor e) : super(e);
  $TempoDatabaseManager get managers => $TempoDatabaseManager(this);
  late final i1.$SchemasTable schemas = i1.$SchemasTable(this);
  late final i2.$ThingsTable things = i2.$ThingsTable(this);
  late final i3.$BlocksTable blocks = i3.$BlocksTable(this);
  @override
  Iterable<i0.TableInfo<i0.Table, Object?>> get allTables =>
      allSchemaEntities.whereType<i0.TableInfo<i0.Table, Object?>>();
  @override
  List<i0.DatabaseSchemaEntity> get allSchemaEntities => [
    schemas,
    things,
    blocks,
  ];
  @override
  i0.DriftDatabaseOptions get options =>
      const i0.DriftDatabaseOptions(storeDateTimeAsText: true);
}

class $TempoDatabaseManager {
  final $TempoDatabase _db;
  $TempoDatabaseManager(this._db);
  i1.$$SchemasTableTableManager get schemas =>
      i1.$$SchemasTableTableManager(_db, _db.schemas);
  i2.$$ThingsTableTableManager get things =>
      i2.$$ThingsTableTableManager(_db, _db.things);
  i3.$$BlocksTableTableManager get blocks =>
      i3.$$BlocksTableTableManager(_db, _db.blocks);
}
