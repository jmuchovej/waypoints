import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../editor/inline.dart';
import '../editor/super_editor.dart';
import '../infrastructure/logging.dart';
import '../editor/nodes/task_node.dart' show taskStyles;
import 'tempo_stylesheet.dart';

const BlockSelector blockSelectorH1 = BlockSelector("header1");
const BlockSelector blockSelectorH2 = BlockSelector("header2");
const BlockSelector blockSelectorH3 = BlockSelector("header3");
const BlockSelector blockSelectorList = BlockSelector("listItem");
const BlockSelector blockSelectorP = BlockSelector("paragraph");
const BlockSelector blockSelectorQuote = BlockSelector("blockquote");
// Document header block attribution
const documentHeaderAttribution = NamedAttribution('documentHeader');

Stylesheet buildStylesheet(WidgetRef ref) {
  final stylesheet = ref.read(tempoStylesheetProvider);
  final textStyles = stylesheet.textStyles;
  final spacing = stylesheet.spacing;
  final colors = stylesheet.colors;

  return Stylesheet(
    documentPadding: spacing.documentPadding,
    rules: [
      StyleRule(BlockSelector.all, (doc, docNode) {
        // Log all nodes being styled to understand the pipeline
        tempoLog.finest(
          '📋 Stylesheet applying to node: ${docNode.runtimeType}, '
          'id: ${docNode.id}, '
          'blockType: ${docNode.getMetadataValue("blockType")}',
        );

        return {
          Styles.maxWidth: spacing.maxContentWidth,
          Styles.textStyle: textStyles.contentBody,
        };
      }),
      StyleRule(blockSelectorH1, (doc, docNode) {
        final textStyle = textStyles.sectionHeader;
        final padding = textStyles.paddingFor(textStyle);
        return {
          Styles.padding: CascadingPadding.only(
            top: padding.top,
            bottom: padding.bottom,
          ),
          Styles.textStyle: textStyle,
        };
      }),
      StyleRule(blockSelectorH2, (doc, docNode) {
        final textStyle = textStyles.subsectionHeader;
        final padding = textStyles.paddingFor(textStyle);
        return {
          Styles.padding: CascadingPadding.only(
            top: padding.top,
            bottom: padding.bottom,
          ),
          Styles.textStyle: textStyle,
        };
      }),
      StyleRule(blockSelectorH3, (doc, docNode) {
        final textStyle = textStyles.cardTitle;
        final padding = textStyles.paddingFor(textStyle);
        return {
          Styles.padding: CascadingPadding.only(
            top: padding.top,
            bottom: padding.bottom,
          ),
          Styles.textStyle: textStyle,
        };
      }),

      StyleRule(blockSelectorP, (doc, docNode) {
        return {Styles.padding: CascadingPadding.only(top: spacing.paragraphSpacing)};
      }),
      StyleRule(blockSelectorP.after("header1"), (doc, docNode) {
        return {Styles.padding: const CascadingPadding.only(top: 0)};
      }),
      StyleRule(blockSelectorP.after("header2"), (doc, docNode) {
        return {Styles.padding: const CascadingPadding.only(top: 0)};
      }),
      StyleRule(blockSelectorP.after("header3"), (doc, docNode) {
        return {Styles.padding: const CascadingPadding.only(top: 0)};
      }),
      StyleRule(blockSelectorList, (doc, docNode) {
        return {Styles.padding: CascadingPadding.only(top: spacing.paragraphSpacing)};
      }),
      StyleRule(blockSelectorQuote, (doc, docNode) {
        return {
          Styles.textStyle: const TextStyle(
            color: Colors.grey,
            fontSize: 20,
            fontWeight: FontWeight.bold,
            height: 1.4,
          ),
        };
      }),
      taskStyles,
      StyleRule(BlockSelector.all.last(), (doc, docNode) {
        return {Styles.padding: const CascadingPadding.only(bottom: 96)};
      }),
    ],
    inlineTextStyler: defaultInlineTextStyler,
    inlineWidgetBuilders: [
      inlineIconBuilder,
      inlineNetworkImageBuilder,
      inlineAssetImageBuilder,
    ],
  );
}

TextStyle defaultInlineTextStyler(
  Set<Attribution> attributions,
  TextStyle existingStyle,
) {
  return existingStyle.merge(defaultStyleBuilder(attributions));
}

/// Creates [TextStyles] for the standard [SuperEditor].
TextStyle defaultStyleBuilder(Set<Attribution> attributions) {
  TextStyle newStyle = const TextStyle();

  for (final attribution in attributions) {
    if (attribution == boldAttribution) {
      newStyle = newStyle.copyWith(fontWeight: FontWeight.bold);
    } else if (attribution == italicsAttribution) {
      newStyle = newStyle.copyWith(fontStyle: FontStyle.italic);
    } else if (attribution == underlineAttribution) {
      newStyle = newStyle.copyWith(
        decoration:
            newStyle.decoration == null
                ? TextDecoration.underline
                : TextDecoration.combine([
                  TextDecoration.underline,
                  newStyle.decoration!,
                ]),
      );
    } else if (attribution == strikethroughAttribution) {
      newStyle = newStyle.copyWith(
        decoration:
            newStyle.decoration == null
                ? TextDecoration.lineThrough
                : TextDecoration.combine([
                  TextDecoration.lineThrough,
                  newStyle.decoration!,
                ]),
      );
    } else if (attribution == superscriptAttribution) {
      newStyle = newStyle.copyWith(
        fontFeatures: [const FontFeature.superscripts()],
      );
    } else if (attribution == subscriptAttribution) {
      newStyle = newStyle.copyWith(
        fontFeatures: [const FontFeature.subscripts()],
      );
    } else if (attribution is ColorAttribution) {
      newStyle = newStyle.copyWith(color: attribution.color);
    } else if (attribution is BackgroundColorAttribution) {
      newStyle = newStyle.copyWith(backgroundColor: attribution.color);
    } else if (attribution is FontSizeAttribution) {
      newStyle = newStyle.copyWith(fontSize: attribution.fontSize);
    } else if (attribution is FontFamilyAttribution) {
      newStyle = newStyle.copyWith(fontFamily: attribution.fontFamily);
    } else if (attribution is LinkAttribution) {
      newStyle = newStyle.copyWith(
        color: Colors.lightBlue,
        decoration: TextDecoration.underline,
      );
    } else if (attribution == documentHeaderAttribution) {
      // final colorScheme = ColorScheme.of(context);
      // final textStyle = TextTheme.of(context).displayLarge!;
      // newStyle = newStyle.copyWith(
      //   fontSize: textStyle.fontSize,
      //   fontWeight: FontWeight.bold,
      //   color: colorScheme.onSurface,
      // ),
    }
  }
  return newStyle;
}
