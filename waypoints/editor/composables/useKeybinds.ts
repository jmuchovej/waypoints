import { computed } from "vue";
import { createSharedComposable } from "@vueuse/core";

export interface ParsedKeyCombo {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

const MODIFIER_MAP: Record<string, keyof Omit<ParsedKeyCombo, "key">> = {
  "⌘": "metaKey",
  "⇧": "shiftKey",
  "⌃": "ctrlKey",
  "⌥": "altKey",
};

const PLUS_PLACEHOLDER = "\0PLUS";

/** Keys where shiftKey should always be matched explicitly. */
const SHIFT_EXPLICIT_KEYS = new Set([
  "arrowup",
  "arrowdown",
  "arrowleft",
  "arrowright",
  "tab",
  "enter",
  "escape",
  "backspace",
  "delete",
  "home",
  "end",
  "pageup",
  "pagedown",
]);

function isAlpha(key: string): boolean {
  return key.length === 1 && /[a-z]/i.test(key);
}

export const useKbd = createSharedComposable(() => {
  const macOS = computed(() => typeof navigator !== "undefined" && /Mac/.test(navigator.userAgent));

  function parseKeyCombo(combo: string): ParsedKeyCombo {
    // Preserve literal '+' key
    const sanitized = combo.replace(/'\+'/g, PLUS_PLACEHOLDER);
    const tokens = sanitized.split(" + ");

    const parsed: ParsedKeyCombo = {
      key: "",
      metaKey: false,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
    };

    for (const token of tokens) {
      const trimmed = token.trim();
      const modFlag = MODIFIER_MAP[trimmed];
      if (modFlag) {
        parsed[modFlag] = true;
      } else {
        parsed.key = trimmed.replace(PLUS_PLACEHOLDER, "+");
      }
    }

    // Platform adjustment: ⌘ maps to metaKey on Mac, ctrlKey elsewhere
    if (parsed.metaKey && !macOS.value) {
      parsed.metaKey = false;
      parsed.ctrlKey = true;
    }

    return parsed;
  }

  function matchKeyCombo(e: KeyboardEvent, parsed: ParsedKeyCombo): boolean {
    if (e.key.toLowerCase() !== parsed.key.toLowerCase()) return false;

    const isMac = macOS.value;

    // Check meta: on Mac it's metaKey, on non-Mac metaKey is never expected
    if (isMac) {
      if (e.metaKey !== parsed.metaKey) return false;
    } else {
      // On non-Mac, ⌘ was already swapped to ctrlKey in parse, so metaKey should be false
      if (e.metaKey) return false;
    }

    if (e.ctrlKey !== parsed.ctrlKey) return false;
    if (e.altKey !== parsed.altKey) return false;

    // Shift: always check for alphabetic keys, shift-explicit keys, or when other modifiers are present
    const keyLower = parsed.key.toLowerCase();
    if (
      isAlpha(keyLower) ||
      SHIFT_EXPLICIT_KEYS.has(keyLower) ||
      parsed.metaKey ||
      parsed.ctrlKey
    ) {
      if (e.shiftKey !== parsed.shiftKey) return false;
    }

    return true;
  }

  function formatKeyCombo(combo: string): string {
    const parsed = parseKeyCombo(combo);
    const parts: string[] = [];

    if (macOS.value) {
      if (parsed.ctrlKey) parts.push("⌃");
      if (parsed.altKey) parts.push("⌥");
      if (parsed.shiftKey) parts.push("⇧");
      if (parsed.metaKey) parts.push("⌘");
    } else {
      if (parsed.ctrlKey || parsed.metaKey) parts.push("Ctrl");
      if (parsed.altKey) parts.push("Alt");
      if (parsed.shiftKey) parts.push("Shift");
    }

    parts.push(parsed.key);
    return parts.join(macOS.value ? "" : " + ");
  }

  return { macOS, parseKeyCombo, matchKeyCombo, formatKeyCombo };
});
