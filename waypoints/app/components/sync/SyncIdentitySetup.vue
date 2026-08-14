<script setup lang="ts">
  import { ipc, createLogger, isNativeScanPlatform, nativeScan, cancelNativeScan } from "#imports";
  import { buildEmergencyKit, saveEmergencyKit } from "~/utils/emergency-kit";

  const logger = createLogger("SyncIdentitySetup");

  const emit = defineEmits<{
    success: [];
  }>();

  type Mode = "choose" | "create" | "import" | "pair";

  const mode = ref<Mode>("choose");
  const busy = ref(false);
  const error = ref<string | null>(null);

  // create flow
  const mnemonic = ref<string | null>(null);
  const memberId = ref<string | null>(null);
  const backedUp = ref(false);

  // import / pair flows
  const phraseInput = ref("");
  const pairingInput = ref("");

  async function createIdentity() {
    error.value = null;
    busy.value = true;
    try {
      const created = await ipc.sync.identityCreate();
      mnemonic.value = created.mnemonic;
      memberId.value = created.memberId;
      mode.value = "create";
    } catch (e) {
      logger.error("Identity creation failed", e);
      error.value = String(e);
    } finally {
      busy.value = false;
    }
  }

  async function confirmCreated() {
    error.value = null;
    busy.value = true;
    try {
      // A fresh identity gets its sync Territory right away; "already exists"
      // just means a previous attempt got this far.
      await ipc.sync.territoryCreate().catch((e) => {
        if (!String(e).includes("already exists")) throw e;
      });
      emit("success");
    } catch (e) {
      logger.error("Sync territory creation failed", e);
      error.value = String(e);
    } finally {
      busy.value = false;
    }
  }

  const kitBusy = ref(false);
  const kitSaved = ref(false);

  async function downloadKit() {
    if (!mnemonic.value || !memberId.value) return;
    kitBusy.value = true;
    try {
      const bytes = await buildEmergencyKit({
        mnemonic: mnemonic.value,
        memberId: memberId.value,
      });
      kitSaved.value = await saveEmergencyKit(bytes);
    } catch (e) {
      logger.error("Emergency kit export failed", e);
      error.value = String(e);
    } finally {
      kitBusy.value = false;
    }
  }

  async function importPhrase() {
    await submitSetupCode(phraseInput.value);
  }

  /** One entry point for anything a scan, paste, or link hands us — the
   * Rust engine classifies and dispatches, so a phrase pasted into the
   * pairing box (or a pairing code into the recovery box) still works. */
  async function submitSetupCode(input: string) {
    error.value = null;
    busy.value = true;
    try {
      await ipc.sync.submitSetupCode(input);
      emit("success");
    } catch (e) {
      logger.error("Setup code failed", e);
      error.value = String(e);
    } finally {
      busy.value = false;
    }
  }

  async function joinWithPairingCode() {
    await submitSetupCode(pairingInput.value);
  }

  function back() {
    error.value = null;
    mode.value = "choose";
  }

  // Camera scanning — offered wherever a code can be typed. Mobile scans
  // with the platform's native scanner (camera renders behind the webview,
  // so a see-through overlay is shown); desktop uses the webview scanner
  // modal. qr-scanner is only ever dynamically imported on non-native
  // platforms, keeping it out of the mobile load path.
  const scanOpen = ref(false);
  const hasCamera = ref(false);
  const nativePlatform = ref(false);
  const scanningNative = ref(false);

  onMounted(async () => {
    nativePlatform.value = await isNativeScanPlatform();
    if (nativePlatform.value) {
      hasCamera.value = true;
      return;
    }
    const { default: QrScanner } = await import("qr-scanner");
    hasCamera.value = await QrScanner.hasCamera().catch(() => false);
  });

  async function startScan() {
    if (!nativePlatform.value) {
      scanOpen.value = true;
      return;
    }
    scanningNative.value = true;
    document.body.classList.add("native-scan");
    try {
      const code = await nativeScan();
      if (code) await submitSetupCode(code);
    } catch (e) {
      logger.error("Native scan failed", e);
      error.value = String(e);
    } finally {
      document.body.classList.remove("native-scan");
      scanningNative.value = false;
    }
  }

  async function abortScan() {
    await cancelNativeScan();
  }

  // A deep link may have carried a setup code here (see plugins/deep-link):
  // prefill the matching input and let the user confirm — links never
  // auto-submit an identity adoption.
  const pendingCode = useState<string | null>("sync-pending-setup-code", () => null);
  onMounted(async () => {
    const code = pendingCode.value;
    if (!code) return;
    pendingCode.value = null;
    const classified = await ipc.sync.classifySetupCode(code);
    if (classified.kind === "phrase") {
      mode.value = "import";
      phraseInput.value = code;
    } else if (classified.kind === "pairing") {
      mode.value = "pair";
      pairingInput.value = code;
    }
  });

  async function onScanned(code: string) {
    await submitSetupCode(code);
  }

  const uiPickerButton = {
    base: "w-full flex flex-col items-center justify-center gap-3 p-6",
    leadingIcon: "size-8",
    label: "text-highlighted",
  };
