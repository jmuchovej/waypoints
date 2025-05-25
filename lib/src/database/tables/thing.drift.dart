// dart format width=80
// ignore_for_file: type=lint
import 'package:drift/drift.dart' as i0;
import 'package:tempo/src/database/tables/thing.drift.dart' as i1;
import 'package:tempo/src/database/tables/thing.dart' as i2;
import 'package:tempo/src/database/converters.dart' as i3;
import 'package:tempo/src/database/tables/schema.drift.dart' as i4;
import 'package:drift/internal/modular.dart' as i5;

typedef $$ThingsTableCreateCompanionBuilder =
    i1.ThingsCompanion Function({
      required String uuid,
      i0.Value<bool> deleted,
      i0.Value<bool> archived,
      i0.Value<DateTime> createdAt,
      i0.Value<DateTime> updatedAt,
      required String title,
      required String schema,
      required Map<String, dynamic> properties,
      i0.Value<Map<String, dynamic>> metadata,
      i0.Value<int> rowid,
    });
typedef $$ThingsTableUpdateCompanionBuilder =
    i1.ThingsCompanion Function({
      i0.Value<String> uuid,
      i0.Value<bool> deleted,
      i0.Value<bool> archived,
      i0.Value<DateTime> createdAt,
      i0.Value<DateTime> updatedAt,
      i0.Value<String> title,
      i0.Value<String> schema,
      i0.Value<Map<String, dynamic>> properties,
      i0.Value<Map<String, dynamic>> metadata,
      i0.Value<int> rowid,
    });

final class $$ThingsTableReferences
    extends i0.BaseReferences<i0.GeneratedDatabase, i1.$ThingsTable, i1.Thing> {
  $$ThingsTableReferences(super.$_db, super.$_table, super.$_typedResult);

  static i4.$SchemasTable _schemaTable(i0.GeneratedDatabase db) =>
      i5.ReadDatabaseContainer(db)
          .resultSet<i4.$SchemasTable>('schemas')
          .createAlias(
            i0.$_aliasNameGenerator(
              i5.ReadDatabaseContainer(
                db,
              ).resultSet<i1.$ThingsTable>('things').schema,
              i5.ReadDatabaseContainer(
                db,
              ).resultSet<i4.$SchemasTable>('schemas').uuid,
            ),
          );

  i4.$$SchemasTableProcessedTableManager get schema {
    final $_column = $_itemColumn<String>('schema')!;

    final manager = i4
        .$$SchemasTableTableManager(
          $_db,
          i5.ReadDatabaseContainer($_db).resultSet<i4.$SchemasTable>('schemas'),
        )
        .filter((f) => f.uuid.sqlEquals($_column));
    final item = $_typedResult.readTableOrNull(_schemaTable($_db));
    if (item == null) return manager;
    return i0.ProcessedTableManager(
      manager.$state.copyWith(prefetchedData: [item]),
    );
  }

  static i0.MultiTypedResultKey<i1.$ChildrenTable, List<i1.Child>>
  _childrenRefsTable(i0.GeneratedDatabase db) => i0
      .MultiTypedResultKey.fromTable(
    i5.ReadDatabaseContainer(db).resultSet<i1.$ChildrenTable>('children'),
    aliasName: i0.$_aliasNameGenerator(
      i5.ReadDatabaseContainer(db).resultSet<i1.$ThingsTable>('things').uuid,
      i5.ReadDatabaseContainer(
        db,
      ).resultSet<i1.$ChildrenTable>('children').parent,
    ),
  );

  i1.$$ChildrenTableProcessedTableManager get childrenRefs {
    final manager = i1
        .$$ChildrenTableTableManager(
          $_db,
          i5.ReadDatabaseContainer(
            $_db,
          ).resultSet<i1.$ChildrenTable>('children'),
        )
        .filter((f) => f.parent.uuid.sqlEquals($_itemColumn<String>('uuid')!));

    final cache = $_typedResult.readTableOrNull(_childrenRefsTable($_db));
    return i0.ProcessedTableManager(
      manager.$state.copyWith(prefetchedData: cache),
    );
  }
}

