<script setup lang="ts">
import { ContentType } from "@waypoints/domain";
import type { ContentNode } from "~~/editor";
import { useMathRenderer, type OrchestratorContext } from "../../composables";

const props = defineProps<{ block: ContentNode<"math"> }>();

const orchestrator = inject<OrchestratorContext>("block-orchestrator");
const { render, loading } = useMathRenderer();

const localExpression = ref(props.block.data.expression);
const renderedHtml = ref("");

// Map domain engine to renderer engine
function toRendererEngine(e: string) {
  if (e === "latex") return "katex" as const;
  return "typst" as const;
}

// Sync from external changes
watch(
  () => props.block.data.expression,
  (val) => {
    if (val !== localExpression.value) {
      localExpression.value = val;
    }
  },
);

// Debounced render + persist
let persistTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  localExpression,
  async (expr) => {
    if (!expr) {
      renderedHtml.value = "";
      return;
    }
    const result = await render(expr, toRendererEngine(props.block.data.engine), true);
    renderedHtml.value = result.html;
  },
  { immediate: true },
);

function onInput(e: Event) {
  const val = (e.target as HTMLTextAreaElement).value;
  localExpression.value = val;

  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    orchestrator?.onChangeBlockType(props.block.id, {
      type: ContentType.Math,
      expression: val,
      engine: props.block.data.engine,
    });
  }, 300);
}

function toggleEngine() {
  const newEngine = props.block.data.engine === "typst" ? "latex" : "typst";
  orchestrator?.onChangeBlockType(props.block.id, {
    type: ContentType.Math,
    expression: localExpression.value,
    engine: newEngine,
  });
}
</script>

<template>
  <div class="math-block">
    <div class="math-block-header">
      <span class="math-block-label">Math</span>
      <button class="math-block-engine" @click="toggleEngine">
        {{ block.data.engine === "typst" ? "Typst" : "LaTeX" }}
      </button>
    </div>
    <textarea
      class="math-block-input"
      :value="localExpression"
      placeholder="Enter expression..."
      rows="2"
      spellcheck="false"
      @input="onInput"
    />
    <div v-if="renderedHtml" class="math-block-preview" v-html="renderedHtml" />
    <div v-else-if="loading" class="math-block-preview math-loading">Rendering...</div>
  </div>
</template>

<style scoped>
@reference "~/assets/css/main.css";

.math-block {
  @apply border border-muted rounded-md overflow-hidden;
}

.math-block-header {
  @apply flex items-center justify-between px-3 py-1.5 bg-muted/50 border-b border-muted;
}

.math-block-label {
  @apply text-xs font-medium text-dimmed;
}

.math-block-engine {
  @apply text-xs px-2 py-0.5 rounded-sm bg-elevated text-dimmed cursor-pointer;
  @apply hover:text-default transition-colors;
}

.math-block-input {
  @apply w-full px-3 py-2 font-mono text-sm bg-transparent resize-y outline-none;
  @apply min-h-10;
}

.math-block-preview {
  @apply px-3 py-3 border-t border-muted flex items-center justify-center overflow-x-auto;
}
</style>
