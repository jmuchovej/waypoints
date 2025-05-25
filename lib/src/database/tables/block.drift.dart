// dart format width=80
// ignore_for_file: type=lint
import 'package:drift/drift.dart' as i0;
import 'package:tempo/src/database/tables/block.drift.dart' as i1;
import 'package:tempo/src/database/tables/block.dart' as i2;
import 'package:tempo/src/database/converters.dart' as i3;

typedef $$BlocksTableCreateCompanionBuilder =
    i1.BlocksCompanion Function({
      required String uuid,
      i0.Value<bool> deleted,
      i0.Value<bool> archived,
      i0.Value<DateTime> createdAt,
      i0.Value<DateTime> updatedAt,
      required String type,
      required Map<String, dynamic> properties,
      i0.Value<Map<String, dynamic>> metadata,
      i0.Value<int> rowid,
    });
typedef $$BlocksTableUpdateCompanionBuilder =
    i1.BlocksCompanion Function({
      i0.Value<String> uuid,
      i0.Value<bool> deleted,
      i0.Value<bool> archived,
      i0.Value<DateTime> createdAt,
      i0.Value<DateTime> updatedAt,
      i0.Value<String> type,
      i0.Value<Map<String, dynamic>> properties,
      i0.Value<Map<String, dynamic>> metadata,
      i0.Value<int> rowid,
    });

