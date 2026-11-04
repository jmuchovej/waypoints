<!--
  SPDX-FileCopyrightText: 2026 John Muchovej
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script setup lang="ts">
  import type { ContentNode } from "#imports";

  defineProps<{ block: ContentNode<"divider"> }>();
</script>

<template>
  <div class="border-default my-4 border-t" />
</template>
