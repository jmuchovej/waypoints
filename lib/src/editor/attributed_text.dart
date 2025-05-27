import 'package:flutter/material.dart';
import 'package:tempo/src/editor/super_editor.dart';

extension MultiAttributionSpanJSON on MultiAttributionSpan {
  Map<String, dynamic> toJson() {
    return {'start': start, 'end': end, 'attributions': attributions.toList()};
  }

  static MultiAttributionSpan fromJson(Map<String, dynamic> json) {
    final {
      'start': int start,
      'end': int end,
      'attributions': List<String> attributions,
    } = json;
    return MultiAttributionSpan(
      attributions: Set.from(attributions.map((a) => NamedAttribution(a))),
      start: start,
      end: end,
    );
  }
}

extension AttributedTextJSON on AttributedText {
  Map<String, dynamic> toJson() {
    return {
      'text': toPlainText(),
      'spans': computeAttributionSpans().map((span) => span.toJson()).toList(),
      'version': 1,
    };
  }

  static AttributedText fromJson(Map<String, dynamic> json) {
    if (json.isEmpty) return AttributedText();

    final text = json['text'] as String? ?? '';
    final multiSpans =
        (json['spans'] as List<dynamic>? ?? [])
            .cast<Map<String, dynamic>>()
            .map((sj) => MultiAttributionSpanJSON.fromJson(sj))
            .toList();
    final attributions = AttributedSpans();
    for (final multiSpan in multiSpans) {
      final MultiAttributionSpan(:start, :end) = multiSpan;
      for (final attribution in multiSpan.attributions) {
        attributions.addAttribution(
          newAttribution: attribution,
          start: start,
          end: end,
        );
      }
    }

    return AttributedText(text, attributions);
  }
}

extension AttributionJSON on Attribution {
  Map<String, dynamic> toJson() {
    switch (this) {
      case NamedAttribution _:
        return {'type': 'named', 'name': (this as NamedAttribution).name};
      case LinkAttribution _:
        return {'type': 'link', 'url': (this as LinkAttribution).plainTextUri};
      // TODO implement oklch support
      case ColorAttribution _:
        return {
          'type': 'color',
          'color': (this as ColorAttribution).color.toString(),
        };
      case BackgroundColorAttribution _:
        return {
          'type': 'background-color',
          'color': (this as BackgroundColorAttribution).color.toString(),
        };
      case FontSizeAttribution _:
        return {
          'type': 'font-family',
          'size': (this as FontSizeAttribution).fontSize,
        };
      case FontFamilyAttribution _:
        return {
          'type': 'font-family',
          'family': (this as FontFamilyAttribution).fontFamily,
        };
      default:
        return {'type': 'unknown', 'attribution': toString()};
    }
  }

  static Attribution fromJson(Map<String, dynamic> json) {
    final {'type': String type} = json;

    switch (type) {
      case 'named':
        return NamedAttribution(json['name'] as String);
      case 'link':
        return LinkAttribution(json['url'] as String);
      case 'color':
        return ColorAttribution(json['color'] as Color);
      case 'background-color':
        return BackgroundColorAttribution(json['color'] as Color);
      case 'font-size':
        return FontSizeAttribution(json['size'] as double);
      case 'font-family':
        return FontFamilyAttribution(json['family'] as String);
      default:
        final {'attribution': String? attribution} = json;
        return NamedAttribution('unknown_${attribution ?? 'attribution'}');
    }
  }
}
