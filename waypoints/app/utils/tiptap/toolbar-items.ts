import { InlineKind } from "@waypoints/domain";
import type { EditorToolbarItem } from "#ui/components/EditorToolbar.vue";

/**
 * Nuxt UI narrows `mark` to its built-in set; we have custom marks
 * (strikethrough, subscript, superscript, spoiler) so we widen the type.
 */
export const inlineToolbarItems = [
  {
    kind: "mark",
    mark: InlineKind.Bold,
    icon: "i-lucide-bold",
    tooltip: { text: "Bold", kbds: ["meta", "B"] },
  },
  {
    kind: "mark",
    mark: InlineKind.Italic,
    icon: "i-lucide-italic",
    tooltip: { text: "Italic", kbds: ["meta", "I"] },
  },
  {
    kind: "mark",
    mark: InlineKind.Strikethrough,
    icon: "i-lucide-strikethrough",
    tooltip: { text: "Strikethrough", kbds: ["meta", "shift", "S"] },
  },
  {
    kind: "mark",
    mark: InlineKind.Underline,
    icon: "i-lucide-underline",
    tooltip: { text: "Underline", kbds: ["meta", "U"] },
  },
  {
    kind: "mark",
    mark: InlineKind.Code,
    icon: "i-lucide-code",
    tooltip: { text: "Code", kbds: ["meta", "E"] },
  },
  {
    kind: "mark",
    mark: InlineKind.Subscript,
    icon: "i-lucide-subscript",
    tooltip: { text: "Subscript", kbds: ["meta", ","] },
  },
  {
    kind: "mark",
    mark: InlineKind.Superscript,
    icon: "i-lucide-superscript",
    tooltip: { text: "Superscript", kbds: ["meta", "."] },
  },
  {
    kind: "mark",
    mark: InlineKind.Spoiler,
    icon: "i-lucide-eye-off",
    tooltip: { text: "Spoiler", kbds: ["meta", "shift", "H"] },
  },
] as EditorToolbarItem[];

export const headingToolbarItems: EditorToolbarItem[] = [
  {
    kind: "heading",
    level: 1,
    icon: "i-lucide-heading-1",
    tooltip: { text: "Heading 1", kbds: ["meta", "alt", "1"] },
  },
  {
    kind: "heading",
    level: 2,
    icon: "i-lucide-heading-2",
    tooltip: { text: "Heading 2", kbds: ["meta", "alt", "2"] },
  },
  {
    kind: "heading",
    level: 3,
    icon: "i-lucide-heading-3",
    tooltip: { text: "Heading 3", kbds: ["meta", "alt", "3"] },
  },
];
