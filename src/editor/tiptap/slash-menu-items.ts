import { ContentType, ContentConverters, type ConversionFn } from "@waypoints/domain";

export type SlashMenuCategory = "text" | "lists" | "advanced";

export interface SlashMenuItem {
  id: string;
  label: string;
  icon: string;
  description: string;
  keywords: string[];
  category: SlashMenuCategory;
  toContent: ConversionFn;
}

export const slashMenuItems: SlashMenuItem[] = [
  // ── Text ──────────────────────────────────────────────────────────────
  {
    id: ContentType.Paragraph,
    label: "Text",
    icon: "i-lucide-type",
    description: "Plain text paragraph",
    keywords: ["paragraph", "text", "plain"],
    category: "text",
    toContent: ContentConverters[ContentType.Paragraph],
  },
  {
    id: ContentType.Heading1,
    label: "Heading 1",
    icon: "i-lucide-heading-1",
    description: "Large section heading",
    keywords: ["heading", "h1", "title"],
    category: "text",
    toContent: ContentConverters[ContentType.Heading1],
  },
  {
    id: ContentType.Heading2,
    label: "Heading 2",
    icon: "i-lucide-heading-2",
    description: "Medium section heading",
    keywords: ["heading", "h2", "subtitle"],
    category: "text",
    toContent: ContentConverters[ContentType.Heading2],
  },
  {
    id: ContentType.Heading3,
    label: "Heading 3",
    icon: "i-lucide-heading-3",
    description: "Small section heading",
    keywords: ["heading", "h3"],
    category: "text",
    toContent: ContentConverters[ContentType.Heading3],
  },

  // ── Lists ─────────────────────────────────────────────────────────────
  {
    id: ContentType.ListItem,
    label: "Bullet List",
    icon: "i-lucide-list",
    description: "Unordered list item",
    keywords: ["bullet", "list", "unordered", "ul"],
    category: "lists",
    toContent: ContentConverters[ContentType.ListItem],
  },
  {
    id: ContentType.EnumItem,
    label: "Numbered List",
    icon: "i-lucide-list-ordered",
    description: "Ordered list item",
    keywords: ["number", "ordered", "ol", "enum"],
    category: "lists",
    toContent: ContentConverters[ContentType.EnumItem],
  },

  // ── Advanced ──────────────────────────────────────────────────────────
  {
    id: ContentType.Blockquote,
    label: "Quote",
    icon: "i-lucide-quote",
    description: "Block quote",
    keywords: ["quote", "blockquote"],
    category: "advanced",
    toContent: ContentConverters[ContentType.Blockquote],
  },
  {
    id: ContentType.Callout,
    label: "Callout",
    icon: "i-lucide-message-square-warning",
    description: "Highlighted callout block",
    keywords: ["callout", "info", "warning", "tip"],
    category: "advanced",
    toContent: ContentConverters[ContentType.Callout],
  },
  {
    id: ContentType.Code,
    label: "Code",
    icon: "i-lucide-code-2",
    description: "Code block",
    keywords: ["code", "pre", "snippet"],
    category: "advanced",
    toContent: ContentConverters[ContentType.Code],
  },
  {
    id: ContentType.Table,
    label: "Table",
    icon: "i-lucide-table",
    description: "Table grid",
    keywords: ["table", "grid", "spreadsheet"],
    category: "advanced",
    toContent: ContentConverters[ContentType.Table],
  },
  {
    id: ContentType.Math,
    label: "Math",
    icon: "i-lucide-sigma",
    description: "Math equation block",
    keywords: ["math", "equation", "formula", "latex", "typst"],
    category: "advanced",
    toContent: ContentConverters[ContentType.Math],
  },
  {
    id: ContentType.Divider,
    label: "Divider",
    icon: "i-lucide-minus",
    description: "Horizontal divider",
    keywords: ["divider", "hr", "separator", "line"],
    category: "advanced",
    toContent: ContentConverters[ContentType.Divider],
  },
];

const categoryOrder: SlashMenuCategory[] = ["text", "lists", "advanced"];

export const categoryLabels: Record<SlashMenuCategory, string> = {
  text: "Text",
  lists: "Lists",
  advanced: "Advanced",
};

export function filterSlashItems(query: string): SlashMenuItem[] {
  const items = query
    ? slashMenuItems.filter(
        (item) =>
          item.label.toLowerCase().includes(query.toLowerCase()) ||
          item.keywords.some((kw) => kw.includes(query.toLowerCase())),
      )
    : slashMenuItems;

  // Sort by category order
  return items.sort(
    (a, b) => categoryOrder.indexOf(a.category) - categoryOrder.indexOf(b.category),
  );
}

/** Group filtered items by category, preserving order. */
export function groupSlashItems(
  items: SlashMenuItem[],
): { category: SlashMenuCategory; label: string; items: SlashMenuItem[] }[] {
  const groups: Map<SlashMenuCategory, SlashMenuItem[]> = new Map();
  for (const item of items) {
    const list = groups.get(item.category) ?? [];
    list.push(item);
    groups.set(item.category, list);
  }
  return categoryOrder
    .filter((cat) => groups.has(cat))
    .map((cat) => ({
      category: cat,
      label: categoryLabels[cat],
      items: groups.get(cat)!,
    }));
}
