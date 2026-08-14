<script setup lang="ts">
  const revealed = ref(false);

  function toggle() {
    revealed.value = !revealed.value;
  }
</script>

<template>
  <span data-inline="spoiler" :class="{ 'spoiler--revealed': revealed }" @click.stop="toggle"
    ><slot
  /></span>
</template>
