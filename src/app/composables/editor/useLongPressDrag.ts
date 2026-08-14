import { onLongPress, useMediaQuery } from "@vueuse/core";
import { type Ref, ref, readonly, watchEffect } from "vue";

import type { DragDropContext } from "./useBlockDragDrop";

export function useLongPressDrag(containerEl: Ref<HTMLElement | null>, dragDrop: DragDropContext) {
  const hasTouchInput = useMediaQuery("(any-pointer: coarse)");
  const pressing = ref(false);
  const pressMarkId = ref<string | null>(null);

  onLongPress(
    containerEl,
    (e) => {
      if (!hasTouchInput.value) return;

      const markEl = (e.target as HTMLElement).closest<HTMLElement>(".mark[data-mark-id]");
      if (!markEl) return;

      // Don't fight text editing
      if ((e.target as HTMLElement).closest('.ProseMirror, [contenteditable="true"]')) return;

      const markId = markEl.getAttribute("data-mark-id");
      if (!markId) return;

      // Suppress context menu only when drag actually activates (not on every pointerdown).
      // modifiers.prevent would call preventDefault on every pointerdown, which blocks
      // mousedown from reaching ProseMirror and prevents caret repositioning on click.
      e.preventDefault();

      pressing.value = false;
      pressMarkId.value = null;
      dragDrop.onPointerDown([markId], e);
    },
    {
      delay: 500,
      distanceThreshold: 10,
    },
  );

  // --- Visual feedback (thin layer on top) ---
  // onLongPress doesn't expose "pressing" state, so we add a
  // pointerdown → 100ms delay → set pressing=true pattern.
  // Cleanup on move/up/cancel mirrors onLongPress's own behavior.

  watchEffect((onCleanup) => {
    const el = containerEl.value;
    if (!el) return;

    let feedbackTimer: ReturnType<typeof setTimeout> | null = null;

    function startFeedback(e: PointerEvent) {
      if (!hasTouchInput.value) return;
      const markEl = (e.target as HTMLElement).closest<HTMLElement>(".mark[data-mark-id]");
      if (!markEl) return;
      if ((e.target as HTMLElement).closest('.ProseMirror, [contenteditable="true"]')) return;

      pressMarkId.value = markEl.getAttribute("data-mark-id");
      feedbackTimer = setTimeout(() => {
        pressing.value = true;
      }, 100);
    }

    function cancelFeedback() {
      if (feedbackTimer) {
        clearTimeout(feedbackTimer);
        feedbackTimer = null;
      }
      pressing.value = false;
      pressMarkId.value = null;
    }

    el.addEventListener("pointerdown", startFeedback, { passive: true });
    el.addEventListener("pointerup", cancelFeedback, { passive: true });
    el.addEventListener("pointermove", cancelFeedback, { passive: true });
    el.addEventListener("pointercancel", cancelFeedback, { passive: true });

    onCleanup(() => {
      el.removeEventListener("pointerdown", startFeedback);
      el.removeEventListener("pointerup", cancelFeedback);
      el.removeEventListener("pointermove", cancelFeedback);
      el.removeEventListener("pointercancel", cancelFeedback);
      cancelFeedback();
    });
  });

  return { pressing: readonly(pressing), pressMarkId: readonly(pressMarkId) };
}
