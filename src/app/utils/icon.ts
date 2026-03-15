import type { Icon } from "@waypoints/domain";

/**
 * Convert a domain `Icon.identifier` (or raw string) into a Nuxt UI icon
 * name (the `i-namespace-icon` form consumed by `<UIcon>` and friends).
 *
 * The domain uses hierarchical identifiers like:
 *   - `iconify:heroicons:document-solid`
 *   - `system:emoji:melting-face`
 *   - `local-image:…`, `remote-image:…` (future)
 *
 * Nuxt UI resolves icons via Iconify's flat `i-<set>-<name>` convention,
 * so the mapping for the common cases is a direct rewrite.
 *
 * Returns `undefined` for anything we can't map — callers should decide
 * what fallback (if any) to display.
 */
export function toNuxtIcon(identifier: string | undefined | null): string | undefined {
  if (!identifier) return undefined;
  const [p1, p2, p3, ...rest] = identifier.split(":");
  if (p1 === "iconify" && p2 && p3) {
    // Trailing segments (rare) get joined with "-" to preserve e.g. compound names
    const name = [p3, ...rest].join("-");
    return `i-${p2}-${name}`;
  }
  if (p1 === "system" && p2 === "emoji" && p3) {
    return `i-fluent-emoji-flat-${p3}`;
  }
  return undefined;
}

/**
 * Convenience: take a full domain `Icon` (or null/undefined) and resolve
 * to a Nuxt UI icon name, with an optional fallback.
 */
export function iconName(icon: Icon | null | undefined, fallback?: string): string | undefined {
  return toNuxtIcon(icon?.identifier) ?? fallback;
}
