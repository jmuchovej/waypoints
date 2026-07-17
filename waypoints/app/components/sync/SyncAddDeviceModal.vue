<script setup lang="ts">
import { qrcode } from "etiket/qr";
import { ipc, createLogger } from "#imports";

const logger = createLogger("SyncAddDeviceModal");

const open = defineModel<boolean>("open", { default: false });

const payload = ref<string | null>(null);
const qrSvg = ref<string | null>(null);
const error = ref<string | null>(null);
const copied = ref(false);

watch(open, async (isOpen) => {
  if (!isOpen) {
    // Don't keep the identity-bearing payload around longer than the modal.
    payload.value = null;
    qrSvg.value = null;
    error.value = null;
    copied.value = false;
    return;
  }
  try {
    payload.value = await ipc.sync.pairingPayload();
    qrSvg.value = qrcode(payload.value, { ecLevel: "M", margin: 0 });
  } catch (e) {
    logger.error("Pairing payload failed", e);
    error.value = String(e);
  }
});

async function copyCode() {
  if (!payload.value) return;
  await navigator.clipboard.writeText(payload.value);
  copied.value = true;
  setTimeout(() => (copied.value = false), 2000);
}
</script>

<template>
  <UModal v-model:open="open" title="Add a device">
    <template #content>
      <div class="flex flex-col items-center gap-4 p-6">
        <h2 class="text-lg font-semibold text-highlighted">Add a device</h2>
        <p class="text-sm text-muted text-center">
          On the new device, choose <span class="text-highlighted">Pair with another device</span>
          during setup, then scan this code — or paste the pairing code.
        </p>

        <UAlert
          v-if="error"
          color="error"
          variant="subtle"
          icon="i-lucide-triangle-alert"
          :description="error"
        />

        <template v-else>
          <!-- eslint-disable-next-line vue/no-v-html — SVG generated locally by etiket from our own payload -->
          <div
            v-if="qrSvg"
            role="img"
            aria-label="Pairing QR code"
            class="size-60 rounded-lg bg-white p-2 [&>svg]:size-full"
            v-html="qrSvg"
          />
          <div v-else class="flex items-center justify-center size-60">
            <UIcon name="i-lucide-loader-circle" class="animate-spin size-6 text-muted" />
          </div>

          <UButton
            variant="subtle"
            color="neutral"
            size="sm"
            :icon="copied ? 'i-lucide-check' : 'i-lucide-clipboard-copy'"
            :label="copied ? 'Copied' : 'Copy pairing code'"
            @click="copyCode"
          />

          <UAlert
            color="warning"
            variant="subtle"
            icon="i-lucide-shield-alert"
            description="This code contains your recovery phrase. Only show it to a device you own, and never share a screenshot of it."
          />
        </template>
      </div>
    </template>
  </UModal>
</template>
