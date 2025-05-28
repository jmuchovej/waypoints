import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:tempo/src/editor/super_editor.dart';

/// Extension methods for AttributedText JSON serialization
extension AttributedTextJson on AttributedText {
  /// Convert AttributedText to a JSON-serializable Map
  Map<String, dynamic> toJson() {
    final Map<String, dynamic> json = {
      'text': toPlainText(),
      'spans': <Map<String, dynamic>>[],
    };

    json["spans"] =
        spans
            .collapseSpans(contentLength: length)
            .map(
              (ms) => {
                "start": ms.start,
                "end": ms.end,
                'attributions':
                    ms.attributions.map((attr) => attr.toJson()).toList(),
              },
            )
            .toList();

    return json;
  }

  /// Convert AttributedText to JSON string
  String toJsonString() => jsonEncode(toJson());

  /// Create AttributedText from a JSON Map
  static AttributedText fromJson(Map<String, dynamic> json) {
    final text = json['text'] as String? ?? '';
    final attributedText = AttributedText(text);

    // Add spans if present
    final spansList = json['spans'] as List<dynamic>? ?? [];
    for (final spanJson in spansList) {
      // Handle the Span structure from toJson which has 'attributions' array
      final attributionsList = spanJson['attributions'] as List<dynamic>? ?? [];
      final start = spanJson['start'] as int;
      final end = spanJson['end'] as int;

      // Process each attribution in the span
      for (final attributionJson in attributionsList) {
        final attribution = AttributionJson.fromJson(
          attributionJson as Map<String, dynamic>,
        );
        if (attribution != null) {
          attributedText.addAttribution(attribution, SpanRange(start, end));
        }
      }
    }

    return attributedText;
  }

  /// Create AttributedText from JSON string
  static AttributedText fromJsonString(String json) {
    if (json.isEmpty) return AttributedText();
    try {
      final data = jsonDecode(json) as Map<String, dynamic>;
      return fromJson(data);
    } catch (e) {
      // If JSON is invalid, return empty AttributedText
      return AttributedText();
    }
  }
}

/// Extension methods for Attribution JSON serialization
extension AttributionJson on Attribution {
  /// Convert Attribution to JSON
  Map<String, dynamic> toJson() {
    // Check for specific attribution types
    switch (this) {
      case NamedAttribution _:
        return {'type': 'named', 'id': id};
      case ScriptAttribution(:final type):
        return {'type': type};
      case LinkAttribution(:final uri, :final plainTextUri):
        return {
          'type': id,
          'url': plainTextUri,
          'uri': uri?.toString(),
        };
      case FontSizeAttribution(:final fontSize):
        return {'type': id, 'size': fontSize};
      case FontFamilyAttribution(:final fontFamily):
        return {'type': id, 'family': fontFamily};
      case ColorAttribution(:final color):
        return {'type': id, 'color': color.toJson()};
      case BackgroundColorAttribution(:final color):
        return {'type': id, 'color': color.toJson()};
      default:
        return {'type': 'unknown', 'as_string': toString()};
    }
  }

  /// Convert JSON back to Attribution
  static Attribution? fromJson(Map<String, dynamic> json) {
    final type = json['type'] as String?;
    switch (type) {
      case 'named':
        return NamedAttribution(json['id'] as String);

      case 'superscript':
        return const ScriptAttribution.superscript();

      case 'subscript':
        return const ScriptAttribution.subscript();

      case 'link':
        final url = json['url'] as String;
        final uriStr = json['uri'] as String?;
        return uriStr != null
            ? LinkAttribution(url, Uri.parse(uriStr))
            : LinkAttribution(url);

      case 'fontSize':
        return FontSizeAttribution(json['size'] as double);

      case 'fontFamily':
        return FontFamilyAttribution(json['family'] as String);

      case 'color':
        final colorData = json['color'] as Map<String, double>?;
        if (colorData != null) {
          return ColorAttribution(ColorJson.fromJson(colorData));
        }
        return null;

      case 'backgroundColor':
        final colorData = json['color'] as Map<String, double>?;
        if (colorData != null) {
          return BackgroundColorAttribution(ColorJson.fromJson(colorData));
        }
        return null;

      default:
        return null;
    }
  }
}

/// Extension to provide easy access to MultiAttributionSpan's attributions
extension MultiAttributionSpanExtensions on MultiAttributionSpan {
  /// Get the attributions as a List (instead of Set)
  List<Attribution> get attributionsList => attributions.toList();
}

extension ColorJson on Color {
  Map<String, double> toJson() {
    return {
      'r': r,
      'g': g,
      'b': b,
      'a': a,
    };
  }

  static Color fromJson(Map<String, double> json) {
    return Color.fromARGB(
      json['a'] as int,
      json['r'] as int,
      json['g'] as int,
      json['b'] as int,
    );
  }
}
