import 'package:tempo/src/database/repositories/uuid_repository.dart';

/// Mixin for entities that need UUID registration
mixin UuidEntity {
  String get uuid;
  int get id;
  String get entityType;

  /// Called after the entity is saved to the database
  void registerUuid() {
    try {
      UuidService.registerUuid(uuid, entityType, id);
    } catch (e) {
      // UUID already exists - update the entity ID
      UuidService.updateEntityId(uuid, id);
    }
  }

  /// Called before the entity is deleted from the database
  void unregisterUuid() {
    UuidService.unregisterUuid(uuid);
  }

  /// Generates a unique UUID for this entity type
  static String generateUniqueUuid() {
    return UuidService.generateUniqueUuid();
  }
}