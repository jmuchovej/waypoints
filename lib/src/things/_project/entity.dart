
import 'package:objectbox/objectbox.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/things/_task/entity.dart';
import 'package:tempo/src/database/objectbox/mixins/attributed_text.dart';
import 'package:tempo/src/database/objectbox/mixins/uuid_entity.dart';

@Entity()
class ProjectEntity with AttributedTextEntity, UuidEntity {
  ProjectEntity({required this.uuid, AttributedText? title}) {
    this.title = title ?? AttributedText('');
  }

  @override
  @Id()
  int id = 0;

  @override
  String uuid;

  String titleJson = '';
  @Index()
  String titlePlainText = '';

  @Backlink('project')
  final tasks = ToMany<TaskEntity>();

  final parent = ToOne<ProjectEntity>();
  @Backlink()
  final children = ToMany<ProjectEntity>();

  @override
  String get entityType => 'project';

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! ProjectEntity) return false;
    return id == other.id && uuid == other.uuid;
  }

  @override
  int get hashCode => uuid.hashCode ^ title.hashCode;

  @Transient()
  AttributedText get title => getAttributedText(titleJson, titlePlainText);
  set title(AttributedText value) => setAttributedText(
    value,
    (json) => titleJson = json,
    (plain) => titlePlainText = plain,
  );
}
