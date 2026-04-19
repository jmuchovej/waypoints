import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";

export class IPCError extends Error {
  constructor(
    public readonly command: string,
    public readonly detail: string,
    public readonly args?: Record<string, unknown>,
  ) {
    super(`IPC ${command}: ${detail}`);
    this.name = "IPCError";
  }
}

/**
 * The seam between the frontend and a backend. Both halves of the app's native
 * coupling live here: request/response (`invoke`) and push (`listen`). Call
 * sites depend only on this interface, never on a concrete backend, so swapping
 * Tauri for the dev HTTP backend touches nothing but the selection below.
 */
export interface Transport {
  invoke<T>(command: string, payload?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn>;
}

const isTauri = (): boolean => typeof window !== "undefined" && "isTauri" in window;

export const tauriTransport: Transport = {
  async invoke<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
    try {
      return await tauriInvoke<T>(command, payload);
    } catch (e) {
      // Tauri rejects with the raw serialized error value (a string, since
      // cartograph errors serialize as strings) rather than an Error — wrap it
      // so call sites always see a structured IPCError.
      if (e instanceof Error) throw e;
      throw new IPCError(command, String(e), payload);
    }
  },
  listen: (event, handler) => tauriListen(event, (e) => handler(e.payload as never)),
};

// Which Waypoints instance (Tauri flavor) this build belongs to. The server is
// flavor-agnostic and resolves data from this token, so it must match the
// desktop build whose territories you want to see. Defaults to the dev flavor.
const FLAVOR = import.meta.env.VITE_WAYPOINTS_FLAVOR ?? "so.waypoints.dev";

/**
 * Development-only browser transport against the `waypoints-serve` backend.
 * Mirrors Tauri's `invoke()` contract over HTTP (`POST /ipc/{command}`).
 */
export function httpTransport(baseUrl = ""): Transport {
  return {
    async invoke<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
      const res = await fetch(`${baseUrl}/ipc/${command}`, {
        method: "POST",
        headers: { "content-type": "application/json", "x-waypoints-flavor": FLAVOR },
        body: JSON.stringify(payload ?? {}),
      });
      if (!res.ok) {
        throw new IPCError(command, await res.text(), payload);
      }
      return (await res.json()) as T;
    },
    // No server-side event source yet — the only events are native-menu events,
    // which don't exist in a browser. No-op until cartograph emits something;
    // wire SSE here then (one-directional, built-in reconnect), not a WS.
    async listen() {
      return () => {};
    },
  };
}

// Outside Tauri, route through the dev HTTP backend. Gated behind
// `import.meta.dev` so Vite dead-code-eliminates the httpTransport branch from
// production bundles — a shipped app only ever uses the Tauri transport.
const transport: Transport = import.meta.dev && !isTauri() ? httpTransport() : tauriTransport;

export function invoke<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  return transport.invoke<T>(command, payload);
}

export function listen<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return transport.listen<T>(event, handler);
}
