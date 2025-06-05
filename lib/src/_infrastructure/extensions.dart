extension StringExtension on String {
  String capitalize() {
    if (isEmpty) return this;
    return "${this[0].toUpperCase()}${substring(1)}";
  }
}

extension EnumDisplayStrings on Enum {
  String toDisplayString() => name.toUpperCase();
}
