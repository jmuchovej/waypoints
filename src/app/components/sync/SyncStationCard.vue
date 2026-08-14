<script setup lang="ts">
  import type { StationStatus, SyncStatus } from "@waypoints/cartograph";

  import { ipc, createLogger } from "#imports";

  const logger = createLogger("SyncStationCard");

  const status = ref<SyncStatus | null>(null);
  const station = ref<StationStatus | null>(null);
  const addrInput = ref("");
  const editing = ref(false);
  const pinging = ref(false);
  const syncing = ref(false);
  const creatingTerritory = ref(false);
  const error = ref<string | null>(null);

  async function load() {
    try {
      status.value = await ipc.sync.status();
      if (status.value.stationConfigured) {
        await ping();
      }
    } catch (e) {
      logger.error("Sync status load failed", e);
      error.value = String(e);
    }
  }

  async function ping() {
    pinging.value = true;
    error.value = null;
    try {
      station.value = await ipc.sync.stationStatus();
    } catch (e) {
      logger.error("Station ping failed", e);
      error.value = String(e);
    } finally {
      pinging.value = false;
    }
  }

  async function saveStation() {
    pinging.value = true;
    error.value = null;
    try {
      station.value = await ipc.sync.stationSet(addrInput.value);
      status.value = await ipc.sync.status();
      editing.value = false;
      addrInput.value = "";
    } catch (e) {
      logger.error("Station save failed", e);
      error.value = String(e);
    } finally {
      pinging.value = false;
    }
  }

  async function syncNow() {
    syncing.value = true;
    error.value = null;
    try {
      status.value = await ipc.sync.syncNow();
      await ping();
    } catch (e) {
      logger.error("Sync failed", e);
      error.value = String(e);
    } finally {
      syncing.value = false;
    }
  }

  async function createTerritory() {
    creatingTerritory.value = true;
    error.value = null;
    try {
      status.value = await ipc.sync.territoryCreate();
      await ping();
    } catch (e) {
      logger.error("Sync territory creation failed", e);
      error.value = String(e);
    } finally {
      creatingTerritory.value = false;
    }
  }

  const statusColor = computed(() => {
    if (!station.value?.configured) return "neutral";
    if (!station.value.reachable) return "error";
    return station.value.servesTerritory ? "success" : "warning";
  });

  const statusLabel = computed(() => {
    if (!station.value?.configured) return "Not configured";
    if (!station.value.reachable) return "Unreachable";
    if (!station.value.servesTerritory) return "Reachable — territory not served";
    return `Connected · ${station.value.latencyMs}ms`;
  });

  onMounted(load);

  defineExpose({ refresh: load });
</script>

<template>
  <UPageCard
    title="Station"
    description="The always-on peer this device syncs through. Paste the address a Station publishes (its station.addr file)."
    variant="subtle">
    <div class="flex flex-col gap-4">
      <div class="flex items-center gap-3">
        <UBadge :color="statusColor" variant="subtle" class="gap-1.5">
          <span
            class="size-1.5 rounded-full"
            :class="{
              'bg-success animate-pulse': statusColor === 'success',
              'bg-warning': statusColor === 'warning',
              'bg-error': statusColor === 'error',
              'bg-neutral': statusColor === 'neutral',
            }" />
          {{ statusLabel }}
        </UBadge>
        <UButton
          v-if="status?.stationConfigured"
          variant="ghost"
          color="neutral"
          size="xs"
          icon="i-lucide-refresh-cw"
          :loading="pinging"
          aria-label="Ping station"
          @click="ping" />
        <span v-if="station?.error" class="text-error truncate text-xs">{{ station.error }}</span>
      </div>

      <p v-if="status?.stationAddr && !editing" class="text-dimmed truncate font-mono text-xs">
        {{ status.stationAddr }}
      </p>

      <template v-if="editing || !status?.stationConfigured">
        <UTextarea
          v-model="addrInput"
          placeholder='{"id":"…","addrs":["…"]} — contents of station.addr'
          :rows="3"
          class="w-full font-mono" />
        <div class="flex gap-2">
          <UButton
            label="Save & test"
            size="sm"
            :disabled="!addrInput.trim()"
            :loading="pinging"
            @click="saveStation" />
          <UButton
            v-if="status?.stationConfigured"
            label="Cancel"
            size="sm"
            variant="ghost"
            color="neutral"
            @click="editing = false" />
        </div>
      </template>

      <div v-else class="flex gap-2">
        <UButton
          v-if="status?.hasTerritory"
          label="Sync now"
          size="sm"
          icon="i-lucide-refresh-cw"
          :loading="syncing"
          @click="syncNow" />
        <UButton
          v-else
          label="Create sync territory"
          size="sm"
          icon="i-lucide-map-plus"
          :loading="creatingTerritory"
          @click="createTerritory" />
        <UButton
          label="Change station"
          size="sm"
          variant="ghost"
          color="neutral"
          @click="editing = true" />
      </div>

      <UAlert
        v-if="error"
        color="error"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        :description="error" />
    </div>
  </UPageCard>
</template>
