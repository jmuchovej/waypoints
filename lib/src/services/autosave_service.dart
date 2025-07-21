import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../editor/editor.dart';
import '../editor/listeners/autosave_listener.dart';
import '../objectbox/services/repository_router.dart';
import '../things/foundation.dart';

final autosaveServiceProvider = Provider<AutosaveService>((ref) => AutosaveService(ref));

sealed class SaveEvent {}

class ThingChangedSaveEvent<BT extends ComparableThing> implements SaveEvent {
  ThingChangedSaveEvent(this.prevThing, this.currThing);

  factory ThingChangedSaveEvent.fromSuperEditor(ThingChangeEvent<BT> event) {
    return ThingChangedSaveEvent<BT>(event.prevThing, event.currThing);
  }

  final BT prevThing;
  final BT currThing;
}

/// Centralize service for persisting changes to [BaseThing] from both
/// [SuperEditor] and across Tempo.
class AutosaveService {
  AutosaveService(this._ref);

  final Ref _ref;

  RepositoryRouter get _router => _ref.read(repositoryRouterProvider);

  /// Save a single [BaseThing] change
  void saveChange(ThingChangedSaveEvent change) {
    _router.put(change.currThing);
  }

  /// Batch save [BaseThing]s as a transaction
  void saveChanges(List<ThingChangedSaveEvent> changes) {
    final things = changes.map((c) => c.currThing).toList();
    _router.putMany(things);
  }
}
