import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../../flavors.dart';
import 'objectbox.g.dart';

/// Provider for the ObjectBox instance
/// This is created once at app startup and never disposed
final objectBoxProvider = Provider<ObjectBox>((ref) {
  throw UnimplementedError('ObjectBox must be initialized before use');
});

/// Provides access to the ObjectBox Store throughout the app.
class ObjectBox {
  ObjectBox._create(this._store, Flavor flavor) {
    if (flavor.isDevelopment && Admin.isAvailable()) {
      _admin = Admin(_store);
    }
  }

  /// The Store of this app.
  late final Store _store;

  /// Expose store for testing purposes only.
  Store get store => _store;

  // ignore: unused_field
  late final Admin _admin;

  /// Create an instance of ObjectBox to use throughout the app.
  static Future<ObjectBox> create(Flavor flavor) async {
    final directory = await flavor.dataDirectory();
    // Future<Store> openStore() {...} is defined in the generated objectbox.g.dart
    final store = await openStore(
      directory: directory,
      macosApplicationGroup: "group.do.tempo",
    );
    return ObjectBox._create(store, flavor);
  }

  /// Create an ObjectBox instance for testing with a custom store.
  static ObjectBox createForTesting(Store store, Flavor flavor) {
    return ObjectBox._create(store, flavor);
  }
}

typedef TaskOBX = TaskEntity_;
typedef ProjectOBX = ProjectEntity_;
typedef ActivityOBX = ActivityEntity_;
typedef ThingOBX = ThingEntity_;
