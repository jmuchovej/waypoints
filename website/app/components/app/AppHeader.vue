<script setup lang="ts">
  import type { ThemeUI } from "@nuxt/ui";

  import { useDocusColorMode, useDocusI18n, useSubNavigation } from "#imports";

  const appConfig = useAppConfig();
  const { header } = appConfig;
  const { forced: forcedColorMode } = useDocusColorMode();

  const { isEnabled: isAssistantEnabled } = useAssistant();
  const { isEnabled, locales } = useDocusI18n();
  const { subNavigationMode } = useSubNavigation();

  const drawerOpen = ref(false);

  useHead({
    // `logo` is optional in Docus' app config schema, so fall back to the same
    // value app.config.ts sets rather than asserting it is present.
    link: [{ rel: "icon", type: "image/svg+xml", href: header.logo?.favicon ?? "/favicon.svg" }],
  });

  const theming: ThemeUI = {
    header: {
      left: "lg:flex-none",
      right: "lg:flex-none",
      center: "flex-1",
      container: "px-4 sm:px-6 lg:px-8",
    },
    navigationMenu: {
      root: "flex-1",
      content: "sm:w-auto",
      viewportWrapper: "absolute top-full left-0 flex justify-start",
      viewport: "sm:w-[var(--reka-navigation-menu-viewport-width)] origin-[top_center] z-1",
    },
  };
</script>

<template>
  <UTheme :ui="theming">
    <UHeader :class="{ 'flex flex-col': subNavigationMode === 'header' }">
      <template #left>
        <AppHeaderLeft />
      </template>

      <AppHeaderCenter />

      <template #right>
        <AppHeaderRight />
      </template>

      <template #toggle>
        <ClientOnly>
          <UDrawer v-model:open="drawerOpen" direction="bottom" should-scale-background>
            <IconMenuToggle :open="drawerOpen" class="lg:hidden" />

            <template #body>
              <LazyAppHeaderBody />
            </template>
          </UDrawer>
        </ClientOnly>
      </template>

      <template v-if="subNavigationMode === 'header'" #bottom>
        <LazyAppHeaderBottom />
      </template>
    </UHeader>
  </UTheme>
</template>
