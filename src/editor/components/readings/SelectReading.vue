<script lang="ts">
import type { Reading, Feature } from "@waypoints/domain";
export interface SelectReadingProps {
  value: Reading | undefined;
  feature: Feature;
}
export interface SelectReadingEmits {
  update: [reading: Reading];
}
</script>
<script setup lang="ts">
import type { GlyphID, SelectOption, SelectGroup, Color } from "@waypoints/domain";
import { createLogger } from "~/utils/logger";
import { toNuxtIcon } from "~/utils/icon";
import type { InputMenuItem } from "@nuxt/ui";

const log = createLogger("select-reading");

// ── Color → Tailwind ───────────────────────────────────────────────────
//
// Maps the domain `Color` enum onto a small bundle of Tailwind classes
// for the chip background, border, and chip-dot color. We hand these
// to UInputMenu via the per-item `ui` field so each option carries its
// own visual identity in both the dropdown row and (in multi mode) the
// rendered tag chip.

interface ColorClasses {
  /** Classes applied to the tag wrapper (chip background + border) */
  tag: string;
  /** Tailwind color name for the leading chip dot in the dropdown row */
  chipDot: "primary" | "neutral" | "info" | "success" | "warning" | "error";
}

const COLOR_CLASSES: Record<Color, ColorClasses> = {
  Default: { tag: "bg-elevated text-default ring-default", chipDot: "neutral" },
  Gray: {
    tag: "bg-neutral-100 text-neutral-700 ring-neutral-300 dark:bg-neutral-800 dark:text-neutral-200 dark:ring-neutral-700",
    chipDot: "neutral",
  },
  Brown: {
    tag: "bg-amber-100 text-amber-900 ring-amber-300 dark:bg-amber-950 dark:text-amber-200 dark:ring-amber-800",
    chipDot: "warning",
  },
  Orange: {
    tag: "bg-orange-100 text-orange-800 ring-orange-300 dark:bg-orange-950 dark:text-orange-200 dark:ring-orange-800",
    chipDot: "warning",
  },
  Yellow: {
    tag: "bg-yellow-100 text-yellow-800 ring-yellow-300 dark:bg-yellow-950 dark:text-yellow-200 dark:ring-yellow-800",
    chipDot: "warning",
  },
  Green: {
    tag: "bg-green-100 text-green-800 ring-green-300 dark:bg-green-950 dark:text-green-200 dark:ring-green-800",
    chipDot: "success",
  },
  Blue: {
    tag: "bg-blue-100 text-blue-800 ring-blue-300 dark:bg-blue-950 dark:text-blue-200 dark:ring-blue-800",
    chipDot: "info",
  },
  Purple: {
    tag: "bg-purple-100 text-purple-800 ring-purple-300 dark:bg-purple-950 dark:text-purple-200 dark:ring-purple-800",
    chipDot: "primary",
  },
  Pink: {
    tag: "bg-pink-100 text-pink-800 ring-pink-300 dark:bg-pink-950 dark:text-pink-200 dark:ring-pink-800",
    chipDot: "primary",
  },
  Red: {
    tag: "bg-red-100 text-red-800 ring-red-300 dark:bg-red-950 dark:text-red-200 dark:ring-red-800",
    chipDot: "error",
  },
};

function classesFor(color: Color | undefined): ColorClasses {
  return COLOR_CLASSES[color ?? "Default"] ?? COLOR_CLASSES.Default;
}

const props = defineProps<SelectReadingProps>();
const emit = defineEmits<SelectReadingEmits>();

// ── Kind introspection ─────────────────────────────────────────────────
// Mirrors the Rust SelectBuilder logic: one component handles all four
// kinds (Select, MultiSelect, GroupedSelect, GroupedMultiSelect) by reading
// flags off the feature.kind discriminator.

const kind = computed(() => props.feature.kind);

const isMulti = computed(() => {
  const k = kind.value.type;
  return ["multi-select", "grouped-multi-select"].includes(k);
});

const isGrouped = computed(() => {
  const k = kind.value.type;
  return ["grouped-select", "grouped-multi-select"].includes(k);
});

// ── Items: a flat list (with separators for groups) ───────────────────
//
// UInputMenu accepts either an array-of-arrays for groups OR a flat array
// containing `{ type: "label", label }` and `{ type: "separator" }` items.
// We use the flat-with-separators approach so option lookup is uniform.

/**
 * An InputMenu item that carries our domain identifiers alongside the
 * standard Nuxt UI fields. The `id` is the GlyphID of the option (the
 * value persisted in `Reading`); `groupId` is set for grouped kinds.
 */
interface OptionItem extends InputMenuItem {
  id: GlyphID;
  groupId?: GlyphID;
  label: string;
}

function isOption(item: InputMenuItem): item is OptionItem {
  return "id" in item;
}

function optionToItem(id: GlyphID, opt: SelectOption, groupId?: GlyphID): OptionItem {
  const colors = classesFor(opt.color);
  return {
    id,
    groupId,
    label: opt.name,
    icon: toNuxtIcon(opt.icon?.identifier),
    description: opt.description ?? undefined,
    // A leading colored dot in the dropdown row, matching the option's color.
    chip: { color: colors.chipDot, size: "md" },
    // Per-item style overrides — InputMenu applies these to the tag chip
    // (multi-select mode) so each tag carries its own color.
    ui: {
      tagsItem: `${colors.tag} ring-1 ring-inset rounded-md px-1.5 py-0.5`,
      tagsItemText: "text-xs font-medium",
    },
  };
}

