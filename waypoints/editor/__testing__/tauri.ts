import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import type { InvokeArgs } from "@tauri-apps/api/core";
import { afterEach } from "vitest";

export type IPCHandler = (cmd: string, payload?: InvokeArgs) => unknown;

/**
 * Set up Tauri IPC mocking for a test suite.
 * Call inside a describe() block; automatically clears after each test.
 */
export function useTauriMock() {
  afterEach(() => {
    clearMocks();
  });

  return {
    /** Install an IPC handler for the current test. */
    mock(handler: IPCHandler) {
      mockIPC(handler);
    },

    /** Install a handler that returns canned responses by command name. */
    mockCommands(responses: Record<string, unknown>) {
      mockIPC((cmd) => responses[cmd]);
    },
  };
}
