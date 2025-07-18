import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class PillButton extends ConsumerWidget {
  const PillButton({
    required this.onPressed,
    required this.child,
    this.onLongPress,
    this.onHover,
    this.onFocusChange,
    this.style,
    this.focusNode,
    this.autofocus = false,
    this.clipBehavior = Clip.none,
    this.statesController,
    super.key,
  }) : _icon = null,
       _label = null,
       _isTonal = false;

  const PillButton.icon({
    required this.onPressed,
    required Widget icon,
    required Widget label,
    this.onLongPress,
    this.onHover,
    this.onFocusChange,
    this.style,
    this.focusNode,
    this.autofocus = false,
    this.clipBehavior = Clip.none,
    this.statesController,
    super.key,
  }) : _icon = icon,
       _label = label,
       child = null,
       _isTonal = false;

  const PillButton.tonal({
    required this.onPressed,
    required this.child,
    this.onLongPress,
    this.onHover,
    this.onFocusChange,
    this.style,
    this.focusNode,
    this.autofocus = false,
    this.clipBehavior = Clip.none,
    this.statesController,
    super.key,
  }) : _icon = null,
       _label = null,
       _isTonal = true;

  const PillButton.tonalIcon({
    required this.onPressed,
    required Widget icon,
    required Widget label,
    this.onLongPress,
    this.onHover,
    this.onFocusChange,
    this.style,
    this.focusNode,
    this.autofocus = false,
    this.clipBehavior = Clip.none,
    this.statesController,
    super.key,
  }) : _icon = icon,
       _label = label,
       child = null,
       _isTonal = true;

  final VoidCallback? onPressed;
  final VoidCallback? onLongPress;
  final ValueChanged<bool>? onHover;
  final ValueChanged<bool>? onFocusChange;
  final ButtonStyle? style;
  final FocusNode? focusNode;
  final bool autofocus;
  final Clip clipBehavior;
  final WidgetStatesController? statesController;
  final Widget? child;
  final Widget? _icon;
  final Widget? _label;
  final bool _isTonal;

  ButtonStyle _defaultStyle(BuildContext context) {
    final theme = Theme.of(context);
    return FilledButton.styleFrom(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final effectiveStyle = style ?? _defaultStyle(context);

    if (_icon != null && _label != null) {
      if (_isTonal) {
        return FilledButton.tonalIcon(
          onPressed: onPressed,
          onLongPress: onLongPress,
          onHover: onHover,
          onFocusChange: onFocusChange,
          style: effectiveStyle,
          focusNode: focusNode,
          autofocus: autofocus,
          clipBehavior: clipBehavior,
          statesController: statesController,
          icon: _icon!,
          label: _label!,
        );
      }
      return FilledButton.icon(
        onPressed: onPressed,
        onLongPress: onLongPress,
        onHover: onHover,
        onFocusChange: onFocusChange,
        style: effectiveStyle,
        focusNode: focusNode,
        autofocus: autofocus,
        clipBehavior: clipBehavior,
        statesController: statesController,
        icon: _icon!,
        label: _label!,
      );
    }

    if (_isTonal) {
      return FilledButton.tonal(
        onPressed: onPressed,
        onLongPress: onLongPress,
        onHover: onHover,
        onFocusChange: onFocusChange,
        style: effectiveStyle,
        focusNode: focusNode,
        autofocus: autofocus,
        clipBehavior: clipBehavior,
        statesController: statesController,
        child: child!,
      );
    }

    return FilledButton(
      onPressed: onPressed,
      onLongPress: onLongPress,
      onHover: onHover,
      onFocusChange: onFocusChange,
      style: effectiveStyle,
      focusNode: focusNode,
      autofocus: autofocus,
      clipBehavior: clipBehavior,
      statesController: statesController,
      child: child!,
    );
  }
}
