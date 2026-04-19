<script setup lang="ts">
import * as z from "zod";
import type { FormSubmitEvent } from "@nuxt/ui";
import { open } from "@tauri-apps/plugin-dialog";
import ipc from "~/ipc";

const emit = defineEmits<{
  success: [];
}>();

const locationType = ref<"managed" | "custom" | "import" | null>(null);
const selectedPath = ref<string | null>(null);

const nameSchema = z.object({
  name: z.string().min(1, "Name is required").max(64, "Name must be 64 characters or fewer"),
});

type NameSchema = z.output<typeof nameSchema>;

const form = reactive<Partial<NameSchema>>({ name: "" });

const nameEntered = computed(() => (form.name?.length ?? 0) > 0);

const showLocationChoice = computed(() => locationType.value !== "import" && nameEntered.value);

const canSubmit = computed(() => {
  if (!nameEntered.value) return false;
  if (locationType.value === "managed") return true;
  if ((locationType.value === "custom" || locationType.value === "import") && selectedPath.value)
    return true;
  return false;
});

async function pickFolder() {
  const path = await open({ directory: true, multiple: false });
  if (path) {
    selectedPath.value = path as string;
  }
}

function selectLocation(type: "managed" | "custom") {
  locationType.value = type;
  selectedPath.value = null;
  if (type === "custom") {
    pickFolder();
  }
}

async function importExisting() {
  locationType.value = "import";
  selectedPath.value = null;
  await pickFolder();
}

async function onSubmit(event: FormSubmitEvent<NameSchema>) {
  try {
    if (locationType.value === "managed") {
      await ipc.territory.create(event.data.name);
    } else {
      await ipc.territory.open(null, selectedPath.value, event.data.name);
    }
    emit("success");
  } catch (e) {
    // TODO: surface error to user
    console.error("Territory creation failed", e);
  }
}

const uiPickerButton = {
  base: "flex-1 flex flex-col items-center justify-center gap-3 p-6",
  leadingIcon: "size-8",
  label: "text-highlighted",
};
</script>

<template>
  <div class="flex flex-col items-center gap-6 w-full max-w-md">
    <p class="text-sm text-muted text-center">
      Create a <span class="text-primary">new territory</span> to get started.
    </p>

    <!-- Name input (always visible) -->
    <UForm :schema="nameSchema" :state="form" class="w-full flex flex-col gap-4" @submit="onSubmit">
      <UFormField name="name" label="What should we call this?">
        <UInput v-model="form.name" placeholder="My Territory" autofocus class="w-full" />
      </UFormField>

      <!-- Location choice (revealed once name is entered) -->
      <template v-if="showLocationChoice">
        <UFormField label="Where should we store it?">
          <div class="flex gap-4 w-full">
            <UButton
              variant="outline"
              size="xl"
              icon="i-lucide-hard-drive"
              label="Managed by Waypoints"
              :ui="uiPickerButton"
              :class="{ 'ring-2 ring-primary': locationType === 'managed' }"
              @click="selectLocation('managed')"
            />

            <UButton
              variant="subtle"
              size="xl"
              icon="i-lucide-folder-open"
              label="Custom Folder"
              :ui="uiPickerButton"
              :class="{
                'ring-2 ring-primary': locationType === 'custom' && selectedPath,
              }"
              @click="selectLocation('custom')"
            />
          </div>
        </UFormField>
      </template>

      <!-- Selected path display (custom / import) -->
      <p v-if="selectedPath" class="text-sm text-muted truncate max-w-full w-full text-center">
        {{ selectedPath }}
      </p>

      <UButton
        v-if="canSubmit"
        type="submit"
        class="w-full justify-center"
        :label="locationType === 'import' ? 'Open' : 'Create'"
      />
    </UForm>

    <USeparator label="OR" />

    <!-- Import escape hatch -->
    <UButton
      variant="subtle"
      color="neutral"
      size="sm"
      class="w-full justify-center"
      @click="importExisting"
    >
      Already have a Waypoints Territory?
      <span class="text-primary">Import it</span>
    </UButton>
  </div>
</template>
