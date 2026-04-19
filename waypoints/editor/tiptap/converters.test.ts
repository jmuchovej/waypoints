import { describe, it, expect } from "vitest";
import { InlineKind } from "@waypoints/domain";
import type { RichText, InlineMark, TextSegment } from "@waypoints/domain";
import { toTipTapMark, toInlineMark, toTipTap, toDomain } from "./converters";

function seg(text: string, marks: InlineMark[] = []): TextSegment {
  return { text, marks };
}

function rt(segments: TextSegment[]): RichText {
  return { segments, plain_text: segments.map((s) => s.text).join("") };
}

describe("toTipTapMark", () => {
  it("converts plain marks (no data)", () => {
    const mark: InlineMark = { kind: InlineKind.Bold };
    expect(toTipTapMark(mark)).toEqual({ type: "bold" });
  });

  it("converts link mark with href", () => {
    const mark: InlineMark = { kind: InlineKind.Link, data: "https://example.com" };
    expect(toTipTapMark(mark)).toEqual({
      type: "link",
      attrs: { href: "https://example.com" },
    });
  });

  it("converts mention mark with id", () => {
    const mark: InlineMark = { kind: InlineKind.Mention, data: "some-uuid" };
    expect(toTipTapMark(mark)).toEqual({
      type: "mention",
      attrs: { id: "some-uuid" },
    });
  });

  it("converts math mark with engine attr", () => {
    const mark: InlineMark = { kind: InlineKind.Math, data: { engine: "katex" } };
    expect(toTipTapMark(mark)).toEqual({
      type: "math",
      attrs: { engine: "katex" },
    });
  });

  it("defaults math engine to typst", () => {
    const mark: InlineMark = { kind: InlineKind.Math, data: {} };
    expect(toTipTapMark(mark)).toEqual({
      type: "math",
      attrs: { engine: "typst" },
    });
  });
});

describe("toInlineMark", () => {
  it("converts plain TipTap marks", () => {
    expect(toInlineMark({ type: "italic" })).toEqual({ kind: "italic" });
  });

  it("converts link mark with href", () => {
    expect(toInlineMark({ type: "link", attrs: { href: "https://example.com" } })).toEqual({
      kind: "link",
      data: "https://example.com",
    });
  });

  it("converts mention mark with id", () => {
    expect(toInlineMark({ type: "mention", attrs: { id: "some-uuid" } })).toEqual({
      kind: "mention",
      data: "some-uuid",
    });
  });

  it("converts math mark with engine", () => {
    expect(toInlineMark({ type: "math", attrs: { engine: "katex" } })).toEqual({
      kind: "math",
      data: { engine: "katex" },
    });
  });

  it("defaults math engine to typst when missing", () => {
    expect(toInlineMark({ type: "math" })).toEqual({
      kind: "math",
      data: { engine: "typst" },
    });
  });
});

describe("mark round-trip", () => {
  it.each([
    { kind: InlineKind.Bold },
    { kind: InlineKind.Italic },
    { kind: InlineKind.Code },
    { kind: InlineKind.Strikethrough },
    { kind: InlineKind.Underline },
    { kind: InlineKind.Subscript },
    { kind: InlineKind.Superscript },
    { kind: InlineKind.Spoiler },
  ] satisfies InlineMark[])("round-trips plain mark: $kind", (mark) => {
    expect(toInlineMark(toTipTapMark(mark))).toEqual(mark);
  });

  it("round-trips link mark", () => {
    const mark: InlineMark = { kind: InlineKind.Link, data: "https://example.com" };
    expect(toInlineMark(toTipTapMark(mark))).toEqual(mark);
  });

  it("round-trips mention mark", () => {
    const mark: InlineMark = { kind: InlineKind.Mention, data: "some-uuid" };
    expect(toInlineMark(toTipTapMark(mark))).toEqual(mark);
  });

  it("round-trips math mark", () => {
    const mark: InlineMark = { kind: InlineKind.Math, data: { engine: "katex" } };
    expect(toInlineMark(toTipTapMark(mark))).toEqual(mark);
  });
});

describe("toTipTap", () => {
  it("returns empty array for empty RichText", () => {
    expect(toTipTap(rt([]))).toEqual([]);
  });

  it("converts a plain text segment", () => {
    expect(toTipTap(rt([seg("hello")]))).toEqual([{ type: "text", text: "hello" }]);
  });

  it("converts a segment with marks", () => {
    const result = toTipTap(
      rt([
        seg("bold link", [
          { kind: InlineKind.Bold },
          { kind: InlineKind.Link, data: "https://x.com" },
        ]),
      ]),
    );
    expect(result).toEqual([
      {
        type: "text",
        text: "bold link",
        marks: [{ type: "bold" }, { type: "link", attrs: { href: "https://x.com" } }],
      },
    ]);
  });

  it("filters out empty-text segments", () => {
    const result = toTipTap(rt([seg(""), seg("kept")]));
    expect(result).toEqual([{ type: "text", text: "kept" }]);
  });

  it("omits marks key when segment has no marks", () => {
    const result = toTipTap(rt([seg("plain")]));
    expect(result[0]).not.toHaveProperty("marks");
  });
});

describe("toDomain", () => {
  it("returns empty RichText for empty nodes", () => {
    expect(toDomain([])).toEqual({ segments: [], plain_text: "" });
  });

  it("converts text nodes to segments", () => {
    const result = toDomain([
      { type: "text", text: "hello " },
      { type: "text", text: "world" },
    ]);
    expect(result.segments).toHaveLength(2);
    expect(result.plain_text).toBe("hello world");
  });

  it("converts text node marks", () => {
    const result = toDomain([{ type: "text", text: "bold", marks: [{ type: "bold" }] }]);
    expect(result.segments[0]!.marks).toEqual([{ kind: "bold" }]);
  });

  it("handles emoji nodes", () => {
    const result = toDomain([{ type: "emoji", attrs: { emoji: "🎯" } }]);
    expect(result.segments).toEqual([{ text: "🎯", marks: [] }]);
    expect(result.plain_text).toBe("🎯");
  });

  it("skips nodes without text or emoji", () => {
    const result = toDomain([
      { type: "text", text: "kept" },
      { type: "hardBreak" },
      { type: "text" }, // no text property
    ]);
    expect(result.segments).toHaveLength(1);
    expect(result.plain_text).toBe("kept");
  });

  it("assigns empty marks array when node has no marks", () => {
    const result = toDomain([{ type: "text", text: "plain" }]);
    expect(result.segments[0]!.marks).toEqual([]);
  });
});
