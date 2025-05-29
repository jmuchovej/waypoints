import 'package:objectbox/objectbox.dart';
import 'package:tempo/src/database/objectbox/models/uuid_registry.dart';
import 'package:tempo/src/database/objectbox.g.dart';

class UuidRepository {
  final Box<UuidRegistry> _box;

  UuidRepository(this._box);

  /// Registers a UUID for an entity type. Throws if UUID already exists.
  void registerUuid(String uuid, String entityType, int entityId) {
    // Check if UUID already exists
    final existing = _box.query(UuidRegistry_.uuid.equals(uuid)).build().findFirst();
    if (existing != null) {
      throw UuidAlreadyExistsException(uuid, existing.entityType, entityType);
    }

    // Register the UUID
    final registry = UuidRegistry(
      uuid: uuid,
      entityType: entityType,
      entityId: entityId,
    );
    _box.put(registry);
  }

  /// Updates the entity ID for an existing UUID (for updates)
  void updateEntityId(String uuid, int newEntityId) {
    final existing = _box.query(UuidRegistry_.uuid.equals(uuid)).build().findFirst();
    if (existing != null) {
      existing.entityId = newEntityId;
      _box.put(existing);
    }
  }

  /// Removes a UUID from the registry
  void unregisterUuid(String uuid) {
    final existing = _box.query(UuidRegistry_.uuid.equals(uuid)).build().findFirst();
    if (existing != null) {
      _box.remove(existing.id);
    }
  }

  /// Looks up entity info by UUID
  UuidRegistry? lookupUuid(String uuid) {
    return _box.query(UuidRegistry_.uuid.equals(uuid)).build().findFirst();
  }

  /// Generates a new UUID that's guaranteed to be unique
  String generateUniqueUuid() {
    String uuid;
    do {
      uuid = _generateUuid();
    } while (lookupUuid(uuid) != null);
    return uuid;
  }

  static String _generateUuid() {
    // Use your preferred UUID generation method
    return DateTime.now().millisecondsSinceEpoch.toString() + 
           (1000 + (999 * (DateTime.now().microsecond / 1000000))).round().toString();
  }
}

// Singleton service for backwards compatibility
class UuidService {
  static late UuidRepository _repository;

  static void initialize(Box<UuidRegistry> box) {
    _repository = UuidRepository(box);
  }

  static void registerUuid(String uuid, String entityType, int entityId) {
    _repository.registerUuid(uuid, entityType, entityId);
  }

  static void updateEntityId(String uuid, int newEntityId) {
    _repository.updateEntityId(uuid, newEntityId);
  }

  static void unregisterUuid(String uuid) {
    _repository.unregisterUuid(uuid);
  }

  static UuidRegistry? lookupUuid(String uuid) {
    return _repository.lookupUuid(uuid);
  }

  static String generateUniqueUuid() {
    return _repository.generateUniqueUuid();
  }
}

class UuidAlreadyExistsException implements Exception {
  final String uuid;
  final String existingEntityType;
  final String attemptedEntityType;

  UuidAlreadyExistsException(this.uuid, this.existingEntityType, this.attemptedEntityType);

  @override
  String toString() {
    return 'UUID $uuid already exists for $existingEntityType (attempted to use for $attemptedEntityType)';
  }
}