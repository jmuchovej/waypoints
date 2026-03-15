<script setup lang="ts">
import { useMathRenderer, type MathEngine } from "../../composables";

const props = defineProps<{
  data: unknown;
}>();

const { render, loading } = useMathRenderer();

const expression = computed(() => {
  if (typeof props.data === "string") return props.data;
  if (
    props.data &&
    typeof props.data === "object" &&
    "expression" in props.data
  ) {
    return (props.data as Record<string, unknown>).expression as string;
  }
  return undefined;
});

const engine = computed<MathEngine>(() => {
  if (props.data && typeof props.data === "object" && "engine" in props.data) {
    const e = (props.data as Record<string, unknown>).engine;
    if (e === "typst") return "typst";
    // Domain uses "latex", renderer uses "katex" (KaTeX is the LaTeX renderer)
    if (e === "latex" || e === "katex") return "katex";
  }
  return "typst";
});

const renderedHtml = ref<string>("");

watch(
  [expression, engine],
  async ([expr, eng]) => {
    if (!expr) {
      renderedHtml.value = "";
      return;
    }
    const result = await render(expr, eng);
    renderedHtml.value = result.html;
  },
  { immediate: true },
);
</script>

<template>
  <span data-inline="math" :title="expression">
    <span v-if="renderedHtml" v-html="renderedHtml" />
    <span v-else-if="loading" class="math-loading"><slot /></span>
    <slot v-else />
  </span>
</template>
