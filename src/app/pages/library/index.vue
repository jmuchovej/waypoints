<!--
  SPDX-FileCopyrightText: 2026 John Muchovej
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script setup lang="ts">
  // Route-level, and carrying the query and hash, so a posed link to /library
  // arrives posed even when it is the link that launched the app.
  definePageMeta({
    redirect: (to) => ({ path: "/library/recents", query: to.query, hash: to.hash }),
  });
</script>

<template />
