<script setup lang="ts">
  // The component this shadow replaces — everything that isn't a mermaid fence
  // is delegated to it unchanged.
  import UProsePre from "@nuxt/ui/runtime/components/prose/Pre.vue";
  import { renderMermaidSVG } from "beautiful-mermaid";

  // Prop set mirrors @nuxt/ui's ProsePre so delegation is lossless.
  const props = defineProps<{
    icon?: string;
    code?: string;
    language?: string;
    filename?: string;
    highlights?: number[];
    hideHeader?: boolean;
    meta?: string;
    class?: unknown;
    ui?: Record<string, string>;
  }>();

  /**
   * Rendered inline (SSR-safe — renderMermaidSVG is synchronous, no DOM).
   * @nuxt/ui theme variables keep the diagram tracking light/dark mode.
   * Null on parse failure, falling through to the plain code block so broken
   * diagram source stays visible instead of vanishing.
   */
  const mermaidSvg = computed(() => {
    if (props.language !== "mermaid" || !props.code) return null;
    try {
      return renderMermaidSVG(props.code, {
        bg: "var(--ui-bg)",
        fg: "var(--ui-text)",
        line: "var(--ui-border-accented)",
        accent: "var(--ui-primary)",
        muted: "var(--ui-text-muted)",
        surface: "var(--ui-surface)",
        transparent: true,
        padding: 16,
        font: "Albert Sans",
      });
    } catch {
      return null;
    }
  });
</script>

<template>
  <figure v-if="mermaidSvg" class="mermaid not-prose inline-flex" v-html="mermaidSvg" />
  <UProsePre v-else v-bind="props">
    <slot />
  </UProsePre>
</template>
