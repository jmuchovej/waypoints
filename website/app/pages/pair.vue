<script setup lang="ts">
  useHead({
    title: "Pair a device",
  });

  const fragment = ref<string | null>(null);
  const copied = ref(false);

  onMounted(() => {
    fragment.value = window.location.hash.length > 1 ? window.location.hash.slice(1) : null;
  });

  function openInApp() {
    if (!fragment.value) return;
    window.location.href = `waypoints://pair#${fragment.value}`;
  }

  async function copyCode() {
    if (!fragment.value) return;
    await navigator.clipboard.writeText(`https://waypoints.so/pair#${fragment.value}`);
    copied.value = true;
    setTimeout(() => (copied.value = false), 2000);
  }
</script>

<template>
  <div class="mx-auto flex max-w-lg flex-col items-center gap-6 px-4 py-24 text-center">
    <h1 class="text-2xl font-semibold">Pair this device with Waypoints</h1>

    <template v-if="fragment">
      <p class="text-sm opacity-70">
        This link carries a pairing code for your Waypoints identity. If Waypoints is already
        installed, open it in the app; otherwise install Waypoints first, then come back to this
        link (or paste the code during setup).
      </p>

      <div class="flex flex-wrap justify-center gap-3">
        <UButton size="lg" icon="i-lucide-rocket" label="Open in Waypoints" @click="openInApp" />
        <UButton size="lg" variant="subtle" to="/download/stable" label="Install Waypoints" />
        <UButton
          size="lg"
          variant="ghost"
          :icon="copied ? 'i-lucide-check' : 'i-lucide-clipboard-copy'"
          :label="copied ? 'Copied' : 'Copy pairing link'"
          @click="copyCode" />
      </div>

      <p class="text-xs opacity-60">
        This pairing code is as sensitive as your recovery phrase – never share it, and only use it
        on a device you own.
      </p>
    </template>

    <template v-else>
      <p class="text-sm opacity-70">
        This page is the landing spot for Waypoints pairing links, but this link carries no pairing
        code. Generate one from
        <span class="font-medium">Settings → Sync → Add device</span> on a device that's already set
        up.
      </p>
      <UButton size="lg" variant="subtle" to="/download/stable" label="Install Waypoints" />
    </template>
  </div>
</template>