class $$BlocksTableFilterComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$BlocksTable> {
  $$BlocksTableFilterComposer({
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

  i0.ColumnFilters<String> get type => $composableBuilder(
    column: $table.type,
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
}

class $$BlocksTableOrderingComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$BlocksTable> {
  $$BlocksTableOrderingComposer({
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

  i0.ColumnOrderings<String> get type => $composableBuilder(
    column: $table.type,
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
}

class $$BlocksTableAnnotationComposer
    extends i0.Composer<i0.GeneratedDatabase, i1.$BlocksTable> {
  $$BlocksTableAnnotationComposer({
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

  i0.GeneratedColumn<String> get type =>
      $composableBuilder(column: $table.type, builder: (column) => column);

  i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  get properties => $composableBuilder(
    column: $table.properties,
    builder: (column) => column,
  );

  i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  get metadata =>
      $composableBuilder(column: $table.metadata, builder: (column) => column);
}

class $$BlocksTableTableManager
    extends
        i0.RootTableManager<
          i0.GeneratedDatabase,
          i1.$BlocksTable,
          i1.Block,
          i1.$$BlocksTableFilterComposer,
          i1.$$BlocksTableOrderingComposer,
          i1.$$BlocksTableAnnotationComposer,
          $$BlocksTableCreateCompanionBuilder,
          $$BlocksTableUpdateCompanionBuilder,
          (
            i1.Block,
            i0.BaseReferences<i0.GeneratedDatabase, i1.$BlocksTable, i1.Block>,
          ),
          i1.Block,
          i0.PrefetchHooks Function()
        > {
  $$BlocksTableTableManager(i0.GeneratedDatabase db, i1.$BlocksTable table)
    : super(
        i0.TableManagerState(
          db: db,
          table: table,
          createFilteringComposer:
              () => i1.$$BlocksTableFilterComposer($db: db, $table: table),
          createOrderingComposer:
              () => i1.$$BlocksTableOrderingComposer($db: db, $table: table),
          createComputedFieldComposer:
              () => i1.$$BlocksTableAnnotationComposer($db: db, $table: table),
          updateCompanionCallback:
              ({
                i0.Value<String> uuid = const i0.Value.absent(),
                i0.Value<bool> deleted = const i0.Value.absent(),
                i0.Value<bool> archived = const i0.Value.absent(),
                i0.Value<DateTime> createdAt = const i0.Value.absent(),
                i0.Value<DateTime> updatedAt = const i0.Value.absent(),
                i0.Value<String> type = const i0.Value.absent(),
                i0.Value<Map<String, dynamic>> properties =
                    const i0.Value.absent(),
                i0.Value<Map<String, dynamic>> metadata =
                    const i0.Value.absent(),
                i0.Value<int> rowid = const i0.Value.absent(),
              }) => i1.BlocksCompanion(
                uuid: uuid,
                deleted: deleted,
                archived: archived,
                createdAt: createdAt,
                updatedAt: updatedAt,
                type: type,
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
                required String type,
                required Map<String, dynamic> properties,
                i0.Value<Map<String, dynamic>> metadata =
                    const i0.Value.absent(),
                i0.Value<int> rowid = const i0.Value.absent(),
              }) => i1.BlocksCompanion.insert(
                uuid: uuid,
                deleted: deleted,
                archived: archived,
                createdAt: createdAt,
                updatedAt: updatedAt,
                type: type,
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
                          i0.BaseReferences(db, table, e),
                        ),
                      )
                      .toList(),
          prefetchHooksCallback: null,
        ),
      );
}

typedef $$BlocksTableProcessedTableManager =
    i0.ProcessedTableManager<
      i0.GeneratedDatabase,
      i1.$BlocksTable,
      i1.Block,
      i1.$$BlocksTableFilterComposer,
      i1.$$BlocksTableOrderingComposer,
      i1.$$BlocksTableAnnotationComposer,
      $$BlocksTableCreateCompanionBuilder,
      $$BlocksTableUpdateCompanionBuilder,
      (
        i1.Block,
        i0.BaseReferences<i0.GeneratedDatabase, i1.$BlocksTable, i1.Block>,
      ),
      i1.Block,
      i0.PrefetchHooks Function()
    >;

class $BlocksTable extends i2.Blocks with i0.TableInfo<$BlocksTable, i1.Block> {
  @override
  final i0.GeneratedDatabase attachedDatabase;
  final String? _alias;
  $BlocksTable(this.attachedDatabase, [this._alias]);
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
  @override
  late final i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  properties = i0.GeneratedColumn<String>(
    'properties',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: true,
  ).withConverter<Map<String, dynamic>>(i1.$BlocksTable.$converterproperties);
  @override
  late final i0.GeneratedColumnWithTypeConverter<Map<String, dynamic>, String>
  metadata = i0.GeneratedColumn<String>(
    'metadata',
    aliasedName,
    false,
    type: i0.DriftSqlType.string,
    requiredDuringInsert: false,
    clientDefault: () => '{}',
  ).withConverter<Map<String, dynamic>>(i1.$BlocksTable.$convertermetadata);
  @override
  List<i0.GeneratedColumn> get $columns => [
    uuid,
    deleted,
    archived,
    createdAt,
    updatedAt,
    type,
    properties,
    metadata,
  ];
  @override
  String get aliasedName => _alias ?? actualTableName;
  @override
  String get actualTableName => $name;
  static const String $name = 'blocks';
  @override
  i0.VerificationContext validateIntegrity(
    i0.Insertable<i1.Block> instance, {
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
    if (data.containsKey('type')) {
      context.handle(
        _typeMeta,
        type.isAcceptableOrUnknown(data['type']!, _typeMeta),
      );
    } else if (isInserting) {
      context.missing(_typeMeta);
    }
    return context;
  }

  @override
  Set<i0.GeneratedColumn> get $primaryKey => {uuid};
  @override
  i1.Block map(Map<String, dynamic> data, {String? tablePrefix}) {
    final effectivePrefix = tablePrefix != null ? '$tablePrefix.' : '';
    return i1.Block(
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
      type:
          attachedDatabase.typeMapping.read(
            i0.DriftSqlType.string,
            data['${effectivePrefix}type'],
          )!,
      properties: i1.$BlocksTable.$converterproperties.fromSql(
        attachedDatabase.typeMapping.read(
          i0.DriftSqlType.string,
          data['${effectivePrefix}properties'],
        )!,
      ),
      metadata: i1.$BlocksTable.$convertermetadata.fromSql(
        attachedDatabase.typeMapping.read(
          i0.DriftSqlType.string,
          data['${effectivePrefix}metadata'],
        )!,
      ),
    );
  }

  @override
  $BlocksTable createAlias(String alias) {
    return $BlocksTable(attachedDatabase, alias);
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

class Block extends i0.DataClass implements i0.Insertable<i1.Block> {
  final String uuid;
  final bool deleted;
  final bool archived;
  final DateTime createdAt;
  final DateTime updatedAt;
  final String type;
  final Map<String, dynamic> properties;
  final Map<String, dynamic> metadata;
  const Block({
    required this.uuid,
    required this.deleted,
    required this.archived,
    required this.createdAt,
    required this.updatedAt,
    required this.type,
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
    map['type'] = i0.Variable<String>(type);
    {
      map['properties'] = i0.Variable<String>(
        i1.$BlocksTable.$converterproperties.toSql(properties),
      );
    }
    {
      map['metadata'] = i0.Variable<String>(
        i1.$BlocksTable.$convertermetadata.toSql(metadata),
      );
    }
    return map;
  }

  i1.BlocksCompanion toCompanion(bool nullToAbsent) {
    return i1.BlocksCompanion(
      uuid: i0.Value(uuid),
      deleted: i0.Value(deleted),
      archived: i0.Value(archived),
      createdAt: i0.Value(createdAt),
      updatedAt: i0.Value(updatedAt),
      type: i0.Value(type),
      properties: i0.Value(properties),
      metadata: i0.Value(metadata),
    );
  }

  factory Block.fromJson(
    Map<String, dynamic> json, {
    i0.ValueSerializer? serializer,
  }) {
    serializer ??= i0.driftRuntimeOptions.defaultSerializer;
    return Block(
      uuid: serializer.fromJson<String>(json['uuid']),
      deleted: serializer.fromJson<bool>(json['deleted']),
      archived: serializer.fromJson<bool>(json['archived']),
      createdAt: serializer.fromJson<DateTime>(json['createdAt']),
      updatedAt: serializer.fromJson<DateTime>(json['updatedAt']),
      type: serializer.fromJson<String>(json['type']),
      properties: i1.$BlocksTable.$converterproperties.fromJson(
        serializer.fromJson<Map<String, dynamic>>(json['properties']),
      ),
      metadata: i1.$BlocksTable.$convertermetadata.fromJson(
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
      'type': serializer.toJson<String>(type),
      'properties': serializer.toJson<Map<String, dynamic>>(
        i1.$BlocksTable.$converterproperties.toJson(properties),
      ),
      'metadata': serializer.toJson<Map<String, dynamic>>(
        i1.$BlocksTable.$convertermetadata.toJson(metadata),
      ),
    };
  }

  i1.Block copyWith({
    String? uuid,
    bool? deleted,
    bool? archived,
    DateTime? createdAt,
    DateTime? updatedAt,
    String? type,
    Map<String, dynamic>? properties,
    Map<String, dynamic>? metadata,
  }) => i1.Block(
    uuid: uuid ?? this.uuid,
    deleted: deleted ?? this.deleted,
    archived: archived ?? this.archived,
    createdAt: createdAt ?? this.createdAt,
    updatedAt: updatedAt ?? this.updatedAt,
    type: type ?? this.type,
    properties: properties ?? this.properties,
    metadata: metadata ?? this.metadata,
  );
  Block copyWithCompanion(i1.BlocksCompanion data) {
    return Block(
      uuid: data.uuid.present ? data.uuid.value : this.uuid,
      deleted: data.deleted.present ? data.deleted.value : this.deleted,
      archived: data.archived.present ? data.archived.value : this.archived,
      createdAt: data.createdAt.present ? data.createdAt.value : this.createdAt,
      updatedAt: data.updatedAt.present ? data.updatedAt.value : this.updatedAt,
      type: data.type.present ? data.type.value : this.type,
      properties:
          data.properties.present ? data.properties.value : this.properties,
      metadata: data.metadata.present ? data.metadata.value : this.metadata,
    );
  }

  @override
  String toString() {
    return (StringBuffer('Block(')
          ..write('uuid: $uuid, ')
          ..write('deleted: $deleted, ')
          ..write('archived: $archived, ')
          ..write('createdAt: $createdAt, ')
          ..write('updatedAt: $updatedAt, ')
          ..write('type: $type, ')
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
    type,
    properties,
    metadata,
  );
  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is i1.Block &&
          other.uuid == this.uuid &&
          other.deleted == this.deleted &&
          other.archived == this.archived &&
          other.createdAt == this.createdAt &&
          other.updatedAt == this.updatedAt &&
          other.type == this.type &&
          other.properties == this.properties &&
          other.metadata == this.metadata);
}

class BlocksCompanion extends i0.UpdateCompanion<i1.Block> {
  final i0.Value<String> uuid;
  final i0.Value<bool> deleted;
  final i0.Value<bool> archived;
  final i0.Value<DateTime> createdAt;
  final i0.Value<DateTime> updatedAt;
  final i0.Value<String> type;
  final i0.Value<Map<String, dynamic>> properties;
  final i0.Value<Map<String, dynamic>> metadata;
  final i0.Value<int> rowid;
  const BlocksCompanion({
    this.uuid = const i0.Value.absent(),
    this.deleted = const i0.Value.absent(),
    this.archived = const i0.Value.absent(),
    this.createdAt = const i0.Value.absent(),
    this.updatedAt = const i0.Value.absent(),
    this.type = const i0.Value.absent(),
    this.properties = const i0.Value.absent(),
    this.metadata = const i0.Value.absent(),
    this.rowid = const i0.Value.absent(),
  });
  BlocksCompanion.insert({
    required String uuid,
    this.deleted = const i0.Value.absent(),
    this.archived = const i0.Value.absent(),
    this.createdAt = const i0.Value.absent(),
    this.updatedAt = const i0.Value.absent(),
    required String type,
    required Map<String, dynamic> properties,
    this.metadata = const i0.Value.absent(),
    this.rowid = const i0.Value.absent(),
  }) : uuid = i0.Value(uuid),
       type = i0.Value(type),
       properties = i0.Value(properties);
  static i0.Insertable<i1.Block> custom({
    i0.Expression<String>? uuid,
    i0.Expression<bool>? deleted,
    i0.Expression<bool>? archived,
    i0.Expression<DateTime>? createdAt,
    i0.Expression<DateTime>? updatedAt,
    i0.Expression<String>? type,
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
      if (type != null) 'type': type,
      if (properties != null) 'properties': properties,
      if (metadata != null) 'metadata': metadata,
      if (rowid != null) 'rowid': rowid,
    });
  }

  i1.BlocksCompanion copyWith({
    i0.Value<String>? uuid,
    i0.Value<bool>? deleted,
    i0.Value<bool>? archived,
    i0.Value<DateTime>? createdAt,
    i0.Value<DateTime>? updatedAt,
    i0.Value<String>? type,
    i0.Value<Map<String, dynamic>>? properties,
    i0.Value<Map<String, dynamic>>? metadata,
    i0.Value<int>? rowid,
  }) {
    return i1.BlocksCompanion(
      uuid: uuid ?? this.uuid,
      deleted: deleted ?? this.deleted,
      archived: archived ?? this.archived,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
      type: type ?? this.type,
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
    if (type.present) {
      map['type'] = i0.Variable<String>(type.value);
    }
    if (properties.present) {
      map['properties'] = i0.Variable<String>(
        i1.$BlocksTable.$converterproperties.toSql(properties.value),
      );
    }
    if (metadata.present) {
      map['metadata'] = i0.Variable<String>(
        i1.$BlocksTable.$convertermetadata.toSql(metadata.value),
      );
    }
    if (rowid.present) {
      map['rowid'] = i0.Variable<int>(rowid.value);
    }
    return map;
  }

  @override
  String toString() {
    return (StringBuffer('BlocksCompanion(')
          ..write('uuid: $uuid, ')
          ..write('deleted: $deleted, ')
          ..write('archived: $archived, ')
          ..write('createdAt: $createdAt, ')
          ..write('updatedAt: $updatedAt, ')
          ..write('type: $type, ')
          ..write('properties: $properties, ')
          ..write('metadata: $metadata, ')
          ..write('rowid: $rowid')
          ..write(')'))
        .toString();
  }
}
