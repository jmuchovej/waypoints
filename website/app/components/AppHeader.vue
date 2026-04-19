<script setup lang="ts">
import { useDocusColorMode, useDocusI18n, useSubNavigation } from "#imports";

const appConfig = useAppConfig();
const { header } = appConfig;
const { forced: forcedColorMode } = useDocusColorMode();

const { isEnabled: isAssistantEnabled } = useAssistant();
const { isEnabled, locales } = useDocusI18n();
const { subNavigationMode } = useSubNavigation();

const drawerOpen = ref(false);

useHead({
  link: [{ rel: "icon", type: "image/svg+xml", href: header.logo.favicon }],
});
</script>

<template>
  <UHeader
    :ui="{
      left: 'lg:flex-none',
      right: 'lg:flex-none',
      center: 'flex-1',
    }"
    :class="{ 'flex flex-col': subNavigationMode === 'header' }"
  >
    <template #left>
      <AppHeaderLeft />
    </template>

    <AppHeaderCenter />

    <template #right>
      <AppHeaderRight />
    </template>

    <template #toggle>
      <ClientOnly>
        <UDrawer
          v-model:open="drawerOpen"
          direction="bottom"
          should-scale-background
        >
          <IconMenuToggle :open="drawerOpen" class="lg:hidden" />

          <template #body>
            <AppHeaderBody />
          </template>
        </UDrawer>
      </ClientOnly>
    </template>

    <template v-if="subNavigationMode === 'header'" #bottom>
      <AppHeaderBottom />
    </template>
  </UHeader>
</template>
