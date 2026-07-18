<script setup lang="ts">
import QrScanner from "qr-scanner";
import { ipc, createLogger, classifySetupCode } from "#imports";
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
 * classifier decides which flow the input drives, so a phrase pasted into
 * the pairing box (or a pairing code into the recovery box) still works. */
async function submitSetupCode(input: string) {
  error.value = null;
  const code = classifySetupCode(input);
  if (code.kind === "unknown") {
    error.value =
      "That doesn't look like a setup code — paste a pairing code or a recovery phrase exactly as it appears.";
    return;
  }
  busy.value = true;
  try {
    if (code.kind === "pairing") {
      await ipc.sync.pairingJoin(code.code);
    } else {
      await ipc.sync.identityImport(code.phrase);
    }
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

// Camera scanning — offered wherever a code can be typed.
const scanOpen = ref(false);
const hasCamera = ref(false);
onMounted(async () => {
  hasCamera.value = await QrScanner.hasCamera().catch(() => false);
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
  <div class="flex flex-col items-center gap-6 w-full max-w-md">
    <!-- choose -->
    <template v-if="mode === 'choose'">
      <p class="text-sm text-muted text-center">
        Waypoints syncs through an <span class="text-primary">identity</span> that lives only on
        your devices — no account, no server-side login.
      </p>

      <div class="flex flex-col gap-4 w-full">
        <UButton
          variant="outline"
          size="xl"
          icon="i-lucide-sparkles"
          label="Create a new identity"
          :loading="busy"
          :ui="uiPickerButton"
          @click="createIdentity"
        />
        <div class="flex gap-4 w-full">
          <UButton
            variant="subtle"
            size="xl"
            icon="i-lucide-qr-code"
            label="Pair with another device"
            :ui="uiPickerButton"
            @click="mode = 'pair'"
          />
          <UButton
            variant="subtle"
            size="xl"
            icon="i-lucide-key-round"
            label="Recover from phrase"
            :ui="uiPickerButton"
            @click="mode = 'import'"
          />
        </div>
      </div>
    </template>

    <!-- create: reveal + confirm backup -->
    <template v-else-if="mode === 'create' && mnemonic">
      <p class="text-sm text-muted text-center">
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
        @click="downloadKit"
      />

      <UCheckbox v-model="backedUp" label="I've written down my recovery phrase" />

      <UButton
        class="w-full justify-center"
        label="Continue"
        :disabled="!backedUp"
        :loading="busy"
        @click="confirmCreated"
      />
    </template>

    <!-- import phrase -->
    <template v-else-if="mode === 'import'">
      <p class="text-sm text-muted text-center">
        Enter your recovery phrase to bring your identity onto this device.
      </p>

      <UTextarea
        v-model="phraseInput"
        placeholder="apple banana cherry …"
        :rows="3"
        autofocus
        class="w-full font-mono"
      />

      <div class="flex gap-2 w-full">
        <UButton variant="ghost" color="neutral" label="Back" @click="back" />
        <UButton
          v-if="hasCamera"
          variant="subtle"
          color="neutral"
          icon="i-lucide-scan-line"
          label="Scan"
          @click="scanOpen = true"
        />
        <UButton
          class="flex-1 justify-center"
          label="Recover identity"
          :disabled="!phraseInput.trim()"
          :loading="busy"
          @click="importPhrase"
        />
      </div>
    </template>

    <!-- pair -->
    <template v-else-if="mode === 'pair'">
      <p class="text-sm text-muted text-center">
        On a device that's already set up, open
        <span class="text-highlighted">Settings → Sync → Add device</span> and paste the pairing
        code here. Pairing brings over your identity, your Territory, and your Station.
      </p>

      <UTextarea
        v-model="pairingInput"
        placeholder='{"v":1,"kind":"waypoints-pairing",…}'
        :rows="4"
        autofocus
        class="w-full font-mono"
      />

      <div class="flex gap-2 w-full">
        <UButton variant="ghost" color="neutral" label="Back" @click="back" />
        <UButton
          v-if="hasCamera"
          variant="subtle"
          color="neutral"
          icon="i-lucide-scan-line"
          label="Scan"
          @click="scanOpen = true"
        />
        <UButton
          class="flex-1 justify-center"
          label="Pair this device"
          :disabled="!pairingInput.trim()"
          :loading="busy"
          @click="joinWithPairingCode"
        />
      </div>
    </template>

    <UAlert
      v-if="error"
      color="error"
      variant="subtle"
      icon="i-lucide-triangle-alert"
      :description="error"
    />

    <LazySyncScanCode v-model:open="scanOpen" @code="onScanned" />
  </div>
</template>
