<script setup lang="ts">
  import type { NavigationMenuItem } from "@nuxt/ui";

  import { resourceGroups } from "~/data/navigation";

  const menu: NavigationMenuItem[] = [
    {
      label: "Resources",
      value: "resources",
      slot: "resources" as const,
      children: [{ label: "_placeholder" }],
    },
    {
      label: "Docs",
      to: "/docs/",
    },
  ];
</script>

<template>
  <UNavigationMenu :items="menu" :unmount-on-hide="false">
    <template #resources-content>
      <div class="divide-default grid grid-flow-col-dense divide-x">
        <div v-for="group in resourceGroups" :key="group.label" class="p-2 pt-1">
          <p class="text-muted px-2 py-1 text-xs font-semibold tracking-wider uppercase">
            {{ group.label }}
          </p>
          <ul class="space-y-0.5">
            <li v-for="link in group.items" :key="link.label">
              <NuxtLink
                :to="link.to"
                :target="link.target"
                class="hover:bg-elevated flex items-start gap-2.5 rounded-md px-2 py-1.5 transition-colors">
                <div class="min-w-0">
                  <p class="text-highlighted truncate text-sm font-medium">
                    {{ link.label }}
                  </p>
                  <p class="text-muted truncate text-xs">
                    {{ link.description }}
                  </p>
                </div>
              </NuxtLink>
            </li>
          </ul>
        </div>
      </div>
    </template>
  </UNavigationMenu>
</template>
