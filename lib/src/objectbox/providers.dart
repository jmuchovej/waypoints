import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'objectbox.dart';

/// Provider for the ObjectBox instance
/// This is created once at app startup and never disposed
final objectBoxProvider = Provider<ObjectBox>((ref) {
  throw UnimplementedError('ObjectBox must be initialized before use');
});

/// Override this provider in main.dart after creating ObjectBox
Provider<ObjectBox> createObjectBoxProvider(ObjectBox obx) {
  return Provider<ObjectBox>((ref) => obx);
}