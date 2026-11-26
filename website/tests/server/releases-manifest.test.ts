// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { beforeAll, describe, expect, it, vi } from "vitest";

type Headers = Record<string, string>;

interface FakeEvent {
  params: Record<string, string>;
  headers: Headers;
  context: { cloudflare: { env: { RELEASES: unknown } } };
}

// The h3 helpers Nitro auto-imports into server code.
beforeAll(() => {
  vi.stubGlobal("defineEventHandler", (handler: unknown) => handler);
  vi.stubGlobal("getRouterParam", (event: FakeEvent, name: string) => event.params[name]);
  vi.stubGlobal("createError", (error: { statusCode: number; statusMessage: string }) =>
    Object.assign(new Error(error.statusMessage), error),
  );
  vi.stubGlobal("setResponseHeaders", (event: FakeEvent, headers: Headers) =>
    Object.assign(event.headers, headers),
  );
  vi.stubGlobal("setResponseHeader", (event: FakeEvent, name: string, value: string) => {
    event.headers[name] = value;
  });
});

/** A bucket holding `objects`, answering `get` the way the R2 binding does. */
function bucket(objects: Record<string, Uint8Array>) {
  return {
    async get(key: string) {
      const bytes = objects[key];
      if (!bytes) return null;
      return {
        size: bytes.byteLength,
        httpEtag: `"etag-${key}"`,
        httpMetadata: {},
        body: new Blob([bytes]).stream(),
        text: async () => new TextDecoder().decode(bytes),
      };
    },
  };
}

function event(channel: string, objects: Record<string, Uint8Array>): FakeEvent {
  return {
    params: { channel },
    headers: {},
    context: { cloudflare: { env: { RELEASES: bucket(objects) } } },
  };
}

async function handler() {
  const module = await import("../../server/routes/api/v1/releases/[channel]/manifest.get");
  return module.default as unknown as (event: FakeEvent) => Promise<ReadableStream<Uint8Array>>;
}

// As xtask writes it: pretty-printed, so a re-serialization would show.
const STORED = new TextEncoder().encode(
  JSON.stringify(
    {
      version: "v0.2.0-nightly.20261109",
      series: "v0.2.x",
      targets: [
        { os: "macos", arch: "arm64", extensions: ["dmg"], digests: { dmg: "ab".repeat(32) } },
      ],
    },
    null,
    2,
  ),
);

describe("GET /api/v1/releases/{channel}/manifest", () => {
  it("streams the stored manifest's bytes unchanged, as JSON", async () => {
    const request = event("nightly", { "nightly.json": STORED });
    const body = await (await handler())(request);
    const served = new Uint8Array(await new Response(body).arrayBuffer());

    expect(served).toEqual(STORED);
    expect(request.headers["Content-Type"]).toBe("application/json");
    expect(request.headers["Content-Length"]).toBe(String(STORED.byteLength));
  });

  it("refuses an unknown channel", async () => {
    await expect((await handler())(event("foo", { "foo.json": STORED }))).rejects.toMatchObject({
      statusCode: 404,
    });
  });

  it("reports a channel with nothing published as missing", async () => {
    await expect((await handler())(event("stable", {}))).rejects.toMatchObject({
      statusCode: 404,
    });
  });
});
