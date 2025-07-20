import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../infrastructure/logging.dart';
import '../../objectbox/objectbox.g.dart';
import '../repository.dart';
import 'block.entity.dart';

/// Repository for managing BlockEntity persistence and auto-save
class BlockRepository extends ThingRepository<BlockEntity> {
  BlockRepository(super._ref);

  @override
  Box<BlockEntity> get box => Box<BlockEntity>(store);

  @override
  String get type => BlockEntity.key;

  /// Save a block and create history snapshot
  // int saveWithHistory(BlockEntity block) {
  //   // Create snapshot before saving changes
  //   if (block.id > 0) {
  //     final snapshot = block.snapshot();
  //     snapshot.block.target = block;
  //     block.history.add(snapshot);
  //   }

  //   block.modifyDate = DateTime.now();
  //   return put(block);
  // }

  /// Bulk save blocks with history
  // List<int> saveAllWithHistory(List<BlockEntity> blocks) {
  //   // Create snapshots for existing blocks
  //   for (final block in blocks.where((b) => b.id > 0)) {
  //     final snapshot = block.snapshot();
  //     snapshot.block.target = block;
  //     block.history.add(snapshot);
  //     block.modifyDate = DateTime.now();
  //   }

  //   return putMany(blocks);
  // }

  /// Reorder blocks within a parent or document
  // void reorder(List<BlockEntity> blocks) {
  //   for (int i = 0; i < blocks.length; i++) {
  //     blocks[i] = blocks[i].copyWith(orderIndex: i);
  //   }
  //   saveAllWithHistory(blocks);
  // }

  /// Get history for a block
  List<BlockSnapshotEntity> historyFor(String blockUuid) {
    final block = getByUUID(blockUuid);
    if (block == null) return [];

    return block.history.toList()
      ..sort((a, b) => b.timestamp.compareTo(a.timestamp));
  }

  /// Restore block from history snapshot
  BlockEntity? restoreFromSnapshot(String blockUuid, int snapshotId) {
    final block = getByUUID(blockUuid);
    if (block == null) return null;

    final snapshot = block.history.firstWhere(
      (s) => s.id == snapshotId,
      orElse: () => throw ArgumentError('Snapshot not found'),
    );

    final restored = block.copyWith(
      nodeType: snapshot.type,
      content: snapshot.content,
      metadataJson: snapshot.metadataJson,
    );

    return restored;
  }
}

/// Provider for BlockRepository
final blockRepositoryProvider = Provider<BlockRepository>((ref) {
  return BlockRepository(ref);
});
