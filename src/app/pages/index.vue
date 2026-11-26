<!--
  SPDX-FileCopyrightText: 2026 John Muchovej
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script setup lang="ts">
  // The root redirects to /library/recents — the library is the default
  // landing surface. A route-level redirect, not a navigation from setup, so it
  // resolves within the initial navigation and a deep link that launched the
  // app is never overtaken by it.
  definePageMeta({ redirect: "/library/recents" });
</script>

<template />
