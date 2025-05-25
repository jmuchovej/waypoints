// dart format width=80
// ignore_for_file: type=lint
import 'package:drift/drift.dart' as i0;
import 'package:tempo/src/database/tables/schema.drift.dart' as i1;
import 'package:tempo/src/database/tables/schema.dart' as i2;
import 'package:tempo/src/database/converters.dart' as i3;

typedef $$SchemasTableCreateCompanionBuilder =
    i1.SchemasCompanion Function({
      required String uuid,
      i0.Value<bool> deleted,
      i0.Value<bool> archived,
      i0.Value<DateTime> createdAt,
      i0.Value<DateTime> updatedAt,
      required String name,
      i0.Value<int> version,
      i0.Value<bool> systemDefined,
      required Map<String, dynamic> definition,
      i0.Value<Map<String, dynamic>> display,
      i0.Value<Map<String, dynamic>> indexed,
      i0.Value<int> rowid,
    });
typedef $$SchemasTableUpdateCompanionBuilder =
    i1.SchemasCompanion Function({
      i0.Value<String> uuid,
      i0.Value<bool> deleted,
      i0.Value<bool> archived,
      i0.Value<DateTime> createdAt,
      i0.Value<DateTime> updatedAt,
      i0.Value<String> name,
      i0.Value<int> version,
      i0.Value<bool> systemDefined,
      i0.Value<Map<String, dynamic>> definition,
      i0.Value<Map<String, dynamic>> display,
      i0.Value<Map<String, dynamic>> indexed,
      i0.Value<int> rowid,
    });

