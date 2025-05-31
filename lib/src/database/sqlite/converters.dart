import 'dart:convert';

import 'package:drift/drift.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/editor/attributed_text.dart';

class JsonConverter extends TypeConverter<Map<String, dynamic>, String>
    with
        JsonTypeConverter2<Map<String, dynamic>, String, Map<String, dynamic>> {
  const JsonConverter();

  @override
  Map<String, dynamic> fromJson(Map<String, dynamic> json) {
    return json;
  }

  @override
  Map<String, dynamic> toJson(Map<String, dynamic> json) {
    return json;
  }

  @override
  Map<String, dynamic> fromSql(String fromDB) {
    return fromJson(json.decode(fromDB) as Map<String, dynamic>);
  }

  @override
  String toSql(Map<String, dynamic> value) {
    return json.encode(toJson(value));
  }
}

class AttributedTextConverter extends TypeConverter<AttributedText, String>
    with JsonTypeConverter2<AttributedText, String, Map<String, dynamic>> {

  const AttributedTextConverter();
  @override
  AttributedText fromJson(Map<String, dynamic> json) {
    return AttributedTextJson.fromJson(json);
  }

  @override
  AttributedText fromSql(String fromDB) {
    return fromJson(json.decode(fromDB) as Map<String, dynamic>);
  }

  @override
  Map<String, dynamic> toJson(AttributedText value) {
    return value.toJson();
  }

  @override
  String toSql(AttributedText value) {
    return json.encode(toJson(value));
  }
}
