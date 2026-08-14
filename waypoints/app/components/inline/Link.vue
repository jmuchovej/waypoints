<script setup lang="ts">
  const props = defineProps<{
    data: unknown;
  }>();

  const href = computed(() => {
    if (typeof props.data === "string") return props.data;
    if (props.data && typeof props.data === "object" && "href" in props.data) {
      return (props.data as Record<string, unknown>).href as string;
    }
    return undefined;
  });

  function handleClick(e: MouseEvent) {
    e.preventDefault();
    if ((e.metaKey || e.ctrlKey) && href.value) {
      window.open(href.value, "_blank", "noopener");
    }
  }
</script>

<template>
  <a :href="href" data-inline="link" target="_blank" rel="noopener" @click.prevent="handleClick"
    ><slot
  /></a>
</template>
