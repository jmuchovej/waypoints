import 'package:flutter/widgets.dart';

import 'super_editor.dart';

class InlineIconPlaceholder {
  const InlineIconPlaceholder(this.icon);
  final Icon icon;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is InlineIconPlaceholder &&
          runtimeType == other.runtimeType &&
          icon == other.icon;

  @override
  int get hashCode => icon.hashCode;
}

Widget? inlineIconBuilder(
  BuildContext context,
  TextStyle textStyle,
  Object placeholder,
) {
  if (placeholder is! InlineIconPlaceholder) return null;

  final icon = Icon(
    placeholder.icon.icon,
    size: placeholder.icon.size ?? textStyle.fontSize,
    color: placeholder.icon.color ?? textStyle.color,
    applyTextScaling: true,
  );

  return LineHeightSquare(style: textStyle, child: icon);
}
