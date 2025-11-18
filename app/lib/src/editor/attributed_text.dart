import 'dart:convert';

import 'package:flutter/material.dart';

import 'super_editor.dart';

/// Extension methods for AttributedText JSON serialization
extension AttributedTextJson on AttributedText {
  /// Convert AttributedText to a JSON-serializable Map
  Map<String, dynamic> toJson() {
    final Map<String, dynamic> json = {
      'text': toPlainText(),
      'spans': <Map<String, dynamic>>[],
    };

    json["spans"] = spans
        .collapseSpans(contentLength: length)
        .map(
          (ms) => {
            "start": ms.start,
            "end": ms.end,
            'attributions': ms.attributions
                .map((attr) => attr.toJson())
                .toList(),
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

  /// Convert AttributedText to Foundation format (compatible with Rust TextMark)
  Map<String, dynamic> toFoundationFormat() {
    final Map<String, dynamic> foundation = {
      'body': toPlainText(includePlaceholders: false),
      'marks': <Map<String, dynamic>>[],
    };

    // Convert each attribution span to a TextMark
    final allSpans = spans.collapseSpans(contentLength: length);
    for (final span in allSpans) {
      for (final attribution in span.attributions) {
        final markType = AttributedTextJson._attributionToMarkType(attribution);
        if (markType != null) {
          final mark = {
            'mark_type': markType,
            'start': span.start,
            'end': span.end,
          };

          // Add additional data for complex attributions
          final data = AttributedTextJson._attributionToData(attribution);
          if (data != null) {
            mark['data'] = data;
          }

          foundation['marks'].add(mark);
        }
      }
    }

    return foundation;
  }

  /// Convert Foundation format to AttributedText
  static AttributedText fromFoundationFormat(Map<String, dynamic> foundation) {
    final body = foundation['body'] as String? ?? '';
    final attributedText = AttributedText(body);

    final marksList = foundation['marks'] as List<dynamic>? ?? [];
    for (final markJson in marksList) {
      final mark = markJson as Map<String, dynamic>;
      final markType = mark['mark_type'] as String;
      final start = mark['start'] as int;
      final end = mark['end'] as int;
      final data = mark['data'] as Map<String, dynamic>?;

      final attribution = AttributedTextJson._markTypeToAttribution(
        markType,
        data,
      );
      if (attribution != null) {
        attributedText.addAttribution(attribution, SpanRange(start, end));
      }
    }

    return attributedText;
  }

  /// Convert SuperEditor Attribution to Foundation MarkType string
  static String? _attributionToMarkType(Attribution attribution) {
    switch (attribution) {
      case NamedAttribution(:final id):
        // Handle standard named attributions
        switch (id) {
          case 'bold':
            return 'bold';
          case 'italics':
            return 'italic';
          case 'underline':
            return 'underline';
          case 'strikethrough':
            return 'strikethrough';
          case 'code':
            return 'code';
          default:
            // Unknown named attribution, use the id as-is
            return id;
        }
      case ScriptAttribution(:final type):
        return type; // 'superscript' or 'subscript'
      case LinkAttribution():
        return 'link';
      case FontSizeAttribution():
        return 'font_size';
      case FontFamilyAttribution():
        return 'font_family';
      case ColorAttribution():
        return 'color';
      case BackgroundColorAttribution():
        return 'background_color';
      default:
        // Unknown attribution type, skip
        return null;
    }
  }

  /// Extract additional data from complex attributions
  static Map<String, dynamic>? _attributionToData(Attribution attribution) {
    switch (attribution) {
      case LinkAttribution(:final uri, :final plainTextUri):
        return {'url': plainTextUri, if (uri != null) 'uri': uri.toString()};
      case FontSizeAttribution(:final fontSize):
        return {'size': fontSize};
      case FontFamilyAttribution(:final fontFamily):
        return {'family': fontFamily};
      case ColorAttribution(:final color):
        return {'color': color.toJson()};
      case BackgroundColorAttribution(:final color):
        return {'color': color.toJson()};
      default:
        return null;
    }
  }

  /// Convert Foundation MarkType string + data back to Attribution
  static Attribution? _markTypeToAttribution(
    String markType,
    Map<String, dynamic>? data,
  ) {
    switch (markType) {
      case 'bold':
        return const NamedAttribution('bold');
      case 'italic':
        return const NamedAttribution('italics');
      case 'underline':
        return const NamedAttribution('underline');
      case 'strikethrough':
        return const NamedAttribution('strikethrough');
      case 'code':
        return const NamedAttribution('code');
      case 'superscript':
        return const ScriptAttribution.superscript();
      case 'subscript':
        return const ScriptAttribution.subscript();
      case 'link':
        if (data != null && data['url'] is String) {
          final url = data['url'] as String;
          final uriStr = data['uri'] as String?;
          return uriStr != null
              ? LinkAttribution(url, Uri.parse(uriStr))
              : LinkAttribution(url);
        }
        return null;
      case 'font_size':
        if (data != null && data['size'] is num) {
          return FontSizeAttribution(data['size'].toDouble());
        }
        return null;
      case 'font_family':
        if (data != null && data['family'] is String) {
          return FontFamilyAttribution(data['family'] as String);
        }
        return null;
      case 'color':
        if (data != null && data['color'] is Map<String, dynamic>) {
          final colorData = data['color'] as Map<String, dynamic>;
          return ColorAttribution(ColorJson.fromJson(colorData));
        }
        return null;
      case 'background_color':
        if (data != null && data['color'] is Map<String, dynamic>) {
          final colorData = data['color'] as Map<String, dynamic>;
          return BackgroundColorAttribution(ColorJson.fromJson(colorData));
        }
        return null;
      default:
        // Unknown mark type, treat as named attribution
        return NamedAttribution(markType);
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
        return {'type': id, 'url': plainTextUri, 'uri': uri?.toString()};
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

extension ColorJson on Color {
  Map<String, int> toJson() {
    return {
      'r': (r * 255).round(),
      'g': (g * 255).round(),
      'b': (b * 255).round(),
      'a': (a * 255).round(),
    };
  }

  static Color fromJson(Map<String, dynamic> json) {
    return Color.fromARGB(
      json['a'] as int,
      json['r'] as int,
      json['g'] as int,
      json['b'] as int,
    );
  }
}

/// Extension to provide easy access to MultiAttributionSpan's attributions
extension MultiAttributionSpanExtensions on MultiAttributionSpan {
  /// Get the attributions as a List (instead of Set)
  List<Attribution> get attributionsList => attributions.toList();
}
