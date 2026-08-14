<script setup lang="ts">
  import type { DropdownMenuItem } from "@nuxt/ui";

  type SidebarState = "expanded" | "collapsed";

  const props = defineProps<{
    collapsed?: SidebarState;
  }>();

  const colorMode = useColorMode();
  const appConfig = useAppConfig();

  const colors = [
    "red",
    "orange",
    "amber",
    "yellow",
    "lime",
    "green",
    "emerald",
    "teal",
    "cyan",
    "sky",
    "blue",
    "indigo",
    "violet",
    "purple",
    "fuchsia",
    "pink",
    "rose",
  ];
  const neutrals = ["slate", "gray", "zinc", "stone", "taupe", "mauve", "mist", "olive"];

  const user = ref({
    name: "John Muchovej",
    avatar: {
      src: "https://github.com/jmuchovej.png",
      alt: "John Muchovej",
    },
  });

  const items = computed<DropdownMenuItem[][]>(() => [
    [
      {
        type: "label",
        label: user.value.name,
        avatar: user.value.avatar,
      },
    ],
    [
      {
        label: "Profile",
        icon: "i-lucide-user",
      },
      {
        label: "Billing",
        icon: "i-lucide-credit-card",
      },
      {
        label: "Settings",
        icon: "i-lucide-settings",
        to: "/settings",
      },
    ],
    [
      {
        label: "Theme",
        icon: "i-lucide-palette",
        children: [
          {
            label: "Primary",
            slot: "chip",
            chip: appConfig.ui.colors.primary,
            content: {
              align: "center",
              collisionPadding: 16,
            },
            children: colors.map((color) => ({
              label: color,
              chip: color,
              slot: "chip",
              checked: appConfig.ui.colors.primary === color,
              type: "checkbox",
              onSelect: (e) => {
                e.preventDefault();

                appConfig.ui.colors.primary = color;
              },
            })),
          },
          {
            label: "Neutral",
            slot: "chip",
            chip: appConfig.ui.colors.neutral,
            content: {
              align: "end",
              collisionPadding: 16,
            },
            children: neutrals.map((color) => ({
              label: color,
              chip: color,
              slot: "chip",
              type: "checkbox",
              checked: appConfig.ui.colors.neutral === color,
              onSelect: (e) => {
                e.preventDefault();

                appConfig.ui.colors.neutral = color;
              },
            })),
          },
        ],
      },
      {
        label: "Appearance",
        icon: "i-lucide-sun-moon",
        children: [
          {
            label: "System",
            icon: "i-lucide-sun-moon",
            type: "checkbox",
            checked: colorMode.preference === "system",
            onUpdateChecked(checked: boolean) {
              if (checked) {
                colorMode.preference = "system";
              }
            },
            onSelect(e: Event) {
              e.preventDefault();
            },
          },
          {
            label: "Light",
            icon: "i-lucide-sun",
            type: "checkbox",
            checked: colorMode.preference === "light",
            onUpdateChecked(checked: boolean) {
              if (checked) {
                colorMode.preference = "light";
              }
            },
            onSelect(e: Event) {
              e.preventDefault();
            },
          },
          {
            label: "Dark",
            icon: "i-lucide-moon",
            type: "checkbox",
            checked: colorMode.preference === "dark",
            onUpdateChecked(checked: boolean) {
              if (checked) {
                colorMode.preference = "dark";
              }
            },
            onSelect(e: Event) {
              e.preventDefault();
            },
          },
        ],
      },
    ],
    [
      {
        label: "Documentation",
        icon: "i-lucide-book-open",
        to: "https://waypoints.so/docs",
        target: "_blank",
      },
      {
        label: "GitHub repository",
        icon: "i-simple-icons-github",
        to: "https://github.com/waypoints-hq/waypoints",
        target: "_blank",
      },
      {
        label: "Log out",
        icon: "i-lucide-log-out",
        color: "error",
      },
    ],
  ]);

  const collapsed = computed<boolean>(() => props.collapsed === "collapsed");
</script>

<template>
  <UDropdownMenu
    :items="items"
    :content="{ align: 'center', collisionPadding: 12 }"
    :ui="{
      content: collapsed ? 'w-48' : 'w-(--reka-dropdown-menu-trigger-width)',
    }">
    <UButton
      v-bind="{
        ...user,
        label: collapsed ? undefined : user?.name,
        trailingIcon: collapsed ? undefined : 'i-lucide-chevrons-up-down',
      }"
      color="neutral"
      variant="ghost"
      block
      :square="collapsed"
      class="data-[state=open]:bg-elevated"
      :ui="{
        trailingIcon: 'text-dimmed',
      }" />

    <template #chip-leading="{ item }">
      <div class="inline-flex size-5 shrink-0 items-center justify-center">
        <span
          class="ring-bg size-2 rounded-full bg-(--chip-light) ring dark:bg-(--chip-dark)"
          :style="{
            '--chip-light': `var(--color-${(item as any).chip}-500)`,
            '--chip-dark': `var(--color-${(item as any).chip}-400)`,
          }" />
      </div>
    </template>
  </UDropdownMenu>
</template>