const items = computed<InputMenuItem[]>(() => {
  const k = kind.value;
  let result: InputMenuItem[] = [];

  if (k.type === "select" || k.type === "multi-select") {
    result = Object.entries(k.options ?? {})
      .filter(([, opt]) => opt != null)
      .map(([id, opt]) => optionToItem(id as GlyphID, opt as SelectOption));
  } else if (k.type === "grouped-select" || k.type === "grouped-multi-select") {
    let first = true;
    for (const [groupId, g] of Object.entries(k.groups ?? {})) {
      if (g == null) continue;
      const group = g as SelectGroup;
      if (!first) result.push({ type: "separator" });
      result.push({ type: "label", label: group.name });
      for (const [optId, opt] of Object.entries(group.options ?? {})) {
        if (opt == null) continue;
        result.push(
          optionToItem(optId as GlyphID, opt as SelectOption, groupId as GlyphID),
        );
      }
      first = false;
    }
  }

  log.debug(`items (${props.feature.name}) recomputed`, {
    kind: k.type,
    count: result.length,
    items: result,
  });
  return result;
});

const allOptions = computed<OptionItem[]>(() => {
  const opts = items.value.filter(isOption);
  log.debug(`allOptions (${props.feature.name})`, {
    count: opts.length,
    ids: opts.map((o) => o.id),
  });
  return opts;
});

// ── Selected state ─────────────────────────────────────────────────────
//
// We model the selection as full OptionItem object(s) — not raw GlyphIDs —
// so that UInputMenu can render their `label` directly without needing to
// resolve a `value-key` lookup (which is unreliable with separator items).

const selectedIds = computed<GlyphID[]>(() => {
  const v = props.value;
  let ids: GlyphID[] = [];
  if (v) {
    if (v.type === "select") ids = [v.value];
    else if (v.type === "multi-select") ids = v.value;
    else if (v.type === "grouped-select")
      ids = Object.values(v.value).flat() as GlyphID[];
  }
  log.debug(`selectedIds (${props.feature.name})`, { rawValue: v, ids });
  return ids;
});

const selectedItems = computed<OptionItem[]>(() => {
  const items = allOptions.value.filter((i) => selectedIds.value.includes(i.id));
  log.debug(`selectedItems (${props.feature.name})`, {
    wantedIds: selectedIds.value,
    availableIds: allOptions.value.map((o) => o.id),
    matchedCount: items.length,
    items,
  });
  return items;
});

const modelValue = computed(() => {
  const mv = isMulti.value ? selectedItems.value : (selectedItems.value[0] ?? null);
  log.debug(`modelValue (${props.feature.name})`, {
    isMulti: isMulti.value,
    mv,
  });
  return mv;
});

/**
 * The leading icon for the input itself. For single-select, this is the
 * icon of the currently selected option (if any); for multi-select, we
 * leave it undefined since there's no one-icon-fits-all. Bound to the
 * `:icon` prop on UInputMenu so the visible input chrome reflects the
 * selected state at a glance.
 */
const selectedIcon = computed<string | undefined>(() => {
  if (isMulti.value) return undefined;
  return selectedItems.value[0]?.icon as string | undefined;
});

// ── Selection handling ─────────────────────────────────────────────────

function onUpdate(val: OptionItem | OptionItem[] | null) {
  const k = kind.value;
  log.info(`onUpdate (${props.feature.name}) fired`, { kindType: k.type, val });

  if (val == null || (Array.isArray(val) && val.length === 0)) {
    log.info(`onUpdate → emitting null`);
    emit("update", { type: "null" });
    return;
  }

  let reading: Reading | null = null;
  switch (k.type) {
    case "select":
      reading = { type: "select", value: (val as OptionItem).id };
      break;
    case "multi-select":
      reading = {
        type: "multi-select",
        value: (val as OptionItem[]).map((i) => i.id),
      };
      break;
    case "grouped-select": {
      const item = val as OptionItem;
      if (item.groupId) {
        reading = {
          type: "grouped-select",
          value: { [item.groupId]: [item.id] } as any,
        };
      } else {
        log.warn(`grouped-select: selected item has no groupId`, { item });
      }
      break;
    }
    case "grouped-multi-select": {
      const grouped: Record<string, GlyphID[]> = {};
      for (const item of val as OptionItem[]) {
        if (!item.groupId) continue;
        if (!grouped[item.groupId]) grouped[item.groupId] = [];
        grouped[item.groupId]!.push(item.id);
      }
      reading = { type: "grouped-select", value: grouped as any };
      break;
    }
  }

  if (reading) {
    log.info(`onUpdate → emitting Reading`, reading);
    emit("update", reading);
  } else {
    log.warn(`onUpdate → no reading constructed, skipping emit`);
  }
}
</script>

<template>
  <!--
    NOTE: deliberately NOT passing `autocomplete` — when true, InputMenu
    behaves as a typeahead and can lose track of the selected item when the
    v-model is a full object. Default (false) renders as a classic select.
  -->
  <UInputMenu
    :model-value="modelValue"
    :items="items"
    :multiple="isMulti"
    :icon="selectedIcon"
    variant="ghost"
    label-key="label"
    placeholder="Select…"
    delete-icon="i-lucide-trash"
    clear-icon="i-lucide-trash"
    open-on-click
    @update:model-value="onUpdate"
  />
</template>
