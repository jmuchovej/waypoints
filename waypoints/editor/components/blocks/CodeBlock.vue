<script setup lang="ts">
import type { ContentNode } from "~~/editor";
import { useCodeHighlighter } from "../../composables/useCodeHighlighter";
import languages from "~/assets/languages.json";

const props = defineProps<{ block: ContentNode<"code"> }>();

const code = computed(() => props.block.data.lines.join("\n"));
const language = computed(() => props.block.data.language);

const langInfo = computed(() => {
  const lang = language.value;
  if (!lang) return { name: "Plain Text", icon: "i-lucide-file-code" };
  const entry = (languages as Record<string, { name: string; icon: string }>)[lang];
  return entry ?? { name: lang, icon: `i-vscode-icons-file-type-${lang}` };
});

const { html, loading } = useCodeHighlighter(code, language);

const copied = ref(false);

async function copyCode() {
  try {
    await navigator.clipboard.writeText(code.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 2000);
  } catch {
    // Clipboard API not available
  }
}
</script>

<template>
  <div class="code-block">
    <div class="code-header">
      <UIcon :name="langInfo.icon" class="size-4" />
      <button type="button" class="code-copy" @click.stop="copyCode">
        <UIcon :name="copied ? 'i-lucide-check' : 'i-lucide-copy'" class="size-3.5" />
      </button>
    </div>
    <div v-if="loading" class="code-loading">
      <pre><code>{{ code }}</code></pre>
    </div>
    <div v-else class="code-body" v-html="html" />
  </div>
</template>