class $$SchemasTableFilterComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$SchemasTable> {
  $$SchemasTableFilterComposer({
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

  i0.ColumnFilters<String> get name => $composableBuilder(
    column: $table.name,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<int> get version => $composableBuilder(
    column: $table.version,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnFilters<bool> get systemDefined => $composableBuilder(
    column: $table.systemDefined,
    builder: (column) => i0.ColumnFilters(column),
  );

  i0.ColumnWithTypeConverterFilters<
    Map<String, dynamic>,
    Map<String, dynamic>,
    String
  >
  get definition => $composableBuilder(
    column: $table.definition,
    builder: (column) => i0.ColumnWithTypeConverterFilters(column),
  );

  i0.ColumnWithTypeConverterFilters<
    Map<String, dynamic>,
    Map<String, dynamic>,
    String
  >
  get display => $composableBuilder(
    column: $table.display,
    builder: (column) => i0.ColumnWithTypeConverterFilters(column),
  );

  i0.ColumnWithTypeConverterFilters<
    Map<String, dynamic>,
    Map<String, dynamic>,
    String
  >
  get indexed => $composableBuilder(
    column: $table.indexed,
    builder: (column) => i0.ColumnWithTypeConverterFilters(column),
  );
}

class $$SchemasTableOrderingComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$SchemasTable> {
  $$SchemasTableOrderingComposer({
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

  i0.ColumnOrderings<String> get name => $composableBuilder(
    column: $table.name,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<int> get version => $composableBuilder(
    column: $table.version,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<bool> get systemDefined => $composableBuilder(
    column: $table.systemDefined,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<String> get definition => $composableBuilder(
    column: $table.definition,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<String> get display => $composableBuilder(
    column: $table.display,
    builder: (column) => i0.ColumnOrderings(column),
  );

  i0.ColumnOrderings<String> get indexed => $composableBuilder(
    column: $table.indexed,
    builder: (column) => i0.ColumnOrderings(column),
  );
}

class $$SchemasTableAnnotationComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$SchemasTable> {
  $$SchemasTableAnnotationComposer({
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

  i0.GeneratedColumn<String> get name =>
      $composableBuilder(column: $table.name, builder: (column) => column);

  i0.GeneratedColumn<int> get version =>
      $composableBuilder(column: $table.version, builder: (column) => column);

  i0.GeneratedColumn<bool> get systemDefined => $composableBuilder(
    column: $table.systemDefined,
    builder: (column) => column,
  );

  i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  get definition => $composableBuilder(
    column: $table.definition,
    builder: (column) => column,
  );

  i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  get display =>
      $composableBuilder(column: $table.display, builder: (column) => column);

  i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  get indexed =>
      $composableBuilder(column: $table.indexed, builder: (column) => column);
}

class $$SchemasTableTableManager
    extends
        i0.RootTableManager<
          i0.GeneratedDatabase,
          i1.$SchemasTable,
          i1.Schema,
          i1.$$SchemasTableFilterComposer,
          i1.$$SchemasTableOrderingComposer,
          i1.$$SchemasTableAnnotationComposer,
          $$SchemasTableCreateCompanionBuilder,
          $$SchemasTableUpdateCompanionBuilder,
          (
            i1.Schema,
            i0.BaseReferences<
              i0.GeneratedDatabase,
              i1.$SchemasTable,
              i1.Schema
            >,
          ),
          i1.Schema,
          i0.PrefetchHooks Function()
        > {
  $$SchemasTableTableManager(i0.GeneratedDatabase db, i1.$SchemasTable table)
    : super(
        i0.TableManagerState(
          db: db,
          table: table,
          createFilteringComposer:
              () => i1.$$SchemasTableFilterComposer($db: db, $table: table),
          createOrderingComposer:
              () => i1.$$SchemasTableOrderingComposer($db: db, $table: table),
          createComputedFieldComposer:
              () => i1.$$SchemasTableAnnotationComposer($db: db, $table: table),
          updateCompanionCallback:
              ({
                i0.Value<String> uuid = const i0.Value.absent(),
                i0.Value<bool> deleted = const i0.Value.absent(),
                i0.Value<bool> archived = const i0.Value.absent(),
                i0.Value<DateTime> createdAt = const i0.Value.absent(),
                i0.Value<DateTime> updatedAt = const i0.Value.absent(),
                i0.Value<String> name = const i0.Value.absent(),
                i0.Value<int> version = const i0.Value.absent(),
                i0.Value<bool> systemDefined = const i0.Value.absent(),
                i0.Value<Map<String, dynamic>> definition =
                    const i0.Value.absent(),
                i0.Value<Map<String, dynamic>> display =
                    const i0.Value.absent(),
                i0.Value<Map<String, dynamic>> indexed =
                    const i0.Value.absent(),
                i0.Value<int> rowid = const i0.Value.absent(),
              }) => i1.SchemasCompanion(
                uuid: uuid,
                deleted: deleted,
                archived: archived,
                createdAt: createdAt,
                updatedAt: updatedAt,
                name: name,
                version: version,
                systemDefined: systemDefined,
                definition: definition,
                display: display,
                indexed: indexed,
                rowid: rowid,
              ),
          createCompanionCallback:
              ({
                required String uuid,
                i0.Value<bool> deleted = const i0.Value.absent(),
                i0.Value<bool> archived = const i0.Value.absent(),
                i0.Value<DateTime> createdAt = const i0.Value.absent(),
                i0.Value<DateTime> updatedAt = const i0.Value.absent(),
                required String name,
                i0.Value<int> version = const i0.Value.absent(),
                i0.Value<bool> systemDefined = const i0.Value.absent(),
                required Map<String, dynamic> definition,
                i0.Value<Map<String, dynamic>> display =
                    const i0.Value.absent(),
                i0.Value<Map<String, dynamic>> indexed =
                    const i0.Value.absent(),
                i0.Value<int> rowid = const i0.Value.absent(),
              }) => i1.SchemasCompanion.insert(
                uuid: uuid,
                deleted: deleted,
                archived: archived,
                createdAt: createdAt,
                updatedAt: updatedAt,
                name: name,
                version: version,
                systemDefined: systemDefined,
                definition: definition,
                display: display,
                indexed: indexed,
                rowid: rowid,
              ),
          withReferenceMapper:
              (p0) =>
                  p0
                      .map(
                        (e) => (
                          e.readTable(table),
                          i0.BaseReferences(db, table, e),
                        ),
                      )
                      .toList(),
          prefetchHooksCallback: null,
        ),
      );
}

typedef $$SchemasTableProcessedTableManager =
    i0.ProcessedTableManager<
      i0.GeneratedDatabase,
      i1.$SchemasTable,
      i1.Schema,
      i1.$$SchemasTableFilterComposer,
      i1.$$SchemasTableOrderingComposer,
      i1.$$SchemasTableAnnotationComposer,
      $$SchemasTableCreateCompanionBuilder,
      $$SchemasTableUpdateCompanionBuilder,
      (
        i1.Schema,
        i0.BaseReferences<i0.GeneratedDatabase, i1.$SchemasTable, i1.Schema>,
      ),
      i1.Schema,
      i0.PrefetchHooks Function()
    >;

class $SchemasTable extends i2.Schemas
    with i0.TableInfo<$SchemasTable, i1.Schema> {
  @override
  final i0.GeneratedDatabase attachedDatabase;
  final String? _alias;
  $SchemasTable(this.attachedDatabase, [this._alias]);
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
  static const i0.VerificationMeta _nameMeta = const i0.VerificationMeta(
    'name',
  );
  @override
  late final i0.GeneratedColumn<String> name = i0.GeneratedColumn<String>(
    'name',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  );
  static const i0.VerificationMeta _versionMeta = const i0.VerificationMeta(
    'version',
  );
  @override
  late final i0.GeneratedColumn<int> version = i0.GeneratedColumn<int>(
    'version',
    aliasedName,
    false,
    type: i0.DriftSqlType.int,
    requiredDuringInsert: false,
    clientDefault: () => 1,
  );
  static const i0.VerificationMeta _systemDefinedMeta =
      const i0.VerificationMeta('systemDefined');
  @override
  late final i0.GeneratedColumn<bool> systemDefined = i0.GeneratedColumn<bool>(
    'system_defined',
    aliasedName,
    false,
    type: i0.DriftSqlType.bool,
    requiredDuringInsert: false,
    defaultConstraints: i0.GeneratedColumn.constraintIsAlways(
      'CHECK ("system_defined" IN (0, 1))',
    ),
    clientDefault: () => false,
  );
  @override
  late final i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  definition = i0.GeneratedColumn<String>(
    'definition',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  ).withConverter<Map<String, dynamic>>(i1.$SchemasTable.$converterdefinition);
  @override
  late final i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  display = i0.GeneratedColumn<String>(
    'display',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: false,
    clientDefault: () => '{}',
  ).withConverter<Map<String, dynamic>>(i1.$SchemasTable.$converterdisplay);
  @override
  late final i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  indexed = i0.GeneratedColumn<String>(
    'indexed',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: false,
    clientDefault: () => '{}',
  ).withConverter<Map<String, dynamic>>(i1.$SchemasTable.$converterindexed);
  @override
  List<i0.GeneratedColumn> get $columns => [
    uuid,
    deleted,
    archived,
    createdAt,
    updatedAt,
    name,
    version,
    systemDefined,
    definition,
    display,
    indexed,
  ];
  @override
  String get aliasedName => _alias ?? actualTableName;
  @override
  String get actualTableName => $name;
  static const String $name = 'schemas';
  @override
  i0.VerificationContext validateIntegrity(
    i0.Insertable<i1.Schema> instance, {
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
    if (data.containsKey('name')) {
      context.handle(
        _nameMeta,
        name.isAcceptableOrUnknown(data['name']!, _nameMeta),
      );
    } else if (isInserting) {
      context.missing(_nameMeta);
    }
    if (data.containsKey('version')) {
      context.handle(
        _versionMeta,
        version.isAcceptableOrUnknown(data['version']!, _versionMeta),
      );
    }
    if (data.containsKey('system_defined')) {
      context.handle(
        _systemDefinedMeta,
        systemDefined.isAcceptableOrUnknown(
          data['system_defined']!,
          _systemDefinedMeta,
        ),
      );
    }
    return context;
  }

  @override
  Set<i0.GeneratedColumn> get $primaryKey => {uuid};
  @override
  i1.Schema map(Map<String, dynamic> data, {String? tablePrefix}) {
    final effectivePrefix = tablePrefix != null ? '$tablePrefix.' : '';
    return i1.Schema(
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
      name:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}name'],
          )!,
      version:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.int,
            data['${effectivePrefix}version'],
          )!,
      systemDefined:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.bool,
            data['${effectivePrefix}system_defined'],
          )!,
      definition: i1.$SchemasTable.$converterdefinition.fromSql(
        attachedDatabase.typeMapping.read(
          i0.DriftSqlType.string,
          data['${effectivePrefix}definition'],
        )!,
      ),
      display: i1.$SchemasTable.$converterdisplay.fromSql(
        attachedDatabase.typeMapping.read(
          i0.DriftSqlType.string,
          data['${effectivePrefix}display'],
        )!,
      ),
      indexed: i1.$SchemasTable.$converterindexed.fromSql(
        attachedDatabase.typeMapping.read(
          i0.DriftSqlType.string,
          data['${effectivePrefix}indexed'],
        )!,
      ),
    );
  }

  @override
  $SchemasTable createAlias(String alias) {
    return $SchemasTable(attachedDatabase, alias);
  }

  static i0.JsonTypeConverter2<
    Map<String, dynamic>,
    String,
    Map<String, dynamic>
  >
  $converterdefinition = const i3.JsonConverter();
  static i0.JsonTypeConverter2<
    Map<String, dynamic>,
    String,
    Map<String, dynamic>
  >
  $converterdisplay = const i3.JsonConverter();
  static i0.JsonTypeConverter2<
    Map<String, dynamic>,
    String,
    Map<String, dynamic>
  >
  $converterindexed = const i3.JsonConverter();
}

class Schema extends i0.DataClass implements i0.Insertable<i1.Schema> {
  final String uuid;
  final bool deleted;
  final bool archived;
  final DateTime createdAt;
  final DateTime updatedAt;
  final String name;
  final int version;
  final bool systemDefined;
  final Map<String, dynamic> definition;
  final Map<String, dynamic> display;
  final Map<String, dynamic> indexed;
  const Schema({
    required this.uuid,
    required this.deleted,
    required this.archived,
    required this.createdAt,
    required this.updatedAt,
    required this.name,
    required this.version,
    required this.systemDefined,
    required this.definition,
    required this.display,
    required this.indexed,
  });
  @override
  Map<String, i0.Expression> toColumns(bool nullToAbsent) {
    final map = <String, i0.Expression>{};
    map['uuid'] = i0.Variable<String>(uuid);
    map['deleted'] = i0.Variable<bool>(deleted);
    map['archived'] = i0.Variable<bool>(archived);
    map['created_at'] = i0.Variable<DateTime>(createdAt);
    map['updated_at'] = i0.Variable<DateTime>(updatedAt);
    map['name'] = i0.Variable<String>(name);
    map['version'] = i0.Variable<int>(version);
    map['system_defined'] = i0.Variable<bool>(systemDefined);
    {
      map['definition'] = i0.Variable<String>(
        i1.$SchemasTable.$converterdefinition.toSql(definition),
      );
    }
    {
      map['display'] = i0.Variable<String>(
        i1.$SchemasTable.$converterdisplay.toSql(display),
      );
    }
    {
      map['indexed'] = i0.Variable<String>(
        i1.$SchemasTable.$converterindexed.toSql(indexed),
      );
    }
    return map;
  }

  i1.SchemasCompanion toCompanion(bool nullToAbsent) {
    return i1.SchemasCompanion(
      uuid: i0.Value(uuid),
      deleted: i0.Value(deleted),
      archived: i0.Value(archived),
      createdAt: i0.Value(createdAt),
      updatedAt: i0.Value(updatedAt),
      name: i0.Value(name),
      version: i0.Value(version),
      systemDefined: i0.Value(systemDefined),
      definition: i0.Value(definition),
      display: i0.Value(display),
      indexed: i0.Value(indexed),
    );
  }

  factory Schema.fromJson(
    Map<String, dynamic> json, {
    i0.ValueSerializer? serializer,
  }) {
    serializer ??= i0.driftRuntimeOptions.defaultSerializer;
    return Schema(
      uuid: serializer.fromJson<String>(json['uuid']),
      deleted: serializer.fromJson<bool>(json['deleted']),
      archived: serializer.fromJson<bool>(json['archived']),
      createdAt: serializer.fromJson<DateTime>(json['createdAt']),
      updatedAt: serializer.fromJson<DateTime>(json['updatedAt']),
      name: serializer.fromJson<String>(json['name']),
      version: serializer.fromJson<int>(json['version']),
      systemDefined: serializer.fromJson<bool>(json['systemDefined']),
      definition: i1.$SchemasTable.$converterdefinition.fromJson(
        serializer.fromJson<Map<String, dynamic>>(json['definition']),
      ),
      display: i1.$SchemasTable.$converterdisplay.fromJson(
        serializer.fromJson<Map<String, dynamic>>(json['display']),
      ),
      indexed: i1.$SchemasTable.$converterindexed.fromJson(
        serializer.fromJson<Map<String, dynamic>>(json['indexed']),
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
      'name': serializer.toJson<String>(name),
      'version': serializer.toJson<int>(version),
      'systemDefined': serializer.toJson<bool>(systemDefined),
      'definition': serializer.toJson<Map<String, dynamic>>(
        i1.$SchemasTable.$converterdefinition.toJson(definition),
      ),
      'display': serializer.toJson<Map<String, dynamic>>(
        i1.$SchemasTable.$converterdisplay.toJson(display),
      ),
      'indexed': serializer.toJson<Map<String, dynamic>>(
        i1.$SchemasTable.$converterindexed.toJson(indexed),
      ),
    };
  }

  i1.Schema copyWith({
    String? uuid,
    bool? deleted,
    bool? archived,
    DateTime? createdAt,
    DateTime? updatedAt,
    String? name,
    int? version,
    bool? systemDefined,
    Map<String, dynamic>? definition,
    Map<String, dynamic>? display,
    Map<String, dynamic>? indexed,
  }) => i1.Schema(
    uuid: uuid ?? this.uuid,
    deleted: deleted ?? this.deleted,
    archived: archived ?? this.archived,
    createdAt: createdAt ?? this.createdAt,
    updatedAt: updatedAt ?? this.updatedAt,
    name: name ?? this.name,
    version: version ?? this.version,
    systemDefined: systemDefined ?? this.systemDefined,
    definition: definition ?? this.definition,
    display: display ?? this.display,
    indexed: indexed ?? this.indexed,
  );
  Schema copyWithCompanion(i1.SchemasCompanion data) {
    return Schema(
      uuid: data.uuid.present ? data.uuid.value : this.uuid,
      deleted: data.deleted.present ? data.deleted.value : this.deleted,
      archived: data.archived.present ? data.archived.value : this.archived,
      createdAt: data.createdAt.present ? data.createdAt.value : this.createdAt,
      updatedAt: data.updatedAt.present ? data.updatedAt.value : this.updatedAt,
      name: data.name.present ? data.name.value : this.name,
      version: data.version.present ? data.version.value : this.version,
      systemDefined:
          data.systemDefined.present
              ? data.systemDefined.value
              : this.systemDefined,
      definition:
          data.definition.present ? data.definition.value : this.definition,
      display: data.display.present ? data.display.value : this.display,
      indexed: data.indexed.present ? data.indexed.value : this.indexed,
    );
  }

  @override
  String toString() {
    return (StringBuffer('Schema(')
          ..write('uuid: $uuid, ')
          ..write('deleted: $deleted, ')
          ..write('archived: $archived, ')
          ..write('createdAt: $createdAt, ')
          ..write('updatedAt: $updatedAt, ')
          ..write('name: $name, ')
          ..write('version: $version, ')
          ..write('systemDefined: $systemDefined, ')
          ..write('definition: $definition, ')
          ..write('display: $display, ')
          ..write('indexed: $indexed')
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
    name,
    version,
    systemDefined,
    definition,
    display,
    indexed,
  );
  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is i1.Schema &&
          other.uuid == this.uuid &&
          other.deleted == this.deleted &&
          other.archived == this.archived &&
          other.createdAt == this.createdAt &&
          other.updatedAt == this.updatedAt &&
          other.name == this.name &&
          other.version == this.version &&
          other.systemDefined == this.systemDefined &&
          other.definition == this.definition &&
          other.display == this.display &&
          other.indexed == this.indexed);
}

class SchemasCompanion extends i0.UpdateCompanion<i1.Schema> {
  final i0.Value<String> uuid;
  final i0.Value<bool> deleted;
  final i0.Value<bool> archived;
  final i0.Value<DateTime> createdAt;
  final i0.Value<DateTime> updatedAt;
  final i0.Value<String> name;
  final i0.Value<int> version;
  final i0.Value<bool> systemDefined;
  final i0.Value<Map<String, dynamic>> definition;
  final i0.Value<Map<String, dynamic>> display;
  final i0.Value<Map<String, dynamic>> indexed;
  final i0.Value<int> rowid;
  const SchemasCompanion({
    this.uuid = const i0.Value.absent(),
    this.deleted = const i0.Value.absent(),
    this.archived = const i0.Value.absent(),
    this.createdAt = const i0.Value.absent(),
    this.updatedAt = const i0.Value.absent(),
    this.name = const i0.Value.absent(),
    this.version = const i0.Value.absent(),
    this.systemDefined = const i0.Value.absent(),
    this.definition = const i0.Value.absent(),
    this.display = const i0.Value.absent(),
    this.indexed = const i0.Value.absent(),
    this.rowid = const i0.Value.absent(),
  });
  SchemasCompanion.insert({
    required String uuid,
    this.deleted = const i0.Value.absent(),
    this.archived = const i0.Value.absent(),
    this.createdAt = const i0.Value.absent(),
    this.updatedAt = const i0.Value.absent(),
    required String name,
    this.version = const i0.Value.absent(),
    this.systemDefined = const i0.Value.absent(),
    required Map<String, dynamic> definition,
    this.display = const i0.Value.absent(),
    this.indexed = const i0.Value.absent(),
    this.rowid = const i0.Value.absent(),
  }) : uuid = i0.Value(uuid),
       name = i0.Value(name),
       definition = i0.Value(definition);
  static i0.Insertable<i1.Schema> custom({
    i0.Expression<String>? uuid,
    i0.Expression<bool>? deleted,
    i0.Expression<bool>? archived,
    i0.Expression<DateTime>? createdAt,
    i0.Expression<DateTime>? updatedAt,
    i0.Expression<String>? name,
    i0.Expression<int>? version,
    i0.Expression<bool>? systemDefined,
    i0.Expression<String>? definition,
    i0.Expression<String>? display,
    i0.Expression<String>? indexed,
    i0.Expression<int>? rowid,
  }) {
    return i0.RawValuesInsertable({
      if (uuid != null) 'uuid': uuid,
      if (deleted != null) 'deleted': deleted,
      if (archived != null) 'archived': archived,
      if (createdAt != null) 'created_at': createdAt,
      if (updatedAt != null) 'updated_at': updatedAt,
      if (name != null) 'name': name,
      if (version != null) 'version': version,
      if (systemDefined != null) 'system_defined': systemDefined,
      if (definition != null) 'definition': definition,
      if (display != null) 'display': display,
      if (indexed != null) 'indexed': indexed,
      if (rowid != null) 'rowid': rowid,
    });
  }

  i1.SchemasCompanion copyWith({
    i0.Value<String>? uuid,
    i0.Value<bool>? deleted,
    i0.Value<bool>? archived,
    i0.Value<DateTime>? createdAt,
    i0.Value<DateTime>? updatedAt,
    i0.Value<String>? name,
    i0.Value<int>? version,
    i0.Value<bool>? systemDefined,
    i0.Value<Map<String, dynamic>>? definition,
    i0.Value<Map<String, dynamic>>? display,
    i0.Value<Map<String, dynamic>>? indexed,
    i0.Value<int>? rowid,
  }) {
    return i1.SchemasCompanion(
      uuid: uuid ?? this.uuid,
      deleted: deleted ?? this.deleted,
      archived: archived ?? this.archived,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
      name: name ?? this.name,
      version: version ?? this.version,
      systemDefined: systemDefined ?? this.systemDefined,
      definition: definition ?? this.definition,
      display: display ?? this.display,
      indexed: indexed ?? this.indexed,
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
    if (name.present) {
      map['name'] = i0.Variable<String>(name.value);
    }
    if (version.present) {
      map['version'] = i0.Variable<int>(version.value);
    }
    if (systemDefined.present) {
      map['system_defined'] = i0.Variable<bool>(systemDefined.value);
    }
    if (definition.present) {
      map['definition'] = i0.Variable<String>(
        i1.$SchemasTable.$converterdefinition.toSql(definition.value),
      );
    }
    if (display.present) {
      map['display'] = i0.Variable<String>(
        i1.$SchemasTable.$converterdisplay.toSql(display.value),
      );
    }
    if (indexed.present) {
      map['indexed'] = i0.Variable<String>(
        i1.$SchemasTable.$converterindexed.toSql(indexed.value),
      );
    }
    if (rowid.present) {
      map['rowid'] = i0.Variable<int>(rowid.value);
    }
    return map;
  }

  @override
  String toString() {
    return (StringBuffer('SchemasCompanion(')
          ..write('uuid: $uuid, ')
          ..write('deleted: $deleted, ')
          ..write('archived: $archived, ')
          ..write('createdAt: $createdAt, ')
          ..write('updatedAt: $updatedAt, ')
          ..write('name: $name, ')
          ..write('version: $version, ')
          ..write('systemDefined: $systemDefined, ')
          ..write('definition: $definition, ')
          ..write('display: $display, ')
          ..write('indexed: $indexed, ')
          ..write('rowid: $rowid')
          ..write(')'))
        .toString();
  }
}
