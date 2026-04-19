<script setup lang="ts">
import { Show, SignInButton, SignOutButton } from "@clerk/vue";
import { useDocusColorMode, useDocusI18n, useSubNavigation } from "#imports";

const { forced: forcedColorMode } = useDocusColorMode();
const { isEnabled: isAssistantEnabled } = useAssistant();
const { isEnabled, locales } = useDocusI18n();
const { subNavigationMode } = useSubNavigation();

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
    <Show when="signed-out">
      <SignInButton>
        <UButton label="Sign In" variant="subtle" color="neutral" />
      </SignInButton>
    </Show>
    <Show when="signed-in">
      <SignOutButton>
        <UButton label="Sign Out" />
      </SignOutButton>
    </Show>
  </ClientOnly>
  <UButton variant="subtle" color="primary" to="/download">
    <template #default> Download </template>
    <template #trailing>
      <UKbd> D </UKbd>
    </template>
  </UButton>
</template>
