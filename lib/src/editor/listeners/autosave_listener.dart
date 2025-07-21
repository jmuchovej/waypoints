import 'package:equatable/equatable.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../infrastructure/logging.dart';
import '../../objectbox/services/repository_router.dart';
import '../../objectbox/services/thing_registry.dart';
import '../../services/autosave_service.dart';
import '../../things/foundation.dart';
import '../editor.dart';
import '../../objectbox/repositories/thing_repository.dart';

/// Tracks changes to [Thing] entities in a [Document].
///
/// Contains both the old [prevThing] and new [currThing] to support
/// constructing diffs on [Thing]s _and_ undo/redo.
class ThingChangeEvent<BT extends ComparableThing> extends NodeDocumentChange
    with EquatableMixin {
  const ThingChangeEvent(this.prevThing, this.currThing);

  @override
  String get nodeId => prevThing.uuid;

  //// The state of the [Thing] **before** the change.
  final BT prevThing;
  //// The state of the [Thing] **after** the change.
  final BT currThing;

  @override
  String describe() => "Change to $nodeId: ${currThing.diff(prevThing)}";

  @override
  String toString() => "ThingChangeEvent (${nodeId})";

  @override
  List<Object?> get props => [nodeId, ...prevThing.props, ...currThing.props];
}

/// A service that automatically saves editor changes to Objectbox.
///
/// This service listens to [EditEvent]s from [SuperEditor] and persists any
/// [ThingChangeEvent]s to the appropriate repository via [RepositoryRouter].
///
/// This service is automatically added to an [Editor] in Tempo created with
/// [createSuperEditor].
///
/// TODO: ensure that updates to [Thing.children] and [Thing.parent] are persisted
class AutoSaveListener implements EditListener {
  /// Creates an [AutoSaveListener] with a [WidgetRef] so we can read the
  /// [repositoryRouterProvider] and [thingRegistryProvider] from Riverpod.
  const AutoSaveListener(this._ref);

  final WidgetRef _ref;

  /// Filters incoming `changeList` for [ThingChangeEvent]s to persist them
  /// using the [RepositoryRouter] for type-driven [ThingRepository] routing.
  @override
  void onEdit(List<EditEvent> changeList) {
    tempoEditorLog.info('Received ${changeList.length} changes');
    final thingChangeEvents =
        changeList
            .whereType<DocumentEdit>()
            .where((edit) => edit.change is ThingChangeEvent)
            .map((edit) => edit.change as ThingChangeEvent)
            .toList();

    tempoEditorLog.info(
      'Found ${thingChangeEvents.length} `ThingChangeEvent` changes',
    );
    if (thingChangeEvents.isEmpty) return;

    final saveService = _ref.read(autosaveServiceProvider);

    for (final change in thingChangeEvents) {
      saveService.saveChange(ThingChangedSaveEvent.fromSuperEditor(change));
      tempoEditorLog.finer('Saving change:: ${change.describe()}');
    }
  }
}
