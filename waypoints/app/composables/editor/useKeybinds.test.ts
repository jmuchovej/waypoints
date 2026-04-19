import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { createApp, type App } from "vue";
import { useEditorKeybind, type ParsedKeyCombo } from "./useKeybinds";

// ── Helpers ─────────────────────────────────────────────────────────────

let app: App;
let kbd: ReturnType<typeof useEditorKeybind>;

function setupKbd(userAgent: string) {
  Object.defineProperty(navigator, "userAgent", {
    value: userAgent,
    configurable: true,
  });
  app = createApp({ setup: () => (kbd = useEditorKeybind()) && (() => {}) });
  app.mount(document.createElement("div"));
}

function keyEvent(
  key: string,
  mods: Partial<Pick<KeyboardEvent, "metaKey" | "ctrlKey" | "shiftKey" | "altKey">> = {},
): KeyboardEvent {
  return new KeyboardEvent("keydown", {
    key,
    metaKey: mods.metaKey ?? false,
    ctrlKey: mods.ctrlKey ?? false,
    shiftKey: mods.shiftKey ?? false,
    altKey: mods.altKey ?? false,
  });
}

// ── Mac tests ───────────────────────────────────────────────────────────

describe("useKbd (macOS)", () => {
  beforeEach(() => setupKbd("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)"));
  afterEach(() => app.unmount());

  describe("parseKeyCombo", () => {
    it("parses ⌘ + k", () => {
      const parsed = kbd.parseKeyCombo("⌘ + k");
      expect(parsed).toMatchObject({ key: "k", metaKey: true, ctrlKey: false });
    });

    it("parses multi-modifier combo", () => {
      const parsed = kbd.parseKeyCombo("⌘ + ⇧ + ArrowUp");
      expect(parsed).toMatchObject({
        key: "ArrowUp",
        metaKey: true,
        shiftKey: true,
        ctrlKey: false,
        altKey: false,
      });
    });

    it("preserves literal + key", () => {
      const parsed = kbd.parseKeyCombo("⌘ + '+'");
      expect(parsed.key).toBe("+");
      expect(parsed.metaKey).toBe(true);
    });

    it("parses ⌃ as ctrlKey", () => {
      const parsed = kbd.parseKeyCombo("⌃ + c");
      expect(parsed.ctrlKey).toBe(true);
      expect(parsed.metaKey).toBe(false);
    });

    it("parses ⌥ as altKey", () => {
      const parsed = kbd.parseKeyCombo("⌥ + Tab");
      expect(parsed.altKey).toBe(true);
    });
  });

  describe("matchKeyCombo", () => {
    it("matches ⌘ + k", () => {
      const parsed = kbd.parseKeyCombo("⌘ + k");
      expect(kbd.matchKeyCombo(keyEvent("k", { metaKey: true }), parsed)).toBe(true);
    });

    it("rejects wrong key", () => {
      const parsed = kbd.parseKeyCombo("⌘ + k");
      expect(kbd.matchKeyCombo(keyEvent("j", { metaKey: true }), parsed)).toBe(false);
    });

    it("rejects missing meta", () => {
      const parsed = kbd.parseKeyCombo("⌘ + k");
      expect(kbd.matchKeyCombo(keyEvent("k"), parsed)).toBe(false);
    });

    it("checks shift explicitly for alpha keys", () => {
      const parsed = kbd.parseKeyCombo("⌘ + k");
      // Shift not in combo → event with shift should fail
      expect(kbd.matchKeyCombo(keyEvent("k", { metaKey: true, shiftKey: true }), parsed)).toBe(
        false,
      );
    });

    it("checks shift explicitly for arrow keys", () => {
      const withShift = kbd.parseKeyCombo("⇧ + ArrowUp");
      const withoutShift = kbd.parseKeyCombo("ArrowUp");
      expect(kbd.matchKeyCombo(keyEvent("ArrowUp", { shiftKey: true }), withShift)).toBe(true);
      expect(kbd.matchKeyCombo(keyEvent("ArrowUp", { shiftKey: true }), withoutShift)).toBe(false);
    });

    it("is case-insensitive on key", () => {
      const parsed = kbd.parseKeyCombo("⌘ + k");
      expect(kbd.matchKeyCombo(keyEvent("K", { metaKey: true }), parsed)).toBe(true);
    });
  });

  describe("formatKeyCombo", () => {
    it("formats Mac-style with symbols", () => {
      expect(kbd.formatKeyCombo("⌘ + ⇧ + k")).toBe("⇧⌘k");
    });

    it("formats single modifier", () => {
      expect(kbd.formatKeyCombo("⌘ + k")).toBe("⌘k");
    });
  });
});

// ── Non-Mac tests ───────────────────────────────────────────────────────

describe("useKbd (non-Mac)", () => {
  beforeEach(() => setupKbd("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"));
  afterEach(() => app.unmount());

  describe("parseKeyCombo", () => {
    it("swaps ⌘ to ctrlKey on non-Mac", () => {
      const parsed = kbd.parseKeyCombo("⌘ + k");
      expect(parsed.ctrlKey).toBe(true);
      expect(parsed.metaKey).toBe(false);
    });
  });

  describe("matchKeyCombo", () => {
    it("matches ⌘ + k as Ctrl+K on non-Mac", () => {
      const parsed = kbd.parseKeyCombo("⌘ + k");
      expect(kbd.matchKeyCombo(keyEvent("k", { ctrlKey: true }), parsed)).toBe(true);
    });

    it("rejects metaKey on non-Mac", () => {
      const parsed = kbd.parseKeyCombo("⌘ + k");
      expect(kbd.matchKeyCombo(keyEvent("k", { metaKey: true }), parsed)).toBe(false);
    });
  });

  describe("formatKeyCombo", () => {
    it("formats non-Mac style with text labels", () => {
      expect(kbd.formatKeyCombo("⌘ + k")).toBe("Ctrl + k");
    });

    it("includes Shift when present", () => {
      expect(kbd.formatKeyCombo("⌘ + ⇧ + k")).toBe("Ctrl + Shift + k");
    });
  });
});
