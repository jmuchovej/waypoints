import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../infrastructure/logging.dart';

class FilterOption {
  const FilterOption({
    required this.title,
    required this.icon,
    required this.onPressed,
  });

  final String title;
  final IconData icon;
  final void Function() onPressed;
}

class FilterButton extends ConsumerStatefulWidget {
  const FilterButton({
    super.key,
    required this.icon,
    required this.label,
    required this.options,
  });

  final IconData icon;
  final String label;
  final List<FilterOption> options;

  @override
  ConsumerState<ConsumerStatefulWidget> createState() => _FilterButtonState();
}

class _FilterButtonState extends ConsumerState<FilterButton> {
  FilterOption? _currentlySelected;

  @override
  Widget build(BuildContext context) {
    final textStyle = TextTheme.of(context).bodySmall!;
    final menuButtons = widget.options
        .map(
          (FilterOption option) => MenuItemButton(
            semanticsLabel: option.title,
            onPressed: () {
              option.onPressed();
              setState(() {
                _currentlySelected = option;
              });
            },
            leadingIcon: Icon(
              option.icon,
              size: textStyle.fontSize! * 1.4,
              opticalSize: textStyle.fontSize,
            ),
            child: Text(option.title, style: textStyle),
          ),
        )
        .toList();

    final textSize = TextTheme.of(context).bodyMedium!;

    return MenuAnchor(
      builder: _menuBuilder,
      menuChildren: menuButtons,
      onOpen: () {
        waypointsLog.finest('opening `FilterButton`');
      },
      onClose: () {
        waypointsLog.finest('closing `FilterButton`');
      },
      alignmentOffset: Offset(24, textSize.fontSize! * 0.5),
    );
  }

  Widget _menuBuilder(
    BuildContext context,
    MenuController controller,
    Widget? child,
  ) {
    final icon = _currentlySelected?.icon ?? widget.icon;
    final label = _currentlySelected?.title ?? widget.label;
    final scheme = ColorScheme.of(context);

    return FilledButton.tonalIcon(
      onPressed: () {
        if (controller.isOpen) {
          controller.close();
        } else {
          controller.open();
        }
      },
      icon: Icon(icon),
      label: Text(label),
      style: ButtonStyle(
        backgroundColor: WidgetStateProperty.fromMap({
          WidgetState.hovered: scheme.surfaceContainerHighest,
          WidgetState.any: scheme.surfaceContainerHigh,
        }),
        enableFeedback: true,
        shadowColor: WidgetStateProperty.fromMap({
          WidgetState.hovered: null,
          WidgetState.any: null,
        }),
        //   foregroundColor: ColorScheme.of(context).primaryContainer,
        iconColor: WidgetStateProperty.all(
          _currentlySelected != null ? scheme.primary : scheme.onSurface,
        ),
      ),
    );
  }
}
