import 'package:drift/drift.dart';
import 'package:drift_flutter/drift_flutter.dart';
import 'package:path_provider/path_provider.dart';

import 'tables/schema.dart';
import 'tables/thing.dart';
import 'tables/block.dart';

import 'database.drift.dart';

@DriftDatabase(tables: [Schemas, Things, Blocks])
class TempoDatabase extends $TempoDatabase {

  TempoDatabase([QueryExecutor? executor]) : super(executor ?? _openConnection());

  @override
  int get schemaVersion => 1;

  static QueryExecutor _openConnection() {
    return driftDatabase(
      name: 'tempo',
      native: const DriftNativeOptions(
        databaseDirectory: getApplicationSupportDirectory,
      ),
    );
  }
}
