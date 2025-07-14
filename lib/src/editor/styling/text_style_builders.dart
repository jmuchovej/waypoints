import 'package:flutter/material.dart';

import '../../styling/text_styles.dart';
import '../editor.dart';

typedef TextStyleBuilder = TextStyle Function(Set<Attribution>);

TextStyleBuilder headerTextStyleBuilder(BuildContext context) => (
  Set<Attribution> attributions,
) {
  final colorScheme = ColorScheme.of(context);
  final textStyles = TempoTextStyles.fromContext(context);
  return textStyles.documentHeader.copyWith(
    color: colorScheme.onSurface,
    fontWeight: FontWeight.w700,
  );
};

TextStyleBuilder headerHintStyleBuilder(BuildContext context) => (
  Set<Attribution> attributions,
) {
  final colorScheme = ColorScheme.of(context);
  final textStyles = TempoTextStyles.fromContext(context);
  return textStyles.documentHeader.copyWith(
    color: colorScheme.onSurface.withValues(alpha: 0.3),
    fontWeight: FontWeight.w700,
  );
};

TextStyleBuilder header1TextStyleBuilder(BuildContext context) => (
  Set<Attribution> attributions,
) {
  final textStyles = TempoTextStyles.fromContext(context);
  return textStyles.sectionHeader.copyWith(
    fontWeight: FontWeight.w600,
  );
};

TextStyleBuilder header2TextStyleBuilder(BuildContext context) => (
  Set<Attribution> attributions,
) {
  final textStyles = TempoTextStyles.fromContext(context);
  return textStyles.subsectionHeader.copyWith(
    fontWeight: FontWeight.w600,
  );
};

TextStyleBuilder header3TextStyleBuilder(BuildContext context) => (
  Set<Attribution> attributions,
) {
  final textStyles = TempoTextStyles.fromContext(context);
  return textStyles.cardTitle.copyWith(
    fontWeight: FontWeight.w600,
  );
};

TextStyleBuilder defaultTextStyleBuilder(BuildContext context) => (
  Set<Attribution> attributions,
) {
  final textStyles = TempoTextStyles.fromContext(context);
  return textStyles.contentBody;
};
