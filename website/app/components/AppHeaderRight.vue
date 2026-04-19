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

  <ColorModeButton v-if="!forcedColorMode" />

  <UContentSearchButton variant="ghost" class="hidden lg:inline-flex" :ui="{ trailing: 'size-6' }">
    <UKbd variant="subtle" value="meta" />
    <UKbd variant="subtle" value="K" />
  </UContentSearchButton>

  <USeparator orientation="vertical" class="hidden lg:block h-8" />

  <ClientOnly>
    <UButton
      v-if="!session"
      label="Sign In"
      variant="subtle"
      color="neutral"
      to="/auth/login"
      class="hidden lg:inline-flex"
    />
    <UButton
      v-else
      label="Sign Out"
      variant="subtle"
      color="neutral"
      class="hidden lg:inline-flex"
      @click="signOut()"
    />
  </ClientOnly>
  <UButton variant="subtle" color="primary" to="/download" class="hidden lg:inline-flex">
    <template #default> Download </template>
    <template #trailing>
      <UKbd> D </UKbd>
    </template>
  </UButton>
</template>
