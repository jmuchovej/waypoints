<script setup lang="ts">
import { useDocusColorMode, useDocusI18n, useSubNavigation } from "#imports";

const appConfig = useAppConfig();
const { forced: forcedColorMode } = useDocusColorMode();

const { isEnabled: isAssistantEnabled } = useAssistant();
const { isEnabled, locales } = useDocusI18n();
const { subNavigationMode } = useSubNavigation();
</script>

<template>
  <UHeader :ui="{ center: 'flex-1' }" :class="{ 'flex flex-col': subNavigationMode === 'header' }">
    <AppHeaderCenter />

    <template #left>
      <AppHeaderLeft />
    </template>

    <template #right>
      <AppHeaderRight />
    </template>

    <template #toggle="{ open, toggle }">
      <IconMenuToggle :open="open" class="lg:hidden" @click="toggle" />
    </template>

    <template #body>
      <AppHeaderBody />
    </template>

    <template v-if="subNavigationMode === 'header'" #bottom>
      <AppHeaderBottom />
    </template>
  </UHeader>
</template>
