<script setup lang="ts">
  // TODO: implement pinned landmarks — requires a "pinned" flag or
  // a dedicated pin list stored in the territory registry / settings.
  useHead({
    title: "Pinned",
  });
</script>

<template>
  <div class="text-dimmed flex h-full flex-col items-center justify-center gap-2">
    <UIcon name="i-lucide-pin-off" class="size-8" />
    <p>No pinned landmarks yet</p>
  </div>
</template>
