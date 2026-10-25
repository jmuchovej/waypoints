import { type Channel, isChannel } from "#shared/utils/releases";

export function routeGuard(channel: ComputedRef<Channel>) {
  if (!isChannel(channel.value)) {
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

export function buildChannelButtons(prefix: ChannelPrefix, channel: ComputedRef<Channel>) {
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

export type StoreLink = {
  icon: string;
  title: string;
  description: string;
  to: string;
};

// Per `fastlane/Fastfile`, stable on Play is the only publicly reachable listing: iOS is
// never submitted to the App Store (`skip_submission`), and preview/nightly are internal-track.
export const STORE_LINKS: Record<Channel, StoreLink[]> = {
  stable: [
    {
      icon: "i-simple-icons-googleplay",
      title: "Android",
      description: "Google Play",
      to: "https://play.google.com/store/apps/details?id=so.waypoints",
    },
  ],
  preview: [],
  nightly: [],
};
