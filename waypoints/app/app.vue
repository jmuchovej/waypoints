<template>
  <Html class="app-wrapper overflow-x-hidden" lang="en">
    <Body class="font-sans antialiased">
      <UApp>
        <NuxtLayout>
          <NuxtPage />
        </NuxtLayout>
      </UApp>
    </Body>
  </Html>
</template>

<style>
@reference "~/assets/css/main.css";

.app-wrapper {
  @apply flex flex-col h-dvh overflow-hidden;
  @apply pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)];
}
</style>
