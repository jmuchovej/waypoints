import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';

import '../../things/foundation.dart';
import '../objectbox.g.dart';
import '../objectbox.dart';
import '../../things/thing.entity.dart';

/// Provider for ThingService
final thingServiceProvider = Provider<ThingService>((ref) {
  final obx = ref.watch(objectBoxProvider);
  return ThingService(obx);
});

/// Service that coordinates Thing operations
/// Now uses providers instead of static methods
class ThingService {
  ThingService(this._obx);

  final ObjectBox _obx;

  Box<ThingEntity> get _box => Box<ThingEntity>(_obx.store);
  Store get store => _obx.store;

  /// Look up a Thing registry by UUID
  ThingEntity? lookup(String uuid) {
    return _box.query(ThingEntity_.uuid.equals(uuid)).build().findFirst();
  }

  /// Generate a unique UUID that doesn't exist in the registry
  String generateUniqueUUID() {
    String uuid;
    do {
      uuid = Uuid().v4();
    } while (lookup(uuid) != null);
    return uuid;
  }

  ThingEntity? get(int id) => _box.get(id);

  /// Register a new Thing in the registry
  /// Ensures UUID uniqueness
  ThingEntity register(String uuid, String type, int id) {
    final existing = lookup(uuid);
    if (existing != null) {
      throw UUIDAlreadyExistsException(uuid, existing.type, type);
    }

    final entity = ThingEntity(uuid: uuid, type: type, id: id);
    _box.put(entity);
    return entity;
  }

  BT link<BT extends BaseThing>(BT entity) {
    final BaseThing(uuid: uuid, type: type, id: id) = entity;
    final existing = lookup(uuid);
    if (existing != null) {
      throw UUIDAlreadyExistsException(uuid, existing.type, type);
    }

    final thing = ThingEntity(uuid: uuid, type: type, id: id);
    entity.thingRef.target = thing;
    return entity;
  }

  BT unlink<BT extends BaseThing>(BT entity) {
    final BaseThing(uuid: uuid) = entity;
    final existing = lookup(uuid);
    if (existing != null) {
      _box.remove(existing.registryID);
    }
    entity.thingRef.target = null;
    return entity;
  }

  /// Unregister a Thing from the registry
  void unregister(String uuid) {
    final existing = lookup(uuid);
    if (existing != null) {
      _box.remove(existing.registryID);
    }
  }

  /// Update the entity ID for a UUID (when entity is re-saved)
  ThingEntity updateID(String uuid, int newID) {
    final existing = lookup(uuid);
    if (existing == null) {
      throw AttemptedToUpdateIDException(uuid);
    }

    existing.id = newID;
    _box.put(existing);
    return existing;
  }

  /// Clear all Thing registrations from the registry
  /// Used when clearing sample data
  int clearAll() {
    return _box.removeAll();
  }
}

class UUIDAlreadyExistsException implements Exception {
  UUIDAlreadyExistsException(
    this.uuid,
    this.existingEntityType,
    this.attemptedEntityType,
  );

  final String uuid;
  final String existingEntityType;
  final String attemptedEntityType;

  @override
  String toString() {
    return 'UUID $uuid already exists for $existingEntityType (attempted to use for $attemptedEntityType)';
  }
}

class AttemptedToUpdateIDException implements Exception {
  AttemptedToUpdateIDException(this.uuid);

  final String uuid;

  @override
  String toString() {
    return "UUID $uuid doesn't exist! Attempted to update `id` on non-existent Thing";
  }
}
