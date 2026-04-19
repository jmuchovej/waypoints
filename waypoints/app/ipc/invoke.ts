import { invoke as tauriInvoke } from "@tauri-apps/api/core";

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

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await tauriInvoke<T>(cmd, args);
  } catch (e) {
    if (e instanceof Error) throw e;
    throw new IPCError(cmd, String(e), args);
  }
}
