import 'package:drift/drift.dart';
import 'package:tempo/src/database/sqlite/daos/project.dart';
import 'package:tempo/src/database/sqlite/daos/task.dart';
import 'package:tempo/src/database/sqlite/database.dart';
import 'package:tempo/src/database/sqlite/tables/schema.drift.dart';
import 'package:tempo/src/database/sqlite/tables/thing.drift.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:uuid/uuid.dart';

class ThingDAO {
  ThingDAO(this._thing, this._schema);

  final Thing _thing;
  final Schema _schema;

  String get uuid => _thing.uuid;
  AttributedText get title => _thing.title;
  DateTime get createdAt => _thing.createdAt;
  DateTime get updatedAt => _thing.updatedAt;
  bool get deleted => _thing.deleted;
  bool get archived => _thing.archived;

  String get schemaName => _schema.name;
  int get schemaVersion => _schema.version;
  Map<String, dynamic> get schema => _schema.definition;

  Map<String, dynamic> get properties => _thing.properties;
  Map<String, dynamic> get metadata => _thing.metadata;

  T? getProperty<T>(String key) {
    final value = properties[key];
    if (value == null) return null;

    if (T == DateTime && value is String) {
      return DateTime.parse(value) as T?;
    }

    if (value is T) return value;
    return null;
  }

  void setProperty<T>(String key, T? value) {
    if (value == null) {
      properties.remove(key);
      return;
    }

    properties[key] = switch (value) {
      DateTime _ => value.toIso8601String(),
      _ => value,
    };
  }

  String? getReference(String key) => getProperty<String>(key);
  List<String> getReferences(String key) {
    final list = getProperty<List>(key);
    return list?.cast<String>() ?? [];
  }

  void setReference(String key, String? uuid) => setProperty(key, uuid);
  void setReferences(String key, List<String> uuids) => setProperty(key, uuids);
}

class SchemaRepository {
  SchemaRepository(this.database);

  final TempoDatabase database;
  final Map<String, Schema> _schemaCache = {};

  Future<Schema> getSchema(String name) async {
    if (_schemaCache.containsKey(name)) {
      return _schemaCache[name]!;
    }

    final schemas =
        await database.managers.schemas
            .filter((f) => f.name.equals(name))
            .get();

    if (schemas.isEmpty) {
      throw StateError('Schema not found! $name');
    }

    final schema = schemas.first;
    _schemaCache[name] = schema;
    return schema;
  }

  Future<List<ThingDAO>> getThingsBySchema(String schemaName) async {
    final schema = await getSchema(schemaName);
    final things =
        await database.managers.things
            .filter(
              (f) =>
                  f.deleted.equals(false) & f.schema.uuid.equals(schema.uuid),
            )
            .get();

    return things.map((thing) => _createDAOWrapper(thing, schema)).toList();
  }

  Future<ThingDAO?> getThingByUUID(String uuid) async {
    final things =
        await database.managers.things
            .filter((f) => f.uuid.equals(uuid) & f.deleted.equals(false))
            .get();

    if (things.isEmpty) return null;

    final thing = things.first;
    final schema = await getSchemaByUUID(thing.schema);
    return _createDAOWrapper(thing, schema);
  }

  Future<Schema> getSchemaByUUID(String uuid) async {
    final schemas =
        await database.managers.schemas
            .filter((f) => f.uuid.equals(uuid))
            .get();

    if (schemas.isEmpty) {
      throw StateError('Schema not found with UUID=$uuid');
    }

    return schemas.first;
  }

  Future<void> save(ThingDAO thing) async {
    await database.managers.things
        .filter((f) => f.uuid.equals(thing.uuid))
        .update(
          (o) => o(
            title: Value(thing.title),
            properties: Value(thing.properties),
            metadata: Value(thing.metadata),
            updatedAt: Value(DateTime.now()),
          ),
        );
  }

  Future<ThingDAO> create(
    String name, {
    required AttributedText title,
    Map<String, dynamic>? properties,
    Map<String, dynamic>? metadata,
  }) async {
    final schema = await getSchema(name);
    final uuid = const Uuid().v4();

    await database.managers.things.create(
      (o) => o(
        uuid: uuid,
        title: title,
        schema: schema.uuid,
        properties: properties ?? {},
        metadata: Value(metadata ?? {}),
      ),
    );

    final thing = await getThingByUUID(uuid);
    return thing!;
  }

  Future<void> delete(String uuid) async {
    await database.managers.things
        .filter((t) => t.uuid.equals(uuid))
        .update(
          (o) => o(deleted: Value(true), updatedAt: Value(DateTime.now())),
        );
  }

  ThingDAO _createDAOWrapper(Thing thing, Schema schema) => switch (schema
      .name) {
    'task' => TaskDAO(thing, schema),
    'project' => ProjectDAO(thing, schema),
    _ => ThingDAO(thing, schema),
  };
}
