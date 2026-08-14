<script setup lang="ts">
  const showTooltip = ref(false);

  function handleClick() {
    showTooltip.value = true;
    setTimeout(() => {
      showTooltip.value = false;
    }, 2000);
  }
</script>

<template>
  <UTooltip text="Comments coming soon" :open="showTooltip">
    <span data-inline="comment" @click.stop="handleClick"><slot /></span>
  </UTooltip>
</template>
