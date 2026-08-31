<script setup lang="ts">
  const colorMode = useColorMode();

  const modes = ["system", "light", "dark"] as const;

  const icon = computed(() => {
    switch (colorMode.preference) {
      case "light":
        return "i-lucide-sun";
      case "dark":
        return "i-lucide-moon";
      default:
        return "i-lucide-sun-moon";
    }
  });

  const label = computed(() => {
    switch (colorMode.preference) {
      case "light":
        return "Light mode";
      case "dark":
        return "Dark mode";
      default:
        return "System mode";
    }
  });

  function cycle() {
    const current = modes.indexOf(colorMode.preference as (typeof modes)[number]);
    colorMode.preference = modes[(current + 1) % modes.length];
  }
</script>

<template>
  <ClientOnly>
    <UButton color="neutral" variant="ghost" :icon="icon" :aria-label="label" @click="cycle" />
    <template #fallback>
      <div class="h-8 w-8 animate-pulse rounded-md bg-neutral-200 dark:bg-neutral-800" />
    </template>
  </ClientOnly>
</template>
