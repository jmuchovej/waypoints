import 'package:objectbox/objectbox.dart';

import '../../editor/super_editor.dart';
import '../../objectbox/attributed_text_mixin.dart';
import '../thing/interface.dart';
import '../thing/thing_entity.dart';

// Resolver is now auto-registered by ProjectRepository
// No need for manual resolver configuration!

@Entity()
class ProjectEntity with AttributedTextEntity, ThingBehavior implements Thing {
  ProjectEntity({
    required this.uuid,
    AttributedText? title,
    DateTime? createDate,
    DateTime? modifyDate,
    this.archived = false,
    this.deleted = false,
    ProjectStatus status = ProjectStatus.todo,
  }) : createDate = createDate ?? DateTime.now(),
       modifyDate = modifyDate ?? DateTime.now(),
       _status = status {
    this.title = title ?? AttributedText('');
  }

  static const key = 'project';

  @override
  @Id()
  int id = 0;

  @override
  String uuid;

  @override
  bool archived;
  @override
  bool deleted;

  String titleJson = '';

  @override
  @Property(type: PropertyType.dateNano)
  DateTime createDate;

  @override
  @Property(type: PropertyType.dateNano)
  DateTime modifyDate;

  /// Polymorphic children relationship - can contain any entity type
  /// When viewing a project, these are the document nodes shown in order
  @override
  final children = ToMany<ThingEntity>();

  /// Direct reference to the ThingEntity registration
  /// This creates a 1:1 relationship between this Project and its registry entry
  @override
  final ToOne<ThingEntity> thing = ToOne<ThingEntity>();

  final parent = ToOne<ProjectEntity>();

  @override
  int get hashCode => uuid.hashCode ^ title.hashCode;

  ProjectStatus _status;

  @Transient()
  ProjectStatus get status => _status;
  set statusOBX(int value) => _status = ProjectStatus.values[value];
  int get statusOBX {
    ProjectStatus.assertStableEnumValues();
    return _status.index;
  }

  @override
  String get type => key;
  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! ProjectEntity) return false;
    return id == other.id && uuid == other.uuid;
  }

  ProjectEntity copyWith({
    String? uuid,
    AttributedText? title,
    DateTime? createDate,
    DateTime? startDate,
    DateTime? endDate,
    DateTime? scheduledDate,
    DateTime? dueDate,
    DateTime? waitDate,
    DateTime? untilDate,
    String? recurrence,
    ProjectStatus? status,
    bool? deleted,
    bool? archived,
    ProjectEntity? parent,
  }) {
    final project = ProjectEntity(
      uuid: uuid ?? this.uuid,
      createDate: createDate ?? this.createDate,
      modifyDate: DateTime.now(),
      status: status ?? this.status,
      deleted: deleted ?? this.deleted,
      archived: archived ?? this.archived,
    );
    project.title = title ?? this.title;

    project.parent.target = parent ?? this.parent.target;

    return project;
  }

  @Transient()
  @override
  AttributedText get title => getAttributedText(titleJson);
  @override
  set title(AttributedText value) =>
      setAttributedText(value, (json) => titleJson = json);

  @override
  String getSearchableText() => title.toPlainText();
}

enum ProjectStatus {
  todo, // Initial state
  started, // We've started but not focused on it
  waiting, // Blocked by something
  paused, // Interrupted but will resume
  workInProgress, // Parent task with child being worked on
  stopped, // No longer working on it
  dropped, // Decided not to complete
  complete; // Finished! 🎉

  static void assertStableEnumValues() {
    assert(0 == ProjectStatus.todo.index);
    assert(1 == ProjectStatus.started.index);
    assert(2 == ProjectStatus.waiting.index);
    assert(3 == ProjectStatus.paused.index);
    assert(4 == ProjectStatus.workInProgress.index);
    assert(5 == ProjectStatus.stopped.index);
    assert(6 == ProjectStatus.dropped.index);
    assert(7 == ProjectStatus.complete.index);
  }
}
