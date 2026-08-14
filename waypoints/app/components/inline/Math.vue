<script setup lang="ts">
  import { useMathRenderer, type MathEngine } from "#imports";

  const props = defineProps<{
    data: unknown;
  }>();

  const { render, loading } = useMathRenderer();

  const expression = computed(() => {
    if (typeof props.data === "string") return props.data;
    if (props.data && typeof props.data === "object" && "expression" in props.data) {
      return (props.data as Record<string, unknown>).expression as string;
    }
    return undefined;
  });

  const engine = computed<MathEngine>(() => {
    if (props.data && typeof props.data === "object" && "engine" in props.data) {
      const e = (props.data as Record<string, unknown>).engine;
      if (e === "typst") return "typst";
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
  <span data-inline="math" :title="expression" class="math-inline">
    <span v-if="renderedHtml" class="math-visual" aria-hidden="true" v-html="renderedHtml" />
    <span class="math-text" :class="{ 'math-text--rendered': !!renderedHtml }">
      <slot />
    </span>
  </span>
</template>
