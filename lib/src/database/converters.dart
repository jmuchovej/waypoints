import 'dart:convert';

import 'package:drift/drift.dart';

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
