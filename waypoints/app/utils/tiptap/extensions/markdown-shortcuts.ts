import { ContentConverters, ContentType, type Content, type ContentKind } from "@waypoints/domain";
import { Extension } from "@tiptap/core";
import { PluginKey } from "@tiptap/pm/state";
import { toDomain } from "#imports";

export interface BlockTypeDelegate {
  onChangeBlockType(blockID: string, content: Content): void;
}

export interface MarkdownShortcutsOptions {
  blockID?: string;
  orchestrator?: BlockTypeDelegate;
}

interface TriggerRule {
  pattern: RegExp;
  type: string;
}

const triggers: TriggerRule[] = [
  { type: ContentType.Heading3, pattern: /^###$/ },
  { type: ContentType.Heading2, pattern: /^##$/ },
  { type: ContentType.Heading1, pattern: /^#$/ },
  { type: ContentType.ListItem, pattern: /^[-*]$/ },
  { type: ContentType.EnumItem, pattern: /^\d+\.$/ },
  { type: ContentType.Blockquote, pattern: /^>$/ },
];

const name = "markdown-shortcuts";
const key = new PluginKey(name);

export const MarkdownShortcuts = Extension.create<MarkdownShortcutsOptions>({
  name,

  addOptions() {
    return {};
  },

  addKeyboardShortcuts() {
    const orchestrator = this.options.orchestrator;

    return {
      " ": ({ editor }) => {
        const blockID = this.options.blockID;
        if (!blockID || !orchestrator?.onChangeBlockType) return false;

        const { $from } = editor.state.selection;
        const textBefore = $from.parent.textBetween(0, $from.parentOffset);

        const match = triggers.find((t) => t.pattern.test(textBefore));
        if (!match) return false;

        // Delete the trigger text (positions are doc-absolute, not parent-relative)
        const from = $from.pos - $from.parentOffset;
        const to = $from.pos;
        editor.chain().deleteRange({ from, to }).run();

        // Convert the block
        const converter = ContentConverters[match.type as ContentKind];
        if (!converter) return true;

        const doc = editor.getJSON();
        const inner = doc.content?.[0]?.content ?? [];
        const rt = toDomain(inner);
        const richText = rt.segments.length > 0 ? rt : undefined;

        orchestrator.onChangeBlockType(blockID, converter(richText));
        return true;
      },
    };
  },
});
