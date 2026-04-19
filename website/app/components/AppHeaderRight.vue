<script setup lang="ts">
import { useDocusColorMode, useDocusI18n, useSubNavigation } from "#imports";

const { forced: forcedColorMode } = useDocusColorMode();
const { isEnabled: isAssistantEnabled } = useAssistant();
const { isEnabled, locales } = useDocusI18n();
const { subNavigationMode } = useSubNavigation();

const { data: session } = await useSession();
const { signOut } = useAuth();

defineShortcuts({
  d: () => navigateTo("/download"),
});
</script>

<template>
  <template v-if="isAssistantEnabled">
    <AssistantChat />
  </template>

  <template v-if="isEnabled && locales.length > 1">
    <ClientOnly>
      <LanguageSelect />

      <template #fallback>
        <div class="h-8 w-8 animate-pulse bg-neutral-200 dark:bg-neutral-800 rounded-md" />
      </template>
    </ClientOnly>

    <USeparator orientation="vertical" class="h-8" />
  </template>

  <ClientOnly v-if="!forcedColorMode">
    <UColorModeButton />

    <template #fallback>
      <div class="h-8 w-8 animate-pulse bg-neutral-200 dark:bg-neutral-800 rounded-md" />
    </template>
  </ClientOnly>

  <UContentSearchButton variant="ghost" :ui="{ trailing: 'size-6' }">
    <UKbd variant="subtle" value="meta" />
    <UKbd variant="subtle" value="K" />
  </UContentSearchButton>
  <USeparator orientation="vertical" class="h-8" />
  <ClientOnly>
    <UButton v-if="!session" label="Sign In" variant="subtle" color="neutral" to="/auth/login" />
    <UButton v-else label="Sign Out" variant="subtle" color="neutral" @click="signOut()" />
  </ClientOnly>
  <UButton variant="subtle" color="primary" to="/download">
    <template #default> Download </template>
    <template #trailing>
      <UKbd> D </UKbd>
    </template>
  </UButton>
</template>
