<script setup lang="ts">
  import { useDocusColorMode, useDocusI18n, useSubNavigation } from "#imports";

  const { forced: forcedColorMode } = useDocusColorMode();
  const assistant = useAssistant();
  const i18n = useDocusI18n();
  const { subNavigationMode } = useSubNavigation();

  const { data: session } = await useSession();
  const { signOut } = useAuth();

  defineShortcuts({
    d: () => navigateTo("/download"),
  });
</script>

<template>
  <template v-if="assistant.isEnabled">
    <LazyAssistantChat />
  </template>

  <template v-if="i18n.isEnabled && i18n.locales.length > 1">
    <ClientOnly>
      <LazyLanguageSelect />

      <template #fallback>
        <div class="h-8 w-8 animate-pulse rounded-md bg-neutral-200 dark:bg-neutral-800" />
      </template>
    </ClientOnly>

    <USeparator orientation="vertical" class="h-8" />
  </template>

  <ColorModeButton v-if="!forcedColorMode" />

  <UContentSearchButton variant="ghost" :ui="{ base: 'hidden lg:inline-flex', trailing: 'size-6' }">
    <UKbd value="meta" />
    <UKbd value="K" />
  </UContentSearchButton>

  <USeparator orientation="vertical" class="hidden h-8 lg:block" />

  <ClientOnly>
    <UTheme :ui="{ button: { base: 'hidden lg:inline-flex' } }">
      <UButton v-if="!session" label="Sign In" to="/auth/login" />
      <UButton v-else label="Sign Out" @click="signOut()" />
    </UTheme>
  </ClientOnly>
  <UButton color="primary" to="/download" class="hidden lg:inline-flex">
    <template #default> Download </template>
    <template #trailing>
      <UKbd value="D" />
    </template>
  </UButton>
</template>
