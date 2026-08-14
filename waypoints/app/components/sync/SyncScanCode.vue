<script setup lang="ts">
  import QrScanner from "qr-scanner";

  import { createLogger } from "#imports";

  const logger = createLogger("SyncScanCode");

  const emit = defineEmits<{
    /** A decoded QR — raw text, for `classifySetupCode`. */
    code: [string];
  }>();

  const open = defineModel<boolean>("open", { default: false });

  const video = ref<HTMLVideoElement>();
  const error = ref<string | null>(null);
  let scanner: QrScanner | null = null;

  function teardown() {
    scanner?.destroy();
    scanner = null;
  }

  watch(open, async (isOpen) => {
    if (!isOpen) {
      teardown();
      error.value = null;
      return;
    }
    await nextTick();
    if (!video.value) return;
    try {
      scanner = new QrScanner(
        video.value,
        (result) => {
          open.value = false;
          emit("code", result.data);
        },
        { returnDetailedScanResult: true, highlightScanRegion: true },
      );
      await scanner.start();
    } catch (e) {
      logger.error("Camera start failed", e);
      error.value =
        "Couldn't access the camera — check Waypoints' camera permission, or paste the code instead.";
      teardown();
    }
  });

  onBeforeUnmount(teardown);
</script>

<template>
  <UModal v-model:open="open" title="Scan a setup code">
    <template #content>
      <div class="flex flex-col items-center gap-4 p-6">
        <h2 class="text-highlighted text-lg font-semibold">Scan a setup code</h2>
        <p class="text-muted text-center text-sm">
          Point the camera at the QR code shown on your other device or printed in your Emergency
          Kit.
        </p>
        <!-- eslint-disable-next-line vuejs-accessibility/media-has-caption — live camera preview -->
        <video ref="video" class="bg-elevated aspect-square w-full rounded-lg object-cover" />
        <UAlert
          v-if="error"
          color="error"
          variant="subtle"
          icon="i-lucide-camera-off"
          :description="error" />
      </div>
    </template>
  </UModal>
</template>
