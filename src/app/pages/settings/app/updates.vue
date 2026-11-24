<!--
  SPDX-FileCopyrightText: 2026 John Muchovej
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script setup lang="ts">
  import type { RadioGroupItem } from "@nuxt/ui";
  import type { SelfUpdatePolicy, SelfUpdateStatus } from "@waypoints/cartograph";

  import { ipc, createLogger } from "#imports";

  const logger = createLogger("SettingsUpdates");

  /** Where the notice sends a user whose copy is managed by a package manager. */
  const MANAGED_INSTALLS_URL = "https://waypoints.so/docs/installation/managed-installs";

  const status = ref<SelfUpdateStatus | null>(null);

  // Least to most permissive — the order the backend compares them in.
  const POLICIES: { value: SelfUpdatePolicy; label: string; description: string }[] = [
    { value: "off", label: "Off", description: "Never check for a newer version." },
    {
      value: "notify",
      label: "Notify",
      description: "Check, and tell me when a newer version is available.",
    },
    {
      value: "on",
      label: "On",
      description: "Check, download, and install updates automatically.",
    },
  ];

  const rank = (policy: SelfUpdatePolicy) => POLICIES.findIndex((p) => p.value === policy);

  const capped = computed(() => status.value !== null && status.value.cap !== "on");

  const items = computed<RadioGroupItem[]>(() =>
    POLICIES.map((policy) => ({
      ...policy,
      disabled: status.value !== null && rank(policy.value) > rank(status.value.cap),
    })),
  );

  async function load() {
    try {
      status.value = await ipc.app.selfUpdateStatus();
    } catch (e) {
      logger.error("Self-update status load failed", e);
    }
  }

  async function select(setting: SelfUpdatePolicy) {
    try {
      status.value = await ipc.app.selfUpdateSet(setting);
    } catch (e) {
      logger.error("Self-update setting write failed", e);
    }
  }

  function learnMore() {
    window.open(MANAGED_INSTALLS_URL, "_blank", "noopener");
  }

  onMounted(load);
</script>

<template>
  <UPageCard
    title="Updates"
    description="What Waypoints does when a newer version is available."
    variant="subtle">
    <div class="flex flex-col gap-4">
      <URadioGroup
        :model-value="status?.setting"
        :items="items"
        :disabled="status === null"
        variant="card"
        @update:model-value="select($event as SelfUpdatePolicy)" />

      <UAlert
        v-if="capped"
        data-testid="managed-notice"
        icon="i-lucide-package"
        color="neutral"
        variant="subtle">
        <template #description>
          Self-updating is disabled because this copy of Waypoints is managed by a package manager,
          which delivers updates itself.
          <a
            :href="MANAGED_INSTALLS_URL"
            class="underline"
            target="_blank"
            rel="noopener"
            @click.prevent="learnMore"
            >Learn more</a
          >
        </template>
      </UAlert>
    </div>
  </UPageCard>
</template>
