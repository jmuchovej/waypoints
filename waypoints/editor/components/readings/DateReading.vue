<script lang="ts">
import type { Reading, Feature } from "@waypoints/domain";

export interface DateReadingProps {
  value: Reading | undefined;
  feature: Feature;
}

export interface DateReadingEmits {
  update: [reading: Reading];
}
</script>
<script setup lang="ts">
import { useTemplateRef } from "vue";
import type { DateGranularity } from "@waypoints/domain";
import { CalendarDateTime, parseDateTime } from "@internationalized/date";
import type { DateValue } from "@internationalized/date";
import { UInputDate, UPopover, UButton, LazyUCalendar } from "#components";

const props = defineProps<DateReadingProps>();
const emit = defineEmits<DateReadingEmits>();

// Reading::Date stores NaiveDateTime, which serializes as ISO 8601 without
// a timezone (e.g. "2026-04-06T15:30:00"). We round-trip via CalendarDateTime
// from @internationalized/date.
//
// TODO (future): support relative/natural date entry — typing "today",
// "@today", "tomorrow", "next friday", etc. should parse to a concrete
// CalendarDateTime. This is a relative-date-parsing problem and should
// probably reach for a library like chrono-node or similar. Relevant
// surface: the segmented input itself, possibly via a custom input mode.

const inputDate = useTemplateRef("inputDate");
const popoverOpen = ref(false);

/**
 * Schema-level precision for the date/time value. Comes from the chart's
 * `Date` feature definition and is mirrored onto `UInputDate`'s `granularity`
 * prop so the editor only exposes the intended fields.
 *
 * Our `DateGranularity` variants (`day`/`hour`/`minute`/`second`) are
 * deliberately the same strings Nuxt UI / @internationalized/date use, so
 * this is a direct pass-through.
 */
const granularity = computed<DateGranularity>(() => {
  if (props.feature.kind.type === "date") {
    return props.feature.kind.granularity ?? "day";
  }
  return "day";
});

const dateValue = computed<CalendarDateTime | null>(() => {
  if (props.value?.type !== "date") return null;
  try {
    return parseDateTime(props.value.value);
  } catch {
    return null;
  }
});

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function onUpdate(val: CalendarDateTime | null) {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    if (!val) {
      emit("update", { type: "null" });
      return;
    }
    emit("update", { type: "date", value: val.toString() });
  }, 250);
}

/**
 * Called when the user picks a day in the calendar popover. We merge the
 * picked y/m/d into any existing time-of-day so time-granular fields
 * (e.g. Meeting's "Date & Time") don't lose their time when the user
 * nudges the date. If there's no existing value, we start at 00:00:00.
 */
function onCalendarPick(picked: DateValue) {
  const current = dateValue.value;
  const next = current
    ? current.set({
        year: picked.year,
        month: picked.month,
        day: picked.day,
      })
    : new CalendarDateTime(picked.year, picked.month, picked.day);

  onUpdate(next);
  popoverOpen.value = false;
}
</script>

<template>
  <UInputDate
    ref="inputDate"
    :model-value="dateValue"
    :granularity="granularity"
    variant="ghost"
    size="md"
    @update:model-value="onUpdate"
  >
    <template #leading>
      <UPopover v-model:open="popoverOpen" :reference="inputDate?.inputsRef[3]?.$el">
        <UButton
          color="neutral"
          variant="link"
          icon="i-lucide-calendar"
          aria-label="Select a date"
          class="px-0"
        />

        <template #content>
          <LazyUCalendar
            :model-value="dateValue"
            class="p-2"
            @update:model-value="onCalendarPick"
          />
        </template>
      </UPopover>
    </template>
  </UInputDate>
</template>
