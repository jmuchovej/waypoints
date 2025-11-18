import 'package:flutter/material.dart';
import 'package:widgetbook/widgetbook.dart';
import 'package:widgetbook_annotation/widgetbook_annotation.dart';

class BrightnessAddon extends WidgetbookAddon<Brightness> {
  final Brightness initialBrightness;

  BrightnessAddon({this.initialBrightness = Brightness.light})
    : super(name: 'Brightness');

  @override
  Widget buildUseCase(BuildContext context, Widget child, Brightness setting) {
    final data = MediaQuery.maybeOf(context);
    return MediaQuery(
      data: (data ?? const MediaQueryData()).copyWith(
        platformBrightness: setting,
      ),
      child: child,
    );
  }

  @override
  List<Field> get fields {
    return [
      BooleanField(
        name: "light mode?",
        initialValue: initialBrightness == Brightness.light,
      ),
    ];
  }

  @override
  Brightness valueFromQueryGroup(Map<String, String> group) {
    final value = valueOf<bool>("light mode?", group)!;
    return switch (value) {
      true => Brightness.light,
      false => Brightness.dark,
    };
  }
}

class BrightnessAddonConfig extends AddonConfig {
  const BrightnessAddonConfig(String value)
    : super('brightness', 'brightness:$value');
}
