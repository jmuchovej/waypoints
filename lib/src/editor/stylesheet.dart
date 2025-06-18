import 'package:flutter/material.dart';

import '../infrastructure/logging.dart';
import '../things/task.dart' show taskStyles;
import 'super_editor.dart';

const BlockSelector blockSelectorDocumentHeader = BlockSelector(
  "documentHeader",
);

const BlockSelector blockSelectorH1 = BlockSelector("header1");
const BlockSelector blockSelectorH2 = BlockSelector("header2");
const BlockSelector blockSelectorH3 = BlockSelector("header3");
const BlockSelector blockSelectorList = BlockSelector("listItem");
const BlockSelector blockSelectorP = BlockSelector("paragraph");
const BlockSelector blockSelectorQuote = BlockSelector("blockquote");
// Document header block attribution
const documentHeaderAttribution = NamedAttribution('documentHeader');

Stylesheet buildStylesheet(BuildContext context) {
  return Stylesheet(
    documentPadding: EdgeInsets.symmetric(horizontal: 50.0, vertical: 24.0),
    rules: [
      StyleRule(BlockSelector.all, (doc, docNode) {
        // Log all nodes being styled to understand the pipeline
        tempoLog.fine(
          '📋 Stylesheet applying to node: ${docNode.runtimeType}, '
          'id: ${docNode.id}, '
          'blockType: ${docNode.getMetadataValue("blockType")}',
        );

        return {
          Styles.maxWidth: 800.0, // Content width constraint
          Styles.textStyle: Theme.of(context).textTheme.bodyMedium,
        };
      }),
      StyleRule(blockSelectorH1, (doc, docNode) {
        return {
          Styles.padding: const CascadingPadding.only(top: 40),
          Styles.textStyle: Theme.of(context).textTheme.displayLarge,
        };
      }),
      StyleRule(blockSelectorH2, (doc, docNode) {
        return {
          Styles.padding: const CascadingPadding.only(top: 32),
          Styles.textStyle: Theme.of(context).textTheme.displayMedium,
        };
      }),
      StyleRule(blockSelectorH3, (doc, docNode) {
        return {
          Styles.padding: const CascadingPadding.only(top: 28),
          Styles.textStyle: Theme.of(context).textTheme.displaySmall,
        };
      }),

      // Document header styling (uses theme colors and typography)
      StyleRule(blockSelectorDocumentHeader, (doc, docNode) {
        final colorScheme = Theme.of(context).colorScheme;
        final textTheme = Theme.of(context).textTheme;

        // Log when DocumentHeader stylesheet rule is applied
        tempoLog.info(
          '📝 DocumentHeader stylesheet rule APPLIED to node: ${docNode.runtimeType}, '
          'blockType: ${docNode.getMetadataValue("blockType")}',
        );

        return {
          Styles.padding: const CascadingPadding.only(bottom: 32, top: 0),
          Styles.textStyle:
              textTheme.displayLarge?.copyWith(
                fontWeight: FontWeight.bold,
                color: colorScheme.onSurface,
              ) ??
              TextStyle(
                fontSize: 32,
                fontWeight: FontWeight.bold,
                color: colorScheme.onSurface,
              ),
        };
      }),
      StyleRule(blockSelectorP, (doc, docNode) {
        return {Styles.padding: const CascadingPadding.only(top: 24)};
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
        return {Styles.padding: const CascadingPadding.only(top: 24)};
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
    }
  }
  return newStyle;
}
