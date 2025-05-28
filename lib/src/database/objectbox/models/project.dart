import 'package:objectbox/objectbox.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/database/objectbox/converters/attributed_text.dart';

@Entity()
class ProjectEntity with AttributedTextEntity {
  @Id()
  int id = 0;

  String uuid;

  String titleJson = '';
  @Index()
  String titlePlainText = '';

  @Backlink('project')
  final tasks = ToMany<TaskEntity>();

  final parent = ToOne<ProjectEntity>();
  @Backlink()
  final children = ToMany<ProjectEntity>();

  ProjectEntity({required this.uuid, AttributedText? title}) {
    this.title = title ?? AttributedText('');
  }

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