</script>

<template>
  <div class="flex w-full max-w-md flex-col items-center gap-6">
    <!-- choose -->
    <template v-if="mode === 'choose'">
      <p class="text-muted text-center text-sm">
        Waypoints syncs through an <span class="text-primary">identity</span> that lives only on
        your devices — no account, no server-side login.
      </p>

      <div class="flex w-full flex-col gap-4">
        <UButton
          variant="outline"
          size="xl"
          icon="i-lucide-sparkles"
          label="Create a new identity"
          :loading="busy"
          :ui="uiPickerButton"
          @click="createIdentity" />
        <div class="flex w-full gap-4">
          <UButton
            variant="subtle"
            size="xl"
            icon="i-lucide-qr-code"
            label="Pair with another device"
            :ui="uiPickerButton"
            @click="mode = 'pair'" />
          <UButton
            variant="subtle"
            size="xl"
            icon="i-lucide-key-round"
            label="Recover from phrase"
            :ui="uiPickerButton"
            @click="mode = 'import'" />
        </div>
      </div>
    </template>

    <!-- create: reveal + confirm backup -->
    <template v-else-if="mode === 'create' && mnemonic">
      <p class="text-muted text-center text-sm">
        This recovery phrase <span class="text-highlighted">is</span> your identity. Write it down
        and keep it safe — it is the only backup, and it can never be shown to anyone else.
      </p>

      <SyncMnemonicWords :phrase="mnemonic" />

      <UButton
        variant="subtle"
        color="neutral"
        size="sm"
        :icon="kitSaved ? 'i-lucide-check' : 'i-lucide-file-down'"
        :label="kitSaved ? 'Emergency kit saved' : 'Download emergency kit'"
        :loading="kitBusy"
        @click="downloadKit" />

      <UCheckbox v-model="backedUp" label="I've written down my recovery phrase" />

      <UButton
        class="w-full justify-center"
        label="Continue"
        :disabled="!backedUp"
        :loading="busy"
        @click="confirmCreated" />
    </template>

    <!-- import phrase -->
    <template v-else-if="mode === 'import'">
      <p class="text-muted text-center text-sm">
        Enter your recovery phrase to bring your identity onto this device.
      </p>

      <UTextarea
        v-model="phraseInput"
        placeholder="apple banana cherry …"
        :rows="3"
        autofocus
        class="w-full font-mono" />

      <div class="flex w-full gap-2">
        <UButton variant="ghost" color="neutral" label="Back" @click="back" />
        <UButton
          v-if="hasCamera"
          variant="subtle"
          color="neutral"
          icon="i-lucide-scan-line"
          label="Scan"
          @click="startScan" />
        <UButton
          class="flex-1 justify-center"
          label="Recover identity"
          :disabled="!phraseInput.trim()"
          :loading="busy"
          @click="importPhrase" />
      </div>
    </template>

    <!-- pair -->
    <template v-else-if="mode === 'pair'">
      <p class="text-muted text-center text-sm">
        On a device that's already set up, open
        <span class="text-highlighted">Settings → Sync → Add device</span> and paste the pairing
        code here. Pairing brings over your identity, your Territory, and your Station.
      </p>

      <UTextarea
        v-model="pairingInput"
        placeholder='{"v":1,"kind":"waypoints-pairing",…}'
        :rows="4"
        autofocus
        class="w-full font-mono" />

      <div class="flex w-full gap-2">
        <UButton variant="ghost" color="neutral" label="Back" @click="back" />
        <UButton
          v-if="hasCamera"
          variant="subtle"
          color="neutral"
          icon="i-lucide-scan-line"
          label="Scan"
          @click="startScan" />
        <UButton
          class="flex-1 justify-center"
          label="Pair this device"
          :disabled="!pairingInput.trim()"
          :loading="busy"
          @click="joinWithPairingCode" />
      </div>
    </template>

    <UAlert
      v-if="error"
      color="error"
      variant="subtle"
      icon="i-lucide-triangle-alert"
      :description="error" />

    <LazySyncScanCode v-model:open="scanOpen" @code="onScanned" />

    <!-- Native scan overlay: the camera renders BEHIND the webview, so
         everything except the hint + cancel must be see-through. -->
    <Teleport to="body">
      <div
        v-if="scanningNative"
        class="fixed inset-0 z-50 flex flex-col items-center justify-end gap-4 pb-16">
        <p class="rounded-full bg-black/60 px-4 py-1.5 text-sm text-white">
          Point the camera at a setup code
        </p>
        <UButton label="Cancel" color="neutral" variant="solid" @click="abortScan" />
      </div>
    </Teleport>
  </div>
</template>

<style>
  /* While the native scanner runs, the page must be transparent so the
   camera preview (rendered behind the webview) shows through. */
  body.native-scan,
  body.native-scan #__nuxt {
    background: transparent !important;
  }
</style>
