import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/things/_task/node.dart';

export 'package:tempo/src/things/_task/entity.dart';
export 'package:tempo/src/things/_task/node.dart';
export 'package:tempo/src/things/_task/components/builder.dart';
export 'package:tempo/src/things/_task/components/component.dart';
export 'package:tempo/src/things/_task/components/chips/status.dart';

const NamedAttribution taskAttribution = NamedAttribution("task");
const BlockSelector taskSelector = BlockSelector("task");
const NamedAttribution taskTitleAttribution = NamedAttribution("taskTitle");
const NamedAttribution taskDescriptionAttribution = NamedAttribution(
  "taskDescription",
);

/// Styles all task components to apply top padding
final taskStyles = StyleRule(taskSelector, (document, node) {
  if (node is! TaskNode) {
    return {};
  }

  // return {Styles.padding: const CascadingPadding.only(top: 6)};
  return {};
});

