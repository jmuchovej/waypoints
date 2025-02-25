library;

import 'package:flutter/widgets.dart';
import 'package:flutter_svg/flutter_svg.dart';

/// Similarly to Material Icons, use [Iconify] Widget to display Iconify.
///
///
/// ```dart
/// import 'package:iconify_flutter/icons/IconsLibrary';
/// HeroIcon(IconsLibrary.arrowLeft)
/// ```

class Iconify extends StatelessWidget {
  final String icon;
  final Color? color;
  final double? size;

  const Iconify(
    this.icon, {
    super.key,
    this.color,
    this.size = 24,
  });

  @override
  Widget build(BuildContext context) {
    return SvgPicture.string(
      icon,
      colorFilter:
          color != null ? ColorFilter.mode(color!, BlendMode.srcIn) : null,
      width: size,
      height: size,
      alignment: Alignment.center,
    );
  }
}

class IconSet {
  const IconSet(this._icons);

  @protected
  final Map<String, String> _icons;

  String operator [](String name) => _icons[name]!;

  Iterable<String> get keys => _icons.keys;
}
