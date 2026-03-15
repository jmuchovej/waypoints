<script setup lang="ts"></script>

<template>
  <UDashboardPanel id="home">
    <template #header>
      <UDashboardNavbar title="Waypoints">
        <template #leading>
          <UDashboardSidebarCollapse />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body> </template>
  </UDashboardPanel>
</template>

<style scoped>
#dashboard-panel-home {
  padding-left: env(titlebar-area-x);
}
</style>
