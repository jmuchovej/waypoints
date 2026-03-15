import type { ContentKind } from "./checks";
import {
  heading1,
  heading2,
  heading3,
  enumItem,
  blockquote,
  listItem,
  paragraph,
  callout,
  code,
  math,
  table,
  divider,
} from "./defaults";
import { ContentType } from "./schema.g";
import type { Content, RichText } from "./types.g";

export * from "./types.g";
export * from "./schema.g";
export * from "./checks";
export * from "./defaults";

export type ConversionFn = (rt: RichText | undefined) => Content;

export const ContentConverters: Record<ContentKind, ConversionFn> = {
  [ContentType.Paragraph]: (rt) => paragraph(rt),
  [ContentType.Heading1]: (rt) => heading1(rt),
  [ContentType.Heading2]: (rt) => heading2(rt),
  [ContentType.Heading3]: (rt) => heading3(rt),
  [ContentType.ListItem]: (rt) => listItem(rt),
  [ContentType.EnumItem]: (rt) => enumItem(rt),
  [ContentType.Blockquote]: () => blockquote(),
  [ContentType.Callout]: () => callout(),
  [ContentType.Code]: () => code(),
  [ContentType.Math]: () => math(),
  [ContentType.Table]: () => table(),
  [ContentType.Divider]: () => divider(),
};