class $$ThingsTableFilterComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$ThingsTable> {
  $$ThingsTableFilterComposer({
    required super.$db,
    required super.$table,
    super.joinBuilder,
    super.$addJoinBuilderToRootComposer,
    super.$removeJoinBuilderFromRootComposer,
  });
  i0.ColumnFilters<String> get uuid => $composableBuilder(
    column: $table.uuid,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<bool> get deleted => $composableBuilder(
    column: $table.deleted,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<bool> get archived => $composableBuilder(
    column: $table.archived,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<DateTime> get createdAt => $composableBuilder(
    column: $table.createdAt,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<DateTime> get updatedAt => $composableBuilder(
    column: $table.updatedAt,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<String> get title => $composableBuilder(
    column: $table.title,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnWithTypeConverterFilters<
    Map<String, dynamic>,
    Map<String, dynamic>,
    String
  >
  get properties => $composableBuilder(
    column: $table.properties,
    builder: (column) => i0.ColumnWithTypeConverterFilters(column),
  );

  i0.ColumnWithTypeConverterFilters<
    Map<String, dynamic>,
    Map<String, dynamic>,
    String
  >
  get metadata => $composableBuilder(
    column: $table.metadata,
    builder: (column) => i0.ColumnWithTypeConverterFilters(column),
  );

  i4.$$SchemasTableFilterComposer get schema {
    final i4.$$SchemasTableFilterComposer composer = $composerBuilder(
      composer: this,
      getCurrentColumn: (t) => t.schema,
      referencedTable: i5.ReadDatabaseContainer(
        $db,
      ).resultSet<i4.$SchemasTable>('schemas'),
      getReferencedColumn: (t) => t.uuid,
      builder:
          (
            joinBuilder, {
            $addJoinBuilderToRootComposer,
            $removeJoinBuilderFromRootComposer,
          }) => i4.$$SchemasTableFilterComposer(
            $db: $db,
            $table: i5.ReadDatabaseContainer(
              $db,
            ).resultSet<i4.$SchemasTable>('schemas'),
            $addJoinBuilderToRootComposer: $addJoinBuilderToRootComposer,
            joinBuilder: joinBuilder,
            $removeJoinBuilderFromRootComposer:
                $removeJoinBuilderFromRootComposer,
          ),
    );
    return composer;
  }

  i0.Expression<bool> childrenRefs(
    i0.Expression<bool> Function(i1.$$ChildrenTableFilterComposer f) f,
  ) {
    final i1.$$ChildrenTableFilterComposer composer = $composerBuilder(
      composer: this,
      getCurrentColumn: (t) => t.uuid,
      referencedTable: i5.ReadDatabaseContainer(
        $db,
      ).resultSet<i1.$ChildrenTable>('children'),
      getReferencedColumn: (t) => t.parent,
      builder:
          (
            joinBuilder, {
            $addJoinBuilderToRootComposer,
            $removeJoinBuilderFromRootComposer,
          }) => i1.$$ChildrenTableFilterComposer(
            $db: $db,
            $table: i5.ReadDatabaseContainer(
              $db,
            ).resultSet<i1.$ChildrenTable>('children'),
            $addJoinBuilderToRootComposer: $addJoinBuilderToRootComposer,
            joinBuilder: joinBuilder,
            $removeJoinBuilderFromRootComposer:
                $removeJoinBuilderFromRootComposer,
          ),
    );
    return f(composer);
  }
}

class $$ThingsTableOrderingComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$ThingsTable> {
  $$ThingsTableOrderingComposer({
    required super.$db,
    required super.$table,
    super.joinBuilder,
    super.$addJoinBuilderToRootComposer,
    super.$removeJoinBuilderFromRootComposer,
  });
  i0.ColumnOrderings<String> get uuid => $composableBuilder(
    column: $table.uuid,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<bool> get deleted => $composableBuilder(
    column: $table.deleted,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<bool> get archived => $composableBuilder(
    column: $table.archived,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<DateTime> get createdAt => $composableBuilder(
    column: $table.createdAt,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<DateTime> get updatedAt => $composableBuilder(
    column: $table.updatedAt,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<String> get title => $composableBuilder(
    column: $table.title,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<String> get properties => $composableBuilder(
    column: $table.properties,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<String> get metadata => $composableBuilder(
    column: $table.metadata,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i4.$$SchemasTableOrderingComposer get schema {
    final i4.$$SchemasTableOrderingComposer composer = $composerBuilder(
      composer: this,
      getCurrentColumn: (t) => t.schema,
      referencedTable: i5.ReadDatabaseContainer(
        $db,
      ).resultSet<i4.$SchemasTable>('schemas'),
      getReferencedColumn: (t) => t.uuid,
      builder:
          (
            joinBuilder, {
            $addJoinBuilderToRootComposer,
            $removeJoinBuilderFromRootComposer,
          }) => i4.$$SchemasTableOrderingComposer(
            $db: $db,
            $table: i5.ReadDatabaseContainer(
              $db,
            ).resultSet<i4.$SchemasTable>('schemas'),
            $addJoinBuilderToRootComposer: $addJoinBuilderToRootComposer,
            joinBuilder: joinBuilder,
            $removeJoinBuilderFromRootComposer:
                $removeJoinBuilderFromRootComposer,
          ),
    );
    return composer;
  }
}

class $$ThingsTableAnnotationComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$ThingsTable> {
  $$ThingsTableAnnotationComposer({
    required super.$db,
    required super.$table,
    super.joinBuilder,
    super.$addJoinBuilderToRootComposer,
    super.$removeJoinBuilderFromRootComposer,
  });
  i0.GeneratedColumn<String> get uuid =>
      $composableBuilder(column: $table.uuid, builder: (column) => column);

  i0.GeneratedColumn<bool> get deleted =>
      $composableBuilder(column: $table.deleted, builder: (column) => column);

  i0.GeneratedColumn<bool> get archived =>
      $composableBuilder(column: $table.archived, builder: (column) => column);

  i0.GeneratedColumn<DateTime> get createdAt =>
      $composableBuilder(column: $table.createdAt, builder: (column) => column);

  i0.GeneratedColumn<DateTime> get updatedAt =>
      $composableBuilder(column: $table.updatedAt, builder: (column) => column);

  i0.GeneratedColumn<String> get title =>
      $composableBuilder(column: $table.title, builder: (column) => column);

  i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  get properties => $composableBuilder(
    column: $table.properties,
    builder: (column) => column,
  );

  i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  get metadata =>
      $composableBuilder(column: $table.metadata, builder: (column) => column);

  i4.$$SchemasTableAnnotationComposer get schema {
    final i4.$$SchemasTableAnnotationComposer composer = $composerBuilder(
      composer: this,
      getCurrentColumn: (t) => t.schema,
      referencedTable: i5.ReadDatabaseContainer(
        $db,
      ).resultSet<i4.$SchemasTable>('schemas'),
      getReferencedColumn: (t) => t.uuid,
      builder:
          (
            joinBuilder, {
            $addJoinBuilderToRootComposer,
            $removeJoinBuilderFromRootComposer,
          }) => i4.$$SchemasTableAnnotationComposer(
            $db: $db,
            $table: i5.ReadDatabaseContainer(
              $db,
            ).resultSet<i4.$SchemasTable>('schemas'),
            $addJoinBuilderToRootComposer: $addJoinBuilderToRootComposer,
            joinBuilder: joinBuilder,
            $removeJoinBuilderFromRootComposer:
                $removeJoinBuilderFromRootComposer,
          ),
    );
    return composer;
  }

  i0.Expression<T> childrenRefs<T extends Object>(
    i0.Expression<T> Function(i1.$$ChildrenTableAnnotationComposer a) f,
  ) {
    final i1.$$ChildrenTableAnnotationComposer composer = $composerBuilder(
      composer: this,
      getCurrentColumn: (t) => t.uuid,
      referencedTable: i5.ReadDatabaseContainer(
        $db,
      ).resultSet<i1.$ChildrenTable>('children'),
      getReferencedColumn: (t) => t.parent,
      builder:
          (
            joinBuilder, {
            $addJoinBuilderToRootComposer,
            $removeJoinBuilderFromRootComposer,
          }) => i1.$$ChildrenTableAnnotationComposer(
            $db: $db,
            $table: i5.ReadDatabaseContainer(
              $db,
            ).resultSet<i1.$ChildrenTable>('children'),
            $addJoinBuilderToRootComposer: $addJoinBuilderToRootComposer,
            joinBuilder: joinBuilder,
            $removeJoinBuilderFromRootComposer:
                $removeJoinBuilderFromRootComposer,
          ),
    );
    return f(composer);
  }
}

class $$ThingsTableTableManager
    extends
        i0.RootTableManager<
          i0.GeneratedDatabase,
          i1.$ThingsTable,
          i1.Thing,
          i1.$$ThingsTableFilterComposer,
          i1.$$ThingsTableOrderingComposer,
          i1.$$ThingsTableAnnotationComposer,
          $$ThingsTableCreateCompanionBuilder,
          $$ThingsTableUpdateCompanionBuilder,
          (i1.Thing, i1.$$ThingsTableReferences),
          i1.Thing,
          i0.PrefetchHooks Function({bool schema, bool childrenRefs})
        > {
  $$ThingsTableTableManager(i0.GeneratedDatabase db, i1.$ThingsTable table)
    : super(
        i0.TableManagerState(
          db: db,
          table: table,
          createFilteringComposer:
              () => i1.$$ThingsTableFilterComposer($db: db, $table: table),
          createOrderingComposer:
              () => i1.$$ThingsTableOrderingComposer($db: db, $table: table),
          createComputedFieldComposer:
              () => i1.$$ThingsTableAnnotationComposer($db: db, $table: table),
          updateCompanionCallback:
              ({
                i0.Value<String> uuid = const i0.Value.absent(),
                i0.Value<bool> deleted = const i0.Value.absent(),
                i0.Value<bool> archived = const i0.Value.absent(),
                i0.Value<DateTime> createdAt = const i0.Value.absent(),
                i0.Value<DateTime> updatedAt = const i0.Value.absent(),
                i0.Value<String> title = const i0.Value.absent(),
                i0.Value<String> schema = const i0.Value.absent(),
                i0.Value<Map<String, dynamic>> properties =
                    const i0.Value.absent(),
                i0.Value<Map<String, dynamic>> metadata =
                    const i0.Value.absent(),
                i0.Value<int> rowid = const i0.Value.absent(),
              }) => i1.ThingsCompanion(
                uuid: uuid,
                deleted: deleted,
                archived: archived,
                createdAt: createdAt,
                updatedAt: updatedAt,
                title: title,
                schema: schema,
                properties: properties,
                metadata: metadata,
                rowid: rowid,
              ),
          createCompanionCallback:
              ({
                required String uuid,
                i0.Value<bool> deleted = const i0.Value.absent(),
                i0.Value<bool> archived = const i0.Value.absent(),
                i0.Value<DateTime> createdAt = const i0.Value.absent(),
                i0.Value<DateTime> updatedAt = const i0.Value.absent(),
                required String title,
                required String schema,
                required Map<String, dynamic> properties,
                i0.Value<Map<String, dynamic>> metadata =
                    const i0.Value.absent(),
                i0.Value<int> rowid = const i0.Value.absent(),
              }) => i1.ThingsCompanion.insert(
                uuid: uuid,
                deleted: deleted,
                archived: archived,
                createdAt: createdAt,
                updatedAt: updatedAt,
                title: title,
                schema: schema,
                properties: properties,
                metadata: metadata,
                rowid: rowid,
              ),
          withReferenceMapper:
              (p0) =>
                  p0
                      .map(
                        (e) => (
                          e.readTable(table),
                          i1.$$ThingsTableReferences(db, table, e),
                        ),
                      )
                      .toList(),
          prefetchHooksCallback: ({schema = false, childrenRefs = false}) {
            return i0.PrefetchHooks(
              db: db,
              explicitlyWatchedTables: [
                if (childrenRefs)
                  i5.ReadDatabaseContainer(
                    db,
                  ).resultSet<i1.$ChildrenTable>('children'),
              ],
              addJoins: <
                T extends i0.TableManagerState<
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic
                >
              >(state) {
                if (schema) {
                  state =
                      state.withJoin(
                            currentTable: table,
                            currentColumn: table.schema,
                            referencedTable: i1.$$ThingsTableReferences
                                ._schemaTable(db),
                            referencedColumn:
                                i1.$$ThingsTableReferences
                                    ._schemaTable(db)
                                    .uuid,
                          )
                          as T;
                }

                return state;
              },
              getPrefetchedDataCallback: (items) async {
                return [
                  if (childrenRefs)
                    await i0.$_getPrefetchedData<
                      i1.Thing,
                      i1.$ThingsTable,
                      i1.Child
                    >(
                      currentTable: table,
                      referencedTable: i1.$$ThingsTableReferences
                          ._childrenRefsTable(db),
                      managerFromTypedResult:
                          (p0) =>
                              i1
                                  .$$ThingsTableReferences(db, table, p0)
                                  .childrenRefs,
                      referencedItemsForCurrentItem:
                          (item, referencedItems) => referencedItems.where(
                            (e) => e.parent == item.uuid,
                          ),
                      typedResults: items,
                    ),
                ];
              },
            );
          },
        ),
      );
}

typedef $$ThingsTableProcessedTableManager =
    i0.ProcessedTableManager<
      i0.GeneratedDatabase,
      i1.$ThingsTable,
      i1.Thing,
      i1.$$ThingsTableFilterComposer,
      i1.$$ThingsTableOrderingComposer,
      i1.$$ThingsTableAnnotationComposer,
      $$ThingsTableCreateCompanionBuilder,
      $$ThingsTableUpdateCompanionBuilder,
      (i1.Thing, i1.$$ThingsTableReferences),
      i1.Thing,
      i0.PrefetchHooks Function({bool schema, bool childrenRefs})
    >;
typedef $$ChildrenTableCreateCompanionBuilder =
    i1.ChildrenCompanion Function({
      required String uuid,
      required String parent,
      required String child,
      required String type,
      required double position,
      required int logicalTimestamp,
      required int sequenceNumber,
      required String deviceID,
      i0.Value<DateTime> createdAt,
      i0.Value<int> rowid,
    });
typedef $$ChildrenTableUpdateCompanionBuilder =
    i1.ChildrenCompanion Function({
      i0.Value<String> uuid,
      i0.Value<String> parent,
      i0.Value<String> child,
      i0.Value<String> type,
      i0.Value<double> position,
      i0.Value<int> logicalTimestamp,
      i0.Value<int> sequenceNumber,
      i0.Value<String> deviceID,
      i0.Value<DateTime> createdAt,
      i0.Value<int> rowid,
    });

final class $$ChildrenTableReferences
    extends
        i0.BaseReferences<i0.GeneratedDatabase, i1.$ChildrenTable, i1.Child> {
  $$ChildrenTableReferences(super.$_db, super.$_table, super.$_typedResult);

  static i1.$ThingsTable _parentTable(i0.GeneratedDatabase db) =>
      i5.ReadDatabaseContainer(db)
          .resultSet<i1.$ThingsTable>('things')
          .createAlias(
            i0.$_aliasNameGenerator(
              i5.ReadDatabaseContainer(
                db,
              ).resultSet<i1.$ChildrenTable>('children').parent,
              i5.ReadDatabaseContainer(
                db,
              ).resultSet<i1.$ThingsTable>('things').uuid,
            ),
          );

  i1.$$ThingsTableProcessedTableManager get parent {
    final $_column = $_itemColumn<String>('parent')!;

    final manager = i1
        .$$ThingsTableTableManager(
          $_db,
          i5.ReadDatabaseContainer($_db).resultSet<i1.$ThingsTable>('things'),
        )
        .filter((f) => f.uuid.sqlEquals($_column));
    final item = $_typedResult.readTableOrNull(_parentTable($_db));
    if (item == null) return manager;
    return i0.ProcessedTableManager(
      manager.$state.copyWith(prefetchedData: [item]),
    );
  }
}

class $$ChildrenTableFilterComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$ChildrenTable> {
  $$ChildrenTableFilterComposer({
    required super.$db,
    required super.$table,
    super.joinBuilder,
    super.$addJoinBuilderToRootComposer,
    super.$removeJoinBuilderFromRootComposer,
  });
  i0.ColumnFilters<String> get uuid => $composableBuilder(
    column: $table.uuid,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<String> get child => $composableBuilder(
    column: $table.child,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<String> get type => $composableBuilder(
    column: $table.type,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<double> get position => $composableBuilder(
    column: $table.position,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<int> get logicalTimestamp => $composableBuilder(
    column: $table.logicalTimestamp,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<int> get sequenceNumber => $composableBuilder(
    column: $table.sequenceNumber,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<String> get deviceID => $composableBuilder(
    column: $table.deviceID,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<DateTime> get createdAt => $composableBuilder(
    column: $table.createdAt,
    builder: (column) => i0.ColumnFilters(column),
  );

  i1.$$ThingsTableFilterComposer get parent {
    final i1.$$ThingsTableFilterComposer composer = $composerBuilder(
      composer: this,
      getCurrentColumn: (t) => t.parent,
      referencedTable: i5.ReadDatabaseContainer(
        $db,
      ).resultSet<i1.$ThingsTable>('things'),
      getReferencedColumn: (t) => t.uuid,
      builder:
          (
            joinBuilder, {
            $addJoinBuilderToRootComposer,
            $removeJoinBuilderFromRootComposer,
          }) => i1.$$ThingsTableFilterComposer(
            $db: $db,
            $table: i5.ReadDatabaseContainer(
              $db,
            ).resultSet<i1.$ThingsTable>('things'),
            $addJoinBuilderToRootComposer: $addJoinBuilderToRootComposer,
            joinBuilder: joinBuilder,
            $removeJoinBuilderFromRootComposer:
                $removeJoinBuilderFromRootComposer,
          ),
    );
    return composer;
  }
}

class $$ChildrenTableOrderingComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$ChildrenTable> {
  $$ChildrenTableOrderingComposer({
    required super.$db,
    required super.$table,
    super.joinBuilder,
    super.$addJoinBuilderToRootComposer,
    super.$removeJoinBuilderFromRootComposer,
  });
  i0.ColumnOrderings<String> get uuid => $composableBuilder(
    column: $table.uuid,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<String> get child => $composableBuilder(
    column: $table.child,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<String> get type => $composableBuilder(
    column: $table.type,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<double> get position => $composableBuilder(
    column: $table.position,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<int> get logicalTimestamp => $composableBuilder(
    column: $table.logicalTimestamp,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<int> get sequenceNumber => $composableBuilder(
    column: $table.sequenceNumber,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<String> get deviceID => $composableBuilder(
    column: $table.deviceID,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<DateTime> get createdAt => $composableBuilder(
    column: $table.createdAt,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i1.$$ThingsTableOrderingComposer get parent {
    final i1.$$ThingsTableOrderingComposer composer = $composerBuilder(
      composer: this,
      getCurrentColumn: (t) => t.parent,
      referencedTable: i5.ReadDatabaseContainer(
        $db,
      ).resultSet<i1.$ThingsTable>('things'),
      getReferencedColumn: (t) => t.uuid,
      builder:
          (
            joinBuilder, {
            $addJoinBuilderToRootComposer,
            $removeJoinBuilderFromRootComposer,
          }) => i1.$$ThingsTableOrderingComposer(
            $db: $db,
            $table: i5.ReadDatabaseContainer(
              $db,
            ).resultSet<i1.$ThingsTable>('things'),
            $addJoinBuilderToRootComposer: $addJoinBuilderToRootComposer,
            joinBuilder: joinBuilder,
            $removeJoinBuilderFromRootComposer:
                $removeJoinBuilderFromRootComposer,
          ),
    );
    return composer;
  }
}

class $$ChildrenTableAnnotationComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$ChildrenTable> {
  $$ChildrenTableAnnotationComposer({
    required super.$db,
    required super.$table,
    super.joinBuilder,
    super.$addJoinBuilderToRootComposer,
    super.$removeJoinBuilderFromRootComposer,
  });
  i0.GeneratedColumn<String> get uuid =>
      $composableBuilder(column: $table.uuid, builder: (column) => column);

  i0.GeneratedColumn<String> get child =>
      $composableBuilder(column: $table.child, builder: (column) => column);

  i0.GeneratedColumn<String> get type =>
      $composableBuilder(column: $table.type, builder: (column) => column);

  i0.GeneratedColumn<double> get position =>
      $composableBuilder(column: $table.position, builder: (column) => column);

  i0.GeneratedColumn<int> get logicalTimestamp => $composableBuilder(
    column: $table.logicalTimestamp,
    builder: (column) => column,
  );

  i0.GeneratedColumn<int> get sequenceNumber => $composableBuilder(
    column: $table.sequenceNumber,
    builder: (column) => column,
  );

  i0.GeneratedColumn<String> get deviceID =>
      $composableBuilder(column: $table.deviceID, builder: (column) => column);

  i0.GeneratedColumn<DateTime> get createdAt =>
      $composableBuilder(column: $table.createdAt, builder: (column) => column);

  i1.$$ThingsTableAnnotationComposer get parent {
    final i1.$$ThingsTableAnnotationComposer composer = $composerBuilder(
      composer: this,
      getCurrentColumn: (t) => t.parent,
      referencedTable: i5.ReadDatabaseContainer(
        $db,
      ).resultSet<i1.$ThingsTable>('things'),
      getReferencedColumn: (t) => t.uuid,
      builder:
          (
            joinBuilder, {
            $addJoinBuilderToRootComposer,
            $removeJoinBuilderFromRootComposer,
          }) => i1.$$ThingsTableAnnotationComposer(
            $db: $db,
            $table: i5.ReadDatabaseContainer(
              $db,
            ).resultSet<i1.$ThingsTable>('things'),
            $addJoinBuilderToRootComposer: $addJoinBuilderToRootComposer,
            joinBuilder: joinBuilder,
            $removeJoinBuilderFromRootComposer:
                $removeJoinBuilderFromRootComposer,
          ),
    );
    return composer;
  }
}

class $$ChildrenTableTableManager
    extends
        i0.RootTableManager<
          i0.GeneratedDatabase,
          i1.$ChildrenTable,
          i1.Child,
          i1.$$ChildrenTableFilterComposer,
          i1.$$ChildrenTableOrderingComposer,
          i1.$$ChildrenTableAnnotationComposer,
          $$ChildrenTableCreateCompanionBuilder,
          $$ChildrenTableUpdateCompanionBuilder,
          (i1.Child, i1.$$ChildrenTableReferences),
          i1.Child,
          i0.PrefetchHooks Function({bool parent})
        > {
  $$ChildrenTableTableManager(i0.GeneratedDatabase db, i1.$ChildrenTable table)
    : super(
        i0.TableManagerState(
          db: db,
          table: table,
          createFilteringComposer:
              () => i1.$$ChildrenTableFilterComposer($db: db, $table: table),
          createOrderingComposer:
              () => i1.$$ChildrenTableOrderingComposer($db: db, $table: table),
          createComputedFieldComposer:
              () =>
                  i1.$$ChildrenTableAnnotationComposer($db: db, $table: table),
          updateCompanionCallback:
              ({
                i0.Value<String> uuid = const i0.Value.absent(),
                i0.Value<String> parent = const i0.Value.absent(),
                i0.Value<String> child = const i0.Value.absent(),
                i0.Value<String> type = const i0.Value.absent(),
                i0.Value<double> position = const i0.Value.absent(),
                i0.Value<int> logicalTimestamp = const i0.Value.absent(),
                i0.Value<int> sequenceNumber = const i0.Value.absent(),
                i0.Value<String> deviceID = const i0.Value.absent(),
                i0.Value<DateTime> createdAt = const i0.Value.absent(),
                i0.Value<int> rowid = const i0.Value.absent(),
              }) => i1.ChildrenCompanion(
                uuid: uuid,
                parent: parent,
                child: child,
                type: type,
                position: position,
                logicalTimestamp: logicalTimestamp,
                sequenceNumber: sequenceNumber,
                deviceID: deviceID,
                createdAt: createdAt,
                rowid: rowid,
              ),
          createCompanionCallback:
              ({
                required String uuid,
                required String parent,
                required String child,
                required String type,
                required double position,
                required int logicalTimestamp,
                required int sequenceNumber,
                required String deviceID,
                i0.Value<DateTime> createdAt = const i0.Value.absent(),
                i0.Value<int> rowid = const i0.Value.absent(),
              }) => i1.ChildrenCompanion.insert(
                uuid: uuid,
                parent: parent,
                child: child,
                type: type,
                position: position,
                logicalTimestamp: logicalTimestamp,
                sequenceNumber: sequenceNumber,
                deviceID: deviceID,
                createdAt: createdAt,
                rowid: rowid,
              ),
          withReferenceMapper:
              (p0) =>
                  p0
                      .map(
                        (e) => (
                          e.readTable(table),
                          i1.$$ChildrenTableReferences(db, table, e),
                        ),
                      )
                      .toList(),
          prefetchHooksCallback: ({parent = false}) {
            return i0.PrefetchHooks(
              db: db,
              explicitlyWatchedTables: [],
              addJoins: <
                T extends i0.TableManagerState<
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic,
                  dynamic
                >
              >(state) {
                if (parent) {
                  state =
                      state.withJoin(
                            currentTable: table,
                            currentColumn: table.parent,
                            referencedTable: i1.$$ChildrenTableReferences
                                ._parentTable(db),
                            referencedColumn:
                                i1.$$ChildrenTableReferences
                                    ._parentTable(db)
                                    .uuid,
                          )
                          as T;
                }

                return state;
              },
              getPrefetchedDataCallback: (items) async {
                return [];
              },
            );
          },
        ),
      );
}

typedef $$ChildrenTableProcessedTableManager =
    i0.ProcessedTableManager<
      i0.GeneratedDatabase,
      i1.$ChildrenTable,
      i1.Child,
      i1.$$ChildrenTableFilterComposer,
      i1.$$ChildrenTableOrderingComposer,
      i1.$$ChildrenTableAnnotationComposer,
      $$ChildrenTableCreateCompanionBuilder,
      $$ChildrenTableUpdateCompanionBuilder,
      (i1.Child, i1.$$ChildrenTableReferences),
      i1.Child,
      i0.PrefetchHooks Function({bool parent})
    >;

class $ThingsTable extends i2.Things with i0.TableInfo<$ThingsTable, i1.Thing> {
  @override
  final i0.GeneratedDatabase attachedDatabase;
  final String? _alias;
  $ThingsTable(this.attachedDatabase, [this._alias]);
  static const i0.VerificationMeta _uuidMeta = const i0.VerificationMeta(
    'uuid',
  );
  @override
  late final i0.GeneratedColumn<String> uuid = i0.GeneratedColumn<String>(
    'uuid',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _deletedMeta = const i0.VerificationMeta(
    'deleted',
  );
  @override
  late final i0.GeneratedColumn<bool> deleted = i0.GeneratedColumn<bool>(
    'deleted',
    aliasedName,
    false,
    type: i0.DriftSqlType.bool,
    requiredDuringInsert: false,
    defaultConstraints: i0.GeneratedColumn.constraintIsAlways(
      'CHECK ("deleted" IN (0, 1))',
    ),
    clientDefault: () => false,
  );
  static const i0.VerificationMeta _archivedMeta = const i0.VerificationMeta(
    'archived',
  );
  @override
  late final i0.GeneratedColumn<bool> archived = i0.GeneratedColumn<bool>(
    'archived',
    aliasedName,
    false,
    type: i0.DriftSqlType.bool,
    requiredDuringInsert: false,
    defaultConstraints: i0.GeneratedColumn.constraintIsAlways(
      'CHECK ("archived" IN (0, 1))',
    ),
    clientDefault: () => false,
  );
  static const i0.VerificationMeta _createdAtMeta = const i0.VerificationMeta(
    'createdAt',
  );
  @override
  late final i0.GeneratedColumn<DateTime> createdAt =
      i0.GeneratedColumn<DateTime>(
        'created_at',
        aliasedName,
        false,
        type: i0.DriftSqlType.dateTime,
        requiredDuringInsert: false,
        clientDefault: () => DateTime.now(),
      );
  static const i0.VerificationMeta _updatedAtMeta = const i0.VerificationMeta(
    'updatedAt',
  );
  @override
  late final i0.GeneratedColumn<DateTime> updatedAt =
      i0.GeneratedColumn<DateTime>(
        'updated_at',
        aliasedName,
        false,
        type: i0.DriftSqlType.dateTime,
        requiredDuringInsert: false,
        clientDefault: () => DateTime.now(),
      );
  static const i0.VerificationMeta _titleMeta = const i0.VerificationMeta(
    'title',
  );
  @override
  late final i0.GeneratedColumn<String> title = i0.GeneratedColumn<String>(
    'title',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _schemaMeta = const i0.VerificationMeta(
    'schema',
  );
  @override
  late final i0.GeneratedColumn<String> schema = i0.GeneratedColumn<String>(
    'schema',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
    defaultConstraints: i0.GeneratedColumn.constraintIsAlways(
      'REFERENCES schemas (uuid)',
    ),
  );
  @override
  late final i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  properties = i0.GeneratedColumn<String>(
    'properties',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  ).withConverter<Map<String, dynamic>>(i1.$ThingsTable.$converterproperties);
  @override
  late final i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  metadata = i0.GeneratedColumn<String>(
    'metadata',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: false,
    clientDefault: () => '{}',
  ).withConverter<Map<String, dynamic>>(i1.$ThingsTable.$convertermetadata);
  @override
  List<i0.GeneratedColumn> get $columns => [
    uuid,
    deleted,
    archived,
    createdAt,
    updatedAt,
    title,
    schema,
    properties,
    metadata,
  ];
  @override
  String get aliasedName => _alias ?? actualTableName;
  @override
  String get actualTableName => $name;
  static const String $name = 'things';
  @override
  i0.VerificationContext validateIntegrity(
    i0.Insertable<i1.Thing> instance, {
    bool isInserting = false,
  }) {
    final context = i0.VerificationContext();
    final data = instance.toColumns(true);
    if (data.containsKey('uuid')) {
      context.handle(
        _uuidMeta,
        uuid.isAcceptableOrUnknown(data['uuid']!, _uuidMeta),
      );
    } else if (isInserting) {
      context.missing(_uuidMeta);
    }
    if (data.containsKey('deleted')) {
      context.handle(
        _deletedMeta,
        deleted.isAcceptableOrUnknown(data['deleted']!, _deletedMeta),
      );
    }
    if (data.containsKey('archived')) {
      context.handle(
        _archivedMeta,
        archived.isAcceptableOrUnknown(data['archived']!, _archivedMeta),
      );
    }
    if (data.containsKey('created_at')) {
      context.handle(
        _createdAtMeta,
        createdAt.isAcceptableOrUnknown(data['created_at']!, _createdAtMeta),
      );
    }
    if (data.containsKey('updated_at')) {
      context.handle(
        _updatedAtMeta,
        updatedAt.isAcceptableOrUnknown(data['updated_at']!, _updatedAtMeta),
      );
    }
    if (data.containsKey('title')) {
      context.handle(
        _titleMeta,
        title.isAcceptableOrUnknown(data['title']!, _titleMeta),
      );
    } else if (isInserting) {
      context.missing(_titleMeta);
    }
    if (data.containsKey('schema')) {
      context.handle(
        _schemaMeta,
        schema.isAcceptableOrUnknown(data['schema']!, _schemaMeta),
      );
    } else if (isInserting) {
      context.missing(_schemaMeta);
    }
    return context;
  }

  @override
  Set<i0.GeneratedColumn> get $primaryKey => {uuid};
  @override
  i1.Thing map(Map<String, dynamic> data, {String? tablePrefix}) {
    final effectivePrefix = tablePrefix != null ? '$tablePrefix.' : '';
    return i1.Thing(
      uuid:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}uuid'],
          )!,
      deleted:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.bool,
            data['${effectivePrefix}deleted'],
          )!,
      archived:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.bool,
            data['${effectivePrefix}archived'],
          )!,
      createdAt:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.dateTime,
            data['${effectivePrefix}created_at'],
          )!,
      updatedAt:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.dateTime,
            data['${effectivePrefix}updated_at'],
          )!,
      title:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}title'],
          )!,
      schema:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}schema'],
          )!,
      properties: i1.$ThingsTable.$converterproperties.fromSql(
        attachedDatabase.typeMapping.read(
          i0.DriftSqlType.string,
          data['${effectivePrefix}properties'],
        )!,
      ),
      metadata: i1.$ThingsTable.$convertermetadata.fromSql(
        attachedDatabase.typeMapping.read(
          i0.DriftSqlType.string,
          data['${effectivePrefix}metadata'],
        )!,
      ),
    );
  }

  @override
  $ThingsTable createAlias(String alias) {
    return $ThingsTable(attachedDatabase, alias);
  }

  static i0.JsonTypeConverter2<
    Map<String, dynamic>,
    String,
    Map<String, dynamic>
  >
  $converterproperties = const i3.JsonConverter();
  static i0.JsonTypeConverter2<
    Map<String, dynamic>,
    String,
    Map<String, dynamic>
  >
  $convertermetadata = const i3.JsonConverter();
}

class Thing extends i0.DataClass implements i0.Insertable<i1.Thing> {
  final String uuid;
  final bool deleted;
  final bool archived;
  final DateTime createdAt;
  final DateTime updatedAt;
  final String title;
  final String schema;
  final Map<String, dynamic> properties;
  final Map<String, dynamic> metadata;
  const Thing({
    required this.uuid,
    required this.deleted,
    required this.archived,
    required this.createdAt,
    required this.updatedAt,
    required this.title,
    required this.schema,
    required this.properties,
    required this.metadata,
  });
  @override
  Map<String, i0.Expression> toColumns(bool nullToAbsent) {
    final map = <String, i0.Expression>{};
    map['uuid'] = i0.Variable<String>(uuid);
    map['deleted'] = i0.Variable<bool>(deleted);
    map['archived'] = i0.Variable<bool>(archived);
    map['created_at'] = i0.Variable<DateTime>(createdAt);
    map['updated_at'] = i0.Variable<DateTime>(updatedAt);
    map['title'] = i0.Variable<String>(title);
    map['schema'] = i0.Variable<String>(schema);
    {
      map['properties'] = i0.Variable<String>(
        i1.$ThingsTable.$converterproperties.toSql(properties),
      );
    }
    {
      map['metadata'] = i0.Variable<String>(
        i1.$ThingsTable.$convertermetadata.toSql(metadata),
      );
    }
    return map;
  }

  i1.ThingsCompanion toCompanion(bool nullToAbsent) {
    return i1.ThingsCompanion(
      uuid: i0.Value(uuid),
      deleted: i0.Value(deleted),
      archived: i0.Value(archived),
      createdAt: i0.Value(createdAt),
      updatedAt: i0.Value(updatedAt),
      title: i0.Value(title),
      schema: i0.Value(schema),
      properties: i0.Value(properties),
      metadata: i0.Value(metadata),
    );
  }

  factory Thing.fromJson(
    Map<String, dynamic> json, {
    i0.ValueSerializer? serializer,
  }) {
    serializer ??= i0.driftRuntimeOptions.defaultSerializer;
    return Thing(
      uuid: serializer.fromJson<String>(json['uuid']),
      deleted: serializer.fromJson<bool>(json['deleted']),
      archived: serializer.fromJson<bool>(json['archived']),
      createdAt: serializer.fromJson<DateTime>(json['createdAt']),
      updatedAt: serializer.fromJson<DateTime>(json['updatedAt']),
      title: serializer.fromJson<String>(json['title']),
      schema: serializer.fromJson<String>(json['schema']),
      properties: i1.$ThingsTable.$converterproperties.fromJson(
        serializer.fromJson<Map<String, dynamic>>(json['properties']),
      ),
      metadata: i1.$ThingsTable.$convertermetadata.fromJson(
        serializer.fromJson<Map<String, dynamic>>(json['metadata']),
      ),
    );
  }
  @override
  Map<String, dynamic> toJson({i0.ValueSerializer? serializer}) {
    serializer ??= i0.driftRuntimeOptions.defaultSerializer;
    return <String, dynamic>{
      'uuid': serializer.toJson<String>(uuid),
      'deleted': serializer.toJson<bool>(deleted),
      'archived': serializer.toJson<bool>(archived),
      'createdAt': serializer.toJson<DateTime>(createdAt),
      'updatedAt': serializer.toJson<DateTime>(updatedAt),
      'title': serializer.toJson<String>(title),
      'schema': serializer.toJson<String>(schema),
      'properties': serializer.toJson<Map<String, dynamic>>(
        i1.$ThingsTable.$converterproperties.toJson(properties),
      ),
      'metadata': serializer.toJson<Map<String, dynamic>>(
        i1.$ThingsTable.$convertermetadata.toJson(metadata),
      ),
    };
  }

  i1.Thing copyWith({
    String? uuid,
    bool? deleted,
    bool? archived,
    DateTime? createdAt,
    DateTime? updatedAt,
    String? title,
    String? schema,
    Map<String, dynamic>? properties,
    Map<String, dynamic>? metadata,
  }) => i1.Thing(
    uuid: uuid ?? this.uuid,
    deleted: deleted ?? this.deleted,
    archived: archived ?? this.archived,
    createdAt: createdAt ?? this.createdAt,
    updatedAt: updatedAt ?? this.updatedAt,
    title: title ?? this.title,
    schema: schema ?? this.schema,
    properties: properties ?? this.properties,
    metadata: metadata ?? this.metadata,
  );
  Thing copyWithCompanion(i1.ThingsCompanion data) {
    return Thing(
      uuid: data.uuid.present ? data.uuid.value : this.uuid,
      deleted: data.deleted.present ? data.deleted.value : this.deleted,
      archived: data.archived.present ? data.archived.value : this.archived,
      createdAt: data.createdAt.present ? data.createdAt.value : this.createdAt,
      updatedAt: data.updatedAt.present ? data.updatedAt.value : this.updatedAt,
      title: data.title.present ? data.title.value : this.title,
      schema: data.schema.present ? data.schema.value : this.schema,
      properties:
          data.properties.present ? data.properties.value : this.properties,
      metadata: data.metadata.present ? data.metadata.value : this.metadata,
    );
  }

  @override
  String toString() {
    return (StringBuffer('Thing(')
          ..write('uuid: $uuid, ')
          ..write('deleted: $deleted, ')
          ..write('archived: $archived, ')
          ..write('createdAt: $createdAt, ')
          ..write('updatedAt: $updatedAt, ')
          ..write('title: $title, ')
          ..write('schema: $schema, ')
          ..write('properties: $properties, ')
          ..write('metadata: $metadata')
          ..write(')'))
        .toString();
  }

  @override
  int get hashCode => Object.hash(
    uuid,
    deleted,
    archived,
    createdAt,
    updatedAt,
    title,
    schema,
    properties,
    metadata,
  );
  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is i1.Thing &&
          other.uuid == this.uuid &&
          other.deleted == this.deleted &&
          other.archived == this.archived &&
          other.createdAt == this.createdAt &&
          other.updatedAt == this.updatedAt &&
          other.title == this.title &&
          other.schema == this.schema &&
          other.properties == this.properties &&
          other.metadata == this.metadata);
}

class ThingsCompanion extends i0.UpdateCompanion<i1.Thing> {
  final i0.Value<String> uuid;
  final i0.Value<bool> deleted;
  final i0.Value<bool> archived;
  final i0.Value<DateTime> createdAt;
  final i0.Value<DateTime> updatedAt;
  final i0.Value<String> title;
  final i0.Value<String> schema;
  final i0.Value<Map<String, dynamic>> properties;
  final i0.Value<Map<String, dynamic>> metadata;
  final i0.Value<int> rowid;
  const ThingsCompanion({
    this.uuid = const i0.Value.absent(),
    this.deleted = const i0.Value.absent(),
    this.archived = const i0.Value.absent(),
    this.createdAt = const i0.Value.absent(),
    this.updatedAt = const i0.Value.absent(),
    this.title = const i0.Value.absent(),
    this.schema = const i0.Value.absent(),
    this.properties = const i0.Value.absent(),
    this.metadata = const i0.Value.absent(),
    this.rowid = const i0.Value.absent(),
  });
  ThingsCompanion.insert({
    required String uuid,
    this.deleted = const i0.Value.absent(),
    this.archived = const i0.Value.absent(),
    this.createdAt = const i0.Value.absent(),
    this.updatedAt = const i0.Value.absent(),
    required String title,
    required String schema,
    required Map<String, dynamic> properties,
    this.metadata = const i0.Value.absent(),
    this.rowid = const i0.Value.absent(),
  }) : uuid = i0.Value(uuid),
       title = i0.Value(title),
       schema = i0.Value(schema),
       properties = i0.Value(properties);
  static i0.Insertable<i1.Thing> custom({
    i0.Expression<String>? uuid,
    i0.Expression<bool>? deleted,
    i0.Expression<bool>? archived,
    i0.Expression<DateTime>? createdAt,
    i0.Expression<DateTime>? updatedAt,
    i0.Expression<String>? title,
    i0.Expression<String>? schema,
    i0.Expression<String>? properties,
    i0.Expression<String>? metadata,
    i0.Expression<int>? rowid,
  }) {
    return i0.RawValuesInsertable({
      if (uuid != null) 'uuid': uuid,
      if (deleted != null) 'deleted': deleted,
      if (archived != null) 'archived': archived,
      if (createdAt != null) 'created_at': createdAt,
      if (updatedAt != null) 'updated_at': updatedAt,
      if (title != null) 'title': title,
      if (schema != null) 'schema': schema,
      if (properties != null) 'properties': properties,
      if (metadata != null) 'metadata': metadata,
      if (rowid != null) 'rowid': rowid,
    });
  }

  i1.ThingsCompanion copyWith({
    i0.Value<String>? uuid,
    i0.Value<bool>? deleted,
    i0.Value<bool>? archived,
    i0.Value<DateTime>? createdAt,
    i0.Value<DateTime>? updatedAt,
    i0.Value<String>? title,
    i0.Value<String>? schema,
    i0.Value<Map<String, dynamic>>? properties,
    i0.Value<Map<String, dynamic>>? metadata,
    i0.Value<int>? rowid,
  }) {
    return i1.ThingsCompanion(
      uuid: uuid ?? this.uuid,
      deleted: deleted ?? this.deleted,
      archived: archived ?? this.archived,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
      title: title ?? this.title,
      schema: schema ?? this.schema,
      properties: properties ?? this.properties,
      metadata: metadata ?? this.metadata,
      rowid: rowid ?? this.rowid,
    );
  }

  @override
  Map<String, i0.Expression> toColumns(bool nullToAbsent) {
    final map = <String, i0.Expression>{};
    if (uuid.present) {
      map['uuid'] = i0.Variable<String>(uuid.value);
    }
    if (deleted.present) {
      map['deleted'] = i0.Variable<bool>(deleted.value);
    }
    if (archived.present) {
      map['archived'] = i0.Variable<bool>(archived.value);
    }
    if (createdAt.present) {
      map['created_at'] = i0.Variable<DateTime>(createdAt.value);
    }
    if (updatedAt.present) {
      map['updated_at'] = i0.Variable<DateTime>(updatedAt.value);
    }
    if (title.present) {
      map['title'] = i0.Variable<String>(title.value);
    }
    if (schema.present) {
      map['schema'] = i0.Variable<String>(schema.value);
    }
    if (properties.present) {
      map['properties'] = i0.Variable<String>(
        i1.$ThingsTable.$converterproperties.toSql(properties.value),
      );
    }
    if (metadata.present) {
      map['metadata'] = i0.Variable<String>(
        i1.$ThingsTable.$convertermetadata.toSql(metadata.value),
      );
    }
    if (rowid.present) {
      map['rowid'] = i0.Variable<int>(rowid.value);
    }
    return map;
  }

  @override
  String toString() {
    return (StringBuffer('ThingsCompanion(')
          ..write('uuid: $uuid, ')
          ..write('deleted: $deleted, ')
          ..write('archived: $archived, ')
          ..write('createdAt: $createdAt, ')
          ..write('updatedAt: $updatedAt, ')
          ..write('title: $title, ')
          ..write('schema: $schema, ')
          ..write('properties: $properties, ')
          ..write('metadata: $metadata, ')
          ..write('rowid: $rowid')
          ..write(')'))
        .toString();
  }
}

class $ChildrenTable extends i2.Children
    with i0.TableInfo<$ChildrenTable, i1.Child> {
  @override
  final i0.GeneratedDatabase attachedDatabase;
  final String? _alias;
  $ChildrenTable(this.attachedDatabase, [this._alias]);
  static const i0.VerificationMeta _uuidMeta = const i0.VerificationMeta(
    'uuid',
  );
  @override
  late final i0.GeneratedColumn<String> uuid = i0.GeneratedColumn<String>(
    'uuid',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _parentMeta = const i0.VerificationMeta(
    'parent',
  );
  @override
  late final i0.GeneratedColumn<String> parent = i0.GeneratedColumn<String>(
    'parent',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
    defaultConstraints: i0.GeneratedColumn.constraintIsAlways(
      'REFERENCES things (uuid) ON DELETE CASCADE',
    ),
  );
  static const i0.VerificationMeta _childMeta = const i0.VerificationMeta(
    'child',
  );
  @override
  late final i0.GeneratedColumn<String> child = i0.GeneratedColumn<String>(
    'child',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _typeMeta = const i0.VerificationMeta(
    'type',
  );
  @override
  late final i0.GeneratedColumn<String> type = i0.GeneratedColumn<String>(
    'type',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _positionMeta = const i0.VerificationMeta(
    'position',
  );
  @override
  late final i0.GeneratedColumn<double> position = i0.GeneratedColumn<double>(
    'position',
    aliasedName,
    false,
    type: i0.DriftSqlType.double,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _logicalTimestampMeta =
      const i0.VerificationMeta('logicalTimestamp');
  @override
  late final i0.GeneratedColumn<int> logicalTimestamp = i0.GeneratedColumn<int>(
    'logical_timestamp',
    aliasedName,
    false,
    type: i0.DriftSqlType.int,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _sequenceNumberMeta =
      const i0.VerificationMeta('sequenceNumber');
  @override
  late final i0.GeneratedColumn<int> sequenceNumber = i0.GeneratedColumn<int>(
    'sequence_number',
    aliasedName,
    false,
    type: i0.DriftSqlType.int,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _deviceIDMeta = const i0.VerificationMeta(
    'deviceID',
  );
  @override
  late final i0.GeneratedColumn<String> deviceID = i0.GeneratedColumn<String>(
    'device_i_d',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _createdAtMeta = const i0.VerificationMeta(
    'createdAt',
  );
  @override
  late final i0.GeneratedColumn<DateTime> createdAt =
      i0.GeneratedColumn<DateTime>(
        'created_at',
        aliasedName,
        false,
        type: i0.DriftSqlType.dateTime,
        requiredDuringInsert: false,
        clientDefault: () => DateTime.now(),
      );
  @override
  List<i0.GeneratedColumn> get $columns => [
    uuid,
    parent,
    child,
    type,
    position,
    logicalTimestamp,
    sequenceNumber,
    deviceID,
    createdAt,
  ];
  @override
  String get aliasedName => _alias ?? actualTableName;
  @override
  String get actualTableName => $name;
  static const String $name = 'children';
  @override
  i0.VerificationContext validateIntegrity(
    i0.Insertable<i1.Child> instance, {
    bool isInserting = false,
  }) {
    final context = i0.VerificationContext();
    final data = instance.toColumns(true);
    if (data.containsKey('uuid')) {
      context.handle(
        _uuidMeta,
        uuid.isAcceptableOrUnknown(data['uuid']!, _uuidMeta),
      );
    } else if (isInserting) {
      context.missing(_uuidMeta);
    }
    if (data.containsKey('parent')) {
      context.handle(
        _parentMeta,
        parent.isAcceptableOrUnknown(data['parent']!, _parentMeta),
      );
    } else if (isInserting) {
      context.missing(_parentMeta);
    }
    if (data.containsKey('child')) {
      context.handle(
        _childMeta,
        child.isAcceptableOrUnknown(data['child']!, _childMeta),
      );
    } else if (isInserting) {
      context.missing(_childMeta);
    }
    if (data.containsKey('type')) {
      context.handle(
        _typeMeta,
        type.isAcceptableOrUnknown(data['type']!, _typeMeta),
      );
    } else if (isInserting) {
      context.missing(_typeMeta);
    }
    if (data.containsKey('position')) {
      context.handle(
        _positionMeta,
        position.isAcceptableOrUnknown(data['position']!, _positionMeta),
      );
    } else if (isInserting) {
      context.missing(_positionMeta);
    }
    if (data.containsKey('logical_timestamp')) {
      context.handle(
        _logicalTimestampMeta,
        logicalTimestamp.isAcceptableOrUnknown(
          data['logical_timestamp']!,
          _logicalTimestampMeta,
        ),
      );
    } else if (isInserting) {
      context.missing(_logicalTimestampMeta);
    }
    if (data.containsKey('sequence_number')) {
      context.handle(
        _sequenceNumberMeta,
        sequenceNumber.isAcceptableOrUnknown(
          data['sequence_number']!,
          _sequenceNumberMeta,
        ),
      );
    } else if (isInserting) {
      context.missing(_sequenceNumberMeta);
    }
    if (data.containsKey('device_i_d')) {
      context.handle(
        _deviceIDMeta,
        deviceID.isAcceptableOrUnknown(data['device_i_d']!, _deviceIDMeta),
      );
    } else if (isInserting) {
      context.missing(_deviceIDMeta);
    }
    if (data.containsKey('created_at')) {
      context.handle(
        _createdAtMeta,
        createdAt.isAcceptableOrUnknown(data['created_at']!, _createdAtMeta),
      );
    }
    return context;
  }

  @override
  Set<i0.GeneratedColumn> get $primaryKey => {uuid};
  @override
  List<Set<i0.GeneratedColumn>> get uniqueKeys => [
    {parent, position},
  ];
  @override
  i1.Child map(Map<String, dynamic> data, {String? tablePrefix}) {
    final effectivePrefix = tablePrefix != null ? '$tablePrefix.' : '';
    return i1.Child(
      uuid:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}uuid'],
          )!,
      parent:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}parent'],
          )!,
      child:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}child'],
          )!,
      type:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}type'],
          )!,
      position:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.double,
            data['${effectivePrefix}position'],
          )!,
      logicalTimestamp:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.int,
            data['${effectivePrefix}logical_timestamp'],
          )!,
      sequenceNumber:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.int,
            data['${effectivePrefix}sequence_number'],
          )!,
      deviceID:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}device_i_d'],
          )!,
      createdAt:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.dateTime,
            data['${effectivePrefix}created_at'],
          )!,
    );
  }

  @override
  $ChildrenTable createAlias(String alias) {
    return $ChildrenTable(attachedDatabase, alias);
  }
}

class Child extends i0.DataClass implements i0.Insertable<i1.Child> {
  final String uuid;
  final String parent;
  final String child;
  final String type;
  final double position;
  final int logicalTimestamp;
  final int sequenceNumber;
  final String deviceID;
  final DateTime createdAt;
  const Child({
    required this.uuid,
    required this.parent,
    required this.child,
    required this.type,
    required this.position,
    required this.logicalTimestamp,
    required this.sequenceNumber,
    required this.deviceID,
    required this.createdAt,
  });
  @override
  Map<String, i0.Expression> toColumns(bool nullToAbsent) {
    final map = <String, i0.Expression>{};
    map['uuid'] = i0.Variable<String>(uuid);
    map['parent'] = i0.Variable<String>(parent);
    map['child'] = i0.Variable<String>(child);
    map['type'] = i0.Variable<String>(type);
    map['position'] = i0.Variable<double>(position);
    map['logical_timestamp'] = i0.Variable<int>(logicalTimestamp);
    map['sequence_number'] = i0.Variable<int>(sequenceNumber);
    map['device_i_d'] = i0.Variable<String>(deviceID);
    map['created_at'] = i0.Variable<DateTime>(createdAt);
    return map;
  }

  i1.ChildrenCompanion toCompanion(bool nullToAbsent) {
    return i1.ChildrenCompanion(
      uuid: i0.Value(uuid),
      parent: i0.Value(parent),
      child: i0.Value(child),
      type: i0.Value(type),
      position: i0.Value(position),
      logicalTimestamp: i0.Value(logicalTimestamp),
      sequenceNumber: i0.Value(sequenceNumber),
      deviceID: i0.Value(deviceID),
      createdAt: i0.Value(createdAt),
    );
  }

  factory Child.fromJson(
    Map<String, dynamic> json, {
    i0.ValueSerializer? serializer,
  }) {
    serializer ??= i0.driftRuntimeOptions.defaultSerializer;
    return Child(
      uuid: serializer.fromJson<String>(json['uuid']),
      parent: serializer.fromJson<String>(json['parent']),
      child: serializer.fromJson<String>(json['child']),
      type: serializer.fromJson<String>(json['type']),
      position: serializer.fromJson<double>(json['position']),
      logicalTimestamp: serializer.fromJson<int>(json['logicalTimestamp']),
      sequenceNumber: serializer.fromJson<int>(json['sequenceNumber']),
      deviceID: serializer.fromJson<String>(json['deviceID']),
      createdAt: serializer.fromJson<DateTime>(json['createdAt']),
    );
  }
  @override
  Map<String, dynamic> toJson({i0.ValueSerializer? serializer}) {
    serializer ??= i0.driftRuntimeOptions.defaultSerializer;
    return <String, dynamic>{
      'uuid': serializer.toJson<String>(uuid),
      'parent': serializer.toJson<String>(parent),
      'child': serializer.toJson<String>(child),
      'type': serializer.toJson<String>(type),
      'position': serializer.toJson<double>(position),
      'logicalTimestamp': serializer.toJson<int>(logicalTimestamp),
      'sequenceNumber': serializer.toJson<int>(sequenceNumber),
      'deviceID': serializer.toJson<String>(deviceID),
      'createdAt': serializer.toJson<DateTime>(createdAt),
    };
  }

  i1.Child copyWith({
    String? uuid,
    String? parent,
    String? child,
    String? type,
    double? position,
    int? logicalTimestamp,
    int? sequenceNumber,
    String? deviceID,
    DateTime? createdAt,
  }) => i1.Child(
    uuid: uuid ?? this.uuid,
    parent: parent ?? this.parent,
    child: child ?? this.child,
    type: type ?? this.type,
    position: position ?? this.position,
    logicalTimestamp: logicalTimestamp ?? this.logicalTimestamp,
    sequenceNumber: sequenceNumber ?? this.sequenceNumber,
    deviceID: deviceID ?? this.deviceID,
    createdAt: createdAt ?? this.createdAt,
  );
  Child copyWithCompanion(i1.ChildrenCompanion data) {
    return Child(
      uuid: data.uuid.present ? data.uuid.value : this.uuid,
      parent: data.parent.present ? data.parent.value : this.parent,
      child: data.child.present ? data.child.value : this.child,
      type: data.type.present ? data.type.value : this.type,
      position: data.position.present ? data.position.value : this.position,
      logicalTimestamp:
          data.logicalTimestamp.present
              ? data.logicalTimestamp.value
              : this.logicalTimestamp,
      sequenceNumber:
          data.sequenceNumber.present
              ? data.sequenceNumber.value
              : this.sequenceNumber,
      deviceID: data.deviceID.present ? data.deviceID.value : this.deviceID,
      createdAt: data.createdAt.present ? data.createdAt.value : this.createdAt,
    );
  }

  @override
  String toString() {
    return (StringBuffer('Child(')
          ..write('uuid: $uuid, ')
          ..write('parent: $parent, ')
          ..write('child: $child, ')
          ..write('type: $type, ')
          ..write('position: $position, ')
          ..write('logicalTimestamp: $logicalTimestamp, ')
          ..write('sequenceNumber: $sequenceNumber, ')
          ..write('deviceID: $deviceID, ')
          ..write('createdAt: $createdAt')
          ..write(')'))
        .toString();
  }

  @override
  int get hashCode => Object.hash(
    uuid,
    parent,
    child,
    type,
    position,
    logicalTimestamp,
    sequenceNumber,
    deviceID,
    createdAt,
  );
  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is i1.Child &&
          other.uuid == this.uuid &&
          other.parent == this.parent &&
          other.child == this.child &&
          other.type == this.type &&
          other.position == this.position &&
          other.logicalTimestamp == this.logicalTimestamp &&
          other.sequenceNumber == this.sequenceNumber &&
          other.deviceID == this.deviceID &&
          other.createdAt == this.createdAt);
}

class ChildrenCompanion extends i0.UpdateCompanion<i1.Child> {
  final i0.Value<String> uuid;
  final i0.Value<String> parent;
  final i0.Value<String> child;
  final i0.Value<String> type;
  final i0.Value<double> position;
  final i0.Value<int> logicalTimestamp;
  final i0.Value<int> sequenceNumber;
  final i0.Value<String> deviceID;
  final i0.Value<DateTime> createdAt;
  final i0.Value<int> rowid;
  const ChildrenCompanion({
    this.uuid = const i0.Value.absent(),
    this.parent = const i0.Value.absent(),
    this.child = const i0.Value.absent(),
    this.type = const i0.Value.absent(),
    this.position = const i0.Value.absent(),
    this.logicalTimestamp = const i0.Value.absent(),
    this.sequenceNumber = const i0.Value.absent(),
    this.deviceID = const i0.Value.absent(),
    this.createdAt = const i0.Value.absent(),
    this.rowid = const i0.Value.absent(),
  });
  ChildrenCompanion.insert({
    required String uuid,
    required String parent,
    required String child,
    required String type,
    required double position,
    required int logicalTimestamp,
    required int sequenceNumber,
    required String deviceID,
    this.createdAt = const i0.Value.absent(),
    this.rowid = const i0.Value.absent(),
  }) : uuid = i0.Value(uuid),
       parent = i0.Value(parent),
       child = i0.Value(child),
       type = i0.Value(type),
       position = i0.Value(position),
       logicalTimestamp = i0.Value(logicalTimestamp),
       sequenceNumber = i0.Value(sequenceNumber),
       deviceID = i0.Value(deviceID);
  static i0.Insertable<i1.Child> custom({
    i0.Expression<String>? uuid,
    i0.Expression<String>? parent,
    i0.Expression<String>? child,
    i0.Expression<String>? type,
    i0.Expression<double>? position,
    i0.Expression<int>? logicalTimestamp,
    i0.Expression<int>? sequenceNumber,
    i0.Expression<String>? deviceID,
    i0.Expression<DateTime>? createdAt,
    i0.Expression<int>? rowid,
  }) {
    return i0.RawValuesInsertable({
      if (uuid != null) 'uuid': uuid,
      if (parent != null) 'parent': parent,
      if (child != null) 'child': child,
      if (type != null) 'type': type,
      if (position != null) 'position': position,
      if (logicalTimestamp != null) 'logical_timestamp': logicalTimestamp,
      if (sequenceNumber != null) 'sequence_number': sequenceNumber,
      if (deviceID != null) 'device_i_d': deviceID,
      if (createdAt != null) 'created_at': createdAt,
      if (rowid != null) 'rowid': rowid,
    });
  }

  i1.ChildrenCompanion copyWith({
    i0.Value<String>? uuid,
    i0.Value<String>? parent,
    i0.Value<String>? child,
    i0.Value<String>? type,
    i0.Value<double>? position,
    i0.Value<int>? logicalTimestamp,
    i0.Value<int>? sequenceNumber,
    i0.Value<String>? deviceID,
    i0.Value<DateTime>? createdAt,
    i0.Value<int>? rowid,
  }) {
    return i1.ChildrenCompanion(
      uuid: uuid ?? this.uuid,
      parent: parent ?? this.parent,
      child: child ?? this.child,
      type: type ?? this.type,
      position: position ?? this.position,
      logicalTimestamp: logicalTimestamp ?? this.logicalTimestamp,
      sequenceNumber: sequenceNumber ?? this.sequenceNumber,
      deviceID: deviceID ?? this.deviceID,
      createdAt: createdAt ?? this.createdAt,
      rowid: rowid ?? this.rowid,
    );
  }

  @override
  Map<String, i0.Expression> toColumns(bool nullToAbsent) {
    final map = <String, i0.Expression>{};
    if (uuid.present) {
      map['uuid'] = i0.Variable<String>(uuid.value);
    }
    if (parent.present) {
      map['parent'] = i0.Variable<String>(parent.value);
    }
    if (child.present) {
      map['child'] = i0.Variable<String>(child.value);
    }
    if (type.present) {
      map['type'] = i0.Variable<String>(type.value);
    }
    if (position.present) {
      map['position'] = i0.Variable<double>(position.value);
    }
    if (logicalTimestamp.present) {
      map['logical_timestamp'] = i0.Variable<int>(logicalTimestamp.value);
    }
    if (sequenceNumber.present) {
      map['sequence_number'] = i0.Variable<int>(sequenceNumber.value);
    }
    if (deviceID.present) {
      map['device_i_d'] = i0.Variable<String>(deviceID.value);
    }
    if (createdAt.present) {
      map['created_at'] = i0.Variable<DateTime>(createdAt.value);
    }
    if (rowid.present) {
      map['rowid'] = i0.Variable<int>(rowid.value);
    }
    return map;
  }

  @override
  String toString() {
    return (StringBuffer('ChildrenCompanion(')
          ..write('uuid: $uuid, ')
          ..write('parent: $parent, ')
          ..write('child: $child, ')
          ..write('type: $type, ')
          ..write('position: $position, ')
          ..write('logicalTimestamp: $logicalTimestamp, ')
          ..write('sequenceNumber: $sequenceNumber, ')
          ..write('deviceID: $deviceID, ')
          ..write('createdAt: $createdAt, ')
          ..write('rowid: $rowid')
          ..write(')'))
        .toString();
  }
}
