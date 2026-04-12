<script setup lang="ts">
definePageMeta({ layout: "onboarding" });
</script>

<template>
  <div class="flex flex-col items-center gap-8 w-full max-w-lg px-4">
    <h1 class="text-2xl font-semibold text-highlighted">Welcome to Waypoints</h1>
    <TerritoryPicker @success="navigateTo('/')" />
  </div>
</template>
