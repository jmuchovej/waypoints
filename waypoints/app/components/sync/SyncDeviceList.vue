<script setup lang="ts">
import type { SyncDevice } from "@waypoints/cartograph";
import { ipc, createLogger } from "#imports";

const logger = createLogger("SyncDeviceList");

const devices = ref<SyncDevice[] | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const addDeviceOpen = ref(false);

async function load() {
  loading.value = true;
  error.value = null;
  try {
    devices.value = await ipc.sync.devices();
  } catch (e) {
    logger.error("Device listing failed", e);
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

const mine = computed(() => devices.value?.filter((d) => d.mine) ?? []);
const others = computed(() => devices.value?.filter((d) => !d.mine) ?? []);

function shortKey(hex: string): string {
  return `${hex.slice(0, 8)}…${hex.slice(-8)}`;
}

function lastSeenLabel(device: SyncDevice): string {
  if (device.connected) return "connected";
  if (device.lastSeen == null) return "never seen";
  const seconds = Math.max(0, Date.now() / 1000 - device.lastSeen);
  if (seconds < 90) return "just now";
  if (seconds < 3600) return `${Math.round(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)}h ago`;
  return `${Math.round(seconds / 86400)}d ago`;
}

onMounted(load);
</script>

<template>
  <UPageCard
    title="Devices"
    description="Every Vessel the Station has sighted on your Territory. Attribution is verified on this device — the Station is never taken at its word."
    variant="subtle"
  >
    <template #footer>
      <div class="flex gap-2">
        <UButton
          label="Add device"
          size="sm"
          icon="i-lucide-smartphone"
          @click="addDeviceOpen = true"
        />
        <UButton
          label="Refresh"
          size="sm"
          variant="ghost"
          color="neutral"
          icon="i-lucide-refresh-cw"
          :loading="loading"
          @click="load"
        />
      </div>
    </template>

    <div class="flex flex-col gap-4">
      <UAlert
        v-if="error"
        color="error"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        :description="error"
      />

      <template v-if="devices">
        <div v-if="mine.length" class="flex flex-col divide-y divide-default">
          <div v-for="device in mine" :key="device.deviceKey" class="flex items-center gap-3 py-3">
            <UIcon
              :name="device.thisDevice ? 'i-lucide-monitor-check' : 'i-lucide-monitor-smartphone'"
              class="size-5 text-muted shrink-0"
            />
            <div class="flex flex-col min-w-0">
              <span class="text-sm text-highlighted font-mono truncate">
                {{ shortKey(device.deviceKey) }}
                <span v-if="device.thisDevice" class="text-primary font-sans">(this device)</span>
              </span>
              <span class="text-xs text-dimmed">{{ lastSeenLabel(device) }}</span>
            </div>
            <UBadge
              class="ms-auto shrink-0"
              :color="device.connected ? 'success' : 'neutral'"
              variant="subtle"
            >
              {{ device.connected ? "Connected" : "Offline" }}
            </UBadge>
          </div>
        </div>
        <p v-else class="text-sm text-muted">
          No devices sighted yet — run a sync so the Station sees this one.
        </p>

        <template v-if="others.length">
          <USeparator label="Other identities" />
          <div class="flex flex-col divide-y divide-default">
            <div
              v-for="device in others"
              :key="device.deviceKey"
              class="flex items-center gap-3 py-3"
            >
              <UIcon name="i-lucide-users" class="size-5 text-muted shrink-0" />
              <div class="flex flex-col min-w-0">
                <span class="text-sm text-highlighted font-mono truncate">
                  {{ shortKey(device.deviceKey) }}
                </span>
                <span class="text-xs text-dimmed font-mono truncate">
                  {{ shortKey(device.memberId) }} · {{ lastSeenLabel(device) }}
                </span>
              </div>
              <UBadge
                class="ms-auto shrink-0"
                :color="device.connected ? 'success' : 'neutral'"
                variant="subtle"
              >
                {{ device.connected ? "Connected" : "Offline" }}
              </UBadge>
            </div>
          </div>
        </template>
      </template>

      <div v-else-if="loading" class="flex items-center gap-2 text-sm text-muted py-2">
        <UIcon name="i-lucide-loader-circle" class="animate-spin size-4" />
        Asking the Station…
      </div>
    </div>

    <LazySyncAddDeviceModal v-model:open="addDeviceOpen" />
  </UPageCard>
</template>
