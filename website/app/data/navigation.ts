import type { NavigationMenuItem } from "@nuxt/ui";

type ResourceLink = {
  icon: string;
  label: string;
  description: string;
  to: string;
  /** Only for off-site links; internal ones leave this unset. */
  target?: "_blank" | "_parent" | "_self" | "_top";
};

type ResourceGroup = {
  label: string;
  items: ResourceLink[];
};

export const resourceGroups: ResourceGroup[] = [
  {
    label: "Product",
    items: [
      {
        icon: "i-lucide-package",
        label: "Releases",
        description: "What's new",
        to: "/releases/stable",
      },
      { icon: "i-lucide-map", label: "Roadmap", description: "What's coming next", to: "/roadmap" },
      {
        icon: "i-lucide-download",
        label: "Download",
        description: "Get the app",
        to: "/download/stable",
      },
    ],
  },
  {
    label: "Company",
    items: [
      {
        icon: "i-lucide-compass",
        label: "Why",
        description: "Learn about Waypoints' vision",
        to: "/why",
      },
    ],
  },
];

/** Flattened version with type labels for vertical UNavigationMenu */
export const resourceLinks = resourceGroups.flatMap((group) => [
  { label: group.label, type: "label" as const },
  ...group.items.map((item) => ({ label: item.label, to: item.to })),
]);

export const menu: NavigationMenuItem[] = [
  {
    label: "Resources",
    value: "resources",
    slot: "resources" as const,
    children: [{ label: "_placeholder" }],
  },
  { label: "Docs", to: "/docs/" },
];
