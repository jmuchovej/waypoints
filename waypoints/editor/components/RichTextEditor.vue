<script lang="ts">
import type { RichText, MarkID } from "@waypoints/domain";

export interface RichTextEditorProps {
  /** When provided, enables focus-to-edit via the orchestrator. Omit for static-only rendering. */
  blockId?: MarkID;
  richText: RichText;
  placeholder?: string;
  /** When provided without blockId, the editor is always active (standalone mode). */
  onUpdate?: (rt: RichText) => void;
  /** Mark names to exclude from the toolbar (e.g. ["bold"]) */
  exclude?: string[];
  /** Intercept keydown before TipTap. Return true to prevent default handling. */
  onKeyDown?: (e: KeyboardEvent) => boolean;
}

export interface RichTextEditorEmits {
  click: [e: MouseEvent];
  mousedown: [e: MouseEvent];
}
</script>

<script setup lang="ts">
import { emojis } from "@tiptap/extension-emoji";
import { useInlineEditor, BlockOrchestratorKey, type OrchestratorContext } from "../composables";
import { inlineToolbarItems } from "../tiptap/toolbar-items";
import { UEditorToolbar, UEditorEmojiMenu } from "#components";
import TextSegment from "./TextSegment.vue";
import ReadonlyRichText from "./RichText.vue";

const props = defineProps<RichTextEditorProps>();
const emit = defineEmits<RichTextEditorEmits>();

function onClick(e: MouseEvent) {
  emit("click", e);
}

const orchestrator = inject<OrchestratorContext>(BlockOrchestratorKey, {});
const { focus } = orchestrator;

const isGroupActive = inject<Ref<boolean>>("group-active", ref(false));

const isStandalone = computed(() => !props.blockId && !!props.onUpdate);

const editing = computed(() => {
  if (isStandalone.value) return true;
  if (!props.blockId || !orchestrator) return false;
  return focus?.isFocused(props.blockId) || isGroupActive.value;
});

const editorEl = ref<HTMLElement | null>(null);
const richTextRef = computed(() => props.richText);

function onEditorUpdate(rt: RichText): void | Promise<void> {
  if (props.blockId) {
    return orchestrator?.onUpdateBlockText(props.blockId, rt);
  }
  props.onUpdate?.(rt);
}

const {
  editor: editorRef,
  handle,
  createEditor,
  destroyEditor,
} = useInlineEditor({
  richText: richTextRef as Ref<ConverterRichText>,
  blockID: props.blockId,
  placeholder: props.placeholder,
  onUpdate: onEditorUpdate,
  onKeyDown: props.onKeyDown,
});

function mountEditor() {
  if (!editorEl.value || editorRef.value) return;
  createEditor(editorEl.value);
  if (props.blockId) {
    // registerEditor executes any pending focus intent (position or coords)
    focus?.registerEditor(props.blockId, handle);
  }
}

onMounted(() => {
  if (editing.value) mountEditor();
});

// Orchestrator mode: mount/unmount as focus changes.
watch(editing, (isEditing, wasEditing) => {
  if (isStandalone.value) return;
  if (isEditing && !wasEditing) {
    nextTick(mountEditor);
  } else if (!isEditing && wasEditing) {
    if (props.blockId) focus?.unregisterEditor(props.blockId);
    destroyEditor();
  }
});

onBeforeUnmount(() => {
  if (props.blockId) focus?.unregisterEditor(props.blockId);
  destroyEditor();
});

const toolbarItems = computed(() => {
  if (!props.exclude?.length) return [inlineToolbarItems];
  return [
    inlineToolbarItems.filter((item) => !("mark" in item && props.exclude!.includes(item.mark))),
  ];
});

function onMousedown(e: MouseEvent) {
  emit("mousedown", e);

  if (editing.value || !props.blockId) return;
  // No preventDefault — let the browser show native text selection during drag.
  // On mouseup, the editor mounts and ProseMirror's selection replaces it.

  const blockId = props.blockId;

  // Measure 1ch from the target element's font for drag threshold.
  const startX = e.clientX;
  const startY = e.clientY;
  const ctx = document.createElement("canvas").getContext("2d");
  const targetEl = e.target as HTMLElement;
  const chPx = ctx ? ((ctx.font = getComputedStyle(targetEl).font), ctx.measureText("0").width) : 8;

  let endX = startX;
  let endY = startY;

  function onMove(ev: MouseEvent) {
    endX = ev.clientX;
    endY = ev.clientY;
  }

  function onUp() {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);

    const dx = endX - startX;
    const dy = endY - startY;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (dist >= chPx) {
      focus?.focusBlockWithRange(blockId, startX, startY, endX, endY);
    } else {
      focus?.focusBlockAt(blockId, { clientX: startX, clientY: startY } as MouseEvent);
    }
  }

  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
}
</script>

<template>
  <span class="block cursor-text" @click="onClick" @mousedown="onMousedown">
    <span v-if="editing" ref="editorEl" />
    <ReadonlyRichText v-else :rich-text="richText" :placeholder="placeholder" />
    <div v-if="editorRef" class="contents">
      <UEditorToolbar
        :editor="editorRef"
        layout="bubble"
        :items="toolbarItems"
        :update-delay="0"
        size="xs"
      />
      <UEditorEmojiMenu :editor="editorRef" :items="emojis" :limit="50" />
    </div>
  </span>
</template>
