import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Sync the native OS window title bar.
 *
 * No-op on mobile (Android/iOS), which has no title bar and where the Window
 * API throws "Window API not available on mobile" — an unguarded call in
 * `<script setup>` would reject the component's setup. The web document title
 * is handled separately via `useHead`.
 */
export async function setNativeTitle(title: string): Promise<void> {
  try {
    await getCurrentWindow().setTitle(title);
  } catch {
    // Window API unavailable (mobile); the title bar is cosmetic, so ignore.
  }
}
