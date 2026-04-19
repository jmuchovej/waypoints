/**
 * Default constructors for domain types, mirroring Rust-side `Default` impls.
 */

import type { Content, MarkData, MathEngine, RichText } from "./types.g";

export const emptyRichText: RichText = { plain_text: "", segments: [] };

export function toRichText(text: string): RichText {
  return { plain_text: text, segments: [{ text, marks: [] }] };
}

export function paragraph(rt?: RichText): Content {
  return { type: "paragraph", rich_text: rt ?? emptyRichText };
}

export function heading1(rt?: RichText): Content {
  return { type: "heading-1", rich_text: rt ?? emptyRichText };
}

export function heading2(rt?: RichText): Content {
  return { type: "heading-2", rich_text: rt ?? emptyRichText };
}

export function heading3(rt?: RichText): Content {
  return { type: "heading-3", rich_text: rt ?? emptyRichText };
}

export function blockquote(): Content {
  return { type: "blockquote" };
}

export function listItem(rt?: RichText): Content {
  return { type: "list-item", rich_text: rt ?? emptyRichText };
}

export function enumItem(rt?: RichText): Content {
  return { type: "enum-item", rich_text: rt ?? emptyRichText };
}

export function callout(icon = "💡"): Content {
  return { type: "callout", icon };
}

export function code(language = ""): Content {
  return { type: "code", language, lines: [] };
}

export function table(): Content {
  return {
    type: "table",
    cells: [[emptyRichText]],
    col_size: null,
    row_size: null,
    col_header: null,
    row_header: null,
    fill_width: null,
    col_colors: null,
    row_colors: null,
  };
}

export function math(expression: string = "", engine: MathEngine = "typst"): Content {
  return { type: "math", expression, engine };
}

export function divider(): Content {
  return { type: "divider" };
}

export function isEmptyRichText(rt: RichText): boolean {
  return rt.segments.length === 0 || rt.plain_text === "";
}

/** True when a Content block has no user-entered data. */
export function isEmpty(c: Content): boolean {
  switch (c.type) {
    case "paragraph":
    case "heading-1":
    case "heading-2":
    case "heading-3":
    case "list-item":
    case "enum-item":
      return isEmptyRichText(c.rich_text);
    case "code":
      return c.lines.length === 0 || c.lines.every((l) => l === "");
    case "math":
      return c.expression === "";
    case "table":
      return c.cells.every((row) => row.every(isEmptyRichText));
    case "blockquote":
    case "callout":
    case "divider":
      return false;
  }
}

export function contentData(content?: Content): MarkData {
  return { mark: "content", ...(content ?? paragraph()) };
}
