<template>
  <div class="flex flex-1 relative">
    <UDashboardPanel id="home">
      <template #header>
        <UDashboardNavbar title="Waypoints" icon="i-lucide-house">
          <template #leading>
            <UDashboardSidebarCollapse />
          </template>
        </UDashboardNavbar>
      </template>

      <template #body>
        <div class="flex flex-1 items-center justify-center">
          <p class="text-muted text-lg">Welcome to Waypoints</p>
        </div>
      </template>
    </UDashboardPanel>
  </div>
</template>
