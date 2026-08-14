import { UPDATE_CHANNELS, type UpdateChannel } from "#shared/utils/releases";

export function routeGuard(channel: ComputedRef<UpdateChannel>) {
  if (!UPDATE_CHANNELS.includes(channel.value)) {
    throw createError({ statusCode: 404, statusMessage: "Not Found" });
  }
}

export type ChannelPrefix = "download" | "releases";

/**
 * Deliberately narrower than `ButtonProps`: these links are rendered both by
 * `UPageHero` (which wants `ButtonProps`) and by `UNavigationMenu` (which wants
 * `NavigationMenuItem`), and the two disagree on `type`. Only the three fields
 * below are ever set, and both components accept all of them.
 */
export type ChannelLink = {
  label: string;
  to: string;
  active: boolean;
};

export function buildChannelButtons(prefix: ChannelPrefix, channel: ComputedRef<UpdateChannel>) {
  return ref<ChannelLink[]>([
    {
      label: "Stable",
      to: `/${prefix}/stable`,
      active: channel.value === "stable",
    },
    {
      label: "Preview",
      to: `/${prefix}/preview`,
      active: channel.value === "preview",
    },
    {
      label: "Nightly",
      to: `/${prefix}/nightly`,
      active: channel.value === "nightly",
    },
  ]);
}
