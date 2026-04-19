<script lang="ts">
import type { MarkID } from "@waypoints/domain";
import type { DropdownMenuItem } from "@nuxt/ui";

export interface BlockContextMenuProps {
  blockId: MarkID;
}
</script>

<script setup lang="ts">
import {
  ContentType,
  ContentConverters,
  contentData,
  isContent,
  type ContentKind,
} from "@waypoints/domain";
import { BlockOrchestratorKey, type OrchestratorContext } from "../composables";
import ipc from "~/ipc";

const props = defineProps<BlockContextMenuProps>();
const open = defineModel<boolean>("open", { default: false });

const orchestrator = inject<OrchestratorContext>(BlockOrchestratorKey);

const block = computed(() => orchestrator?.blocks.value.find((b) => b.id === props.blockId));

const currentType = computed(() => {
  const data = block.value?.data;
  if (!data || !isContent(data)) return null;
  return data.type;
});

// ── Convert to submenu ──────────────────────────────────────────────

interface ConvertOption {
  type: ContentKind;
  label: string;
  icon: string;
}

const convertOptions: ConvertOption[] = [
  { type: ContentType.Paragraph, label: "Text", icon: "i-lucide-type" },
  { type: ContentType.Heading1, label: "Heading 1", icon: "i-lucide-heading-1" },
  { type: ContentType.Heading2, label: "Heading 2", icon: "i-lucide-heading-2" },
  { type: ContentType.Heading3, label: "Heading 3", icon: "i-lucide-heading-3" },
  { type: ContentType.ListItem, label: "Bullet List", icon: "i-lucide-list" },
  { type: ContentType.EnumItem, label: "Numbered List", icon: "i-lucide-list-ordered" },
  { type: ContentType.Blockquote, label: "Quote", icon: "i-lucide-quote" },
  {
    type: ContentType.Callout,
    label: "Callout",
    icon: "i-lucide-message-square-warning",
  },
  { type: ContentType.Divider, label: "Divider", icon: "i-lucide-minus" },
];

function convertTo(type: ContentKind) {
  if (!block.value) return;
  const data = block.value.data;
  const rt = isContent(data) && "rich_text" in data ? data.rich_text : undefined;
  const converter = ContentConverters[type];
  if (converter) {
    orchestrator?.onChangeBlockType(props.blockId, converter(rt));
  }
}

// ── Duplicate ───────────────────────────────────────────────────────

async function duplicate() {
  if (!block.value || !orchestrator) return;
  const parentId = block.value.parent_id ?? orchestrator.landmarkID.value;
  try {
    const result = await ipc.editor.createAfter(
      props.blockId,
      parentId,
      contentData(block.value.data.mark === "content" ? block.value.data : undefined),
      orchestrator.landmarkID.value,
    );
    orchestrator.blocks.value = result.blocks;
    orchestrator.focus.focusBlock(result.new_mark_id, "start");
  } catch (e) {
    console.error("duplicate failed:", e);
  }
}

// ── Menu items ──────────────────────────────────────────────────────

const items = computed<DropdownMenuItem[][]>(() => {
  const actions: DropdownMenuItem[] = [
    {
      label: "Duplicate",
      icon: "i-lucide-copy",
      kbds: ["meta", "D"],
      onSelect() {
        duplicate();
      },
    },
  ];

  const convertChildren: DropdownMenuItem[] = convertOptions
    .filter((opt) => opt.type !== currentType.value)
    .map((opt) => ({
      label: opt.label,
      icon: opt.icon,
      onSelect() {
        convertTo(opt.type);
      },
    }));

  const convertGroup: DropdownMenuItem[] = [
    {
      label: "Convert to",
      icon: "i-lucide-repeat",
      children: convertChildren,
    },
  ];

  const destructive: DropdownMenuItem[] = [
    {
      label: "Delete",
      icon: "i-lucide-trash-2",
      color: "error" as const,
      kbds: ["backspace"],
      onSelect() {
        orchestrator?.onDeleteBlocks([props.blockId]);
      },
    },
  ];

  return [actions, convertGroup, destructive];
});

defineShortcuts(computed(() => extractShortcuts(items.value)));
</script>

<template>
  <UDropdownMenu
    v-model:open="open"
    :items="items"
    :content="{ align: 'center', side: 'left' }"
    size="sm"
  >
    <slot />
  </UDropdownMenu>
</template>
