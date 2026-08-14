<script setup lang="ts">
  import type { SyncStatus } from "@waypoints/cartograph";

  import { ipc, createLogger, navigateTo } from "#imports";
  import { buildEmergencyKit, saveEmergencyKit } from "~/utils/emergency-kit";

  const logger = createLogger("SettingsSync");

  const status = ref<SyncStatus | null>(null);
  const mnemonic = ref<string | null>(null);
  const revealOpen = ref(false);
  const copied = ref(false);

  async function load() {
    try {
      status.value = await ipc.sync.status();
    } catch (e) {
      logger.error("Sync status load failed", e);
    }
  }

  async function reveal() {
    try {
      mnemonic.value = await ipc.sync.identityMnemonic();
      revealOpen.value = true;
    } catch (e) {
      logger.error("Mnemonic reveal failed", e);
    }
  }

  watch(revealOpen, (isOpen) => {
    if (!isOpen) mnemonic.value = null;
  });

  const kitBusy = ref(false);

  async function downloadKit() {
    if (!status.value?.memberId) return;
    kitBusy.value = true;
    try {
      // The phrase is only pulled for the moment of the export.
      const phrase = await ipc.sync.identityMnemonic();
      const bytes = await buildEmergencyKit({
        mnemonic: phrase,
        memberId: status.value.memberId,
      });
      await saveEmergencyKit(bytes);
    } catch (e) {
      logger.error("Emergency kit export failed", e);
    } finally {
      kitBusy.value = false;
    }
  }

  function goToOnboarding() {
    navigateTo("/onboarding");
  }

  async function copyMemberId() {
    if (!status.value?.memberId) return;
    await navigator.clipboard.writeText(status.value.memberId);
    copied.value = true;
    setTimeout(() => (copied.value = false), 2000);
  }

  function shortKey(hex: string): string {
    return `${hex.slice(0, 12)}…${hex.slice(-12)}`;
  }

  onMounted(load);
</script>

<template>
  <!-- Identity -->
  <UPageCard
    title="Identity"
    description="Your identity is a keypair derived from your recovery phrase — it lives only on your devices."
    variant="subtle">
    <div v-if="status?.hasIdentity" class="flex flex-col gap-3">
      <div class="flex items-center gap-2">
        <span class="text-muted shrink-0 text-sm">Member ID</span>
        <span class="text-highlighted truncate font-mono text-sm">
          {{ status.memberId ? shortKey(status.memberId) : "—" }}
        </span>
        <UButton
          variant="ghost"
          color="neutral"
          size="xs"
          :icon="copied ? 'i-lucide-check' : 'i-lucide-clipboard-copy'"
          aria-label="Copy member ID"
          @click="copyMemberId" />
      </div>
      <div class="flex items-center gap-2">
        <span class="text-muted shrink-0 text-sm">Device index</span>
        <span class="text-highlighted font-mono text-sm">{{ status.deviceIndex }}</span>
      </div>
      <div class="flex gap-2">
        <UButton
          label="Reveal recovery phrase"
          size="sm"
          variant="subtle"
          color="neutral"
          icon="i-lucide-eye"
          @click="reveal" />
        <UButton
          label="Download emergency kit"
          size="sm"
          variant="subtle"
          color="neutral"
          icon="i-lucide-file-down"
          :loading="kitBusy"
          @click="downloadKit" />
      </div>
    </div>
    <div v-else-if="status" class="flex flex-col gap-3">
      <p class="text-muted text-sm">This device has no identity yet.</p>
      <UButton label="Set up sync" size="sm" class="w-fit" @click="goToOnboarding" />
    </div>
  </UPageCard>

  <template v-if="status?.hasIdentity">
    <SyncStationCard />
    <SyncDeviceList v-if="status.hasTerritory && status.stationConfigured" />
  </template>

  <!-- Recovery phrase modal -->
  <UModal v-model:open="revealOpen" title="Recovery phrase">
    <template #content>
      <div class="flex flex-col items-center gap-4 p-6">
        <h2 class="text-highlighted text-lg font-semibold">Recovery phrase</h2>
        <SyncMnemonicWords v-if="mnemonic" :phrase="mnemonic" />
        <UAlert
          color="warning"
          variant="subtle"
          icon="i-lucide-shield-alert"
          description="Anyone with this phrase is you. Never enter it anywhere except a device you are setting up." />
      </div>
    </template>
  </UModal>
</template>
