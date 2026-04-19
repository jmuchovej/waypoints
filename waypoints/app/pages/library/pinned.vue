<script setup lang="ts">
// TODO: implement pinned landmarks — requires a "pinned" flag or
// a dedicated pin list stored in the territory registry / settings.
useHead({
  title: "Pinned",
});
</script>

<template>
  <div class="flex flex-col items-center justify-center h-full gap-2 text-dimmed">
    <UIcon name="i-lucide-pin-off" class="size-8" />
    <p>No pinned landmarks yet</p>
  </div>
</template>
