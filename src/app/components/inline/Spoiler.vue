<!--
  SPDX-FileCopyrightText: 2026 John Muchovej
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

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
