<script setup lang="ts">
  import { ipc, navigateTo, createLogger } from "#imports";

  const logger = createLogger("Onboarding");

  definePageMeta({ layout: "onboarding" });

  type Step = "loading" | "identity" | "territory";

  const step = ref<Step>("loading");

  onMounted(async () => {
    try {
      const status = await ipc.sync.status();
      step.value = status.hasIdentity ? "territory" : "identity";
    } catch (e) {
      // No backend (production browser build) — fall through to the classic
      // territory step so onboarding still works.
      logger.error("Sync status unavailable, skipping identity step", e);
      step.value = "territory";
    }
  });
</script>

<template>
  <div class="flex w-full max-w-lg flex-col items-center gap-8 px-4">
    <h1 class="text-highlighted text-2xl font-semibold">Welcome to Waypoints</h1>
    <UIcon
      v-if="step === 'loading'"
      name="i-lucide-loader-circle"
      class="text-muted size-6 animate-spin" />
    <SyncIdentitySetup v-else-if="step === 'identity'" @success="step = 'territory'" />
    <TerritoryPicker v-else @success="navigateTo('/')" />
  </div>
</template>
