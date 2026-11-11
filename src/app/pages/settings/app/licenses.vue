<!--
  SPDX-FileCopyrightText: 2026 John Muchovej
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script setup lang="ts">
  const notices = ref<string>();
  const unavailable = ref(false);

  onMounted(async () => {
    if (!("isTauri" in window)) {
      unavailable.value = true;
      return;
    }
    try {
      const { BaseDirectory, readTextFile } = await import("@tauri-apps/plugin-fs");
      notices.value = await readTextFile("licenses/THIRD-PARTY-NOTICES.md", {
        baseDir: BaseDirectory.Resource,
      });
    } catch (error) {
      console.error("Failed to read the third-party notices", error);
      unavailable.value = true;
    }
  });
</script>

<template>
  <UPage>
    <UPageHeader
      title="Licenses"
      description="Waypoints is free software under the GNU AGPL, version 3 or any later version. It includes the third-party software listed here."
      headline="App Settings" />
    <p v-if="unavailable" class="text-muted text-sm">
      The notices ship with the installed application.
    </p>
    <pre v-else-if="notices" class="text-xs break-words whitespace-pre-wrap">{{ notices }}</pre>
  </UPage>
</template>
