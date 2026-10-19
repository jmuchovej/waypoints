import { describe, expect, it } from "vitest";

import {
  channelOf,
  compareVersionsDescending,
  DOWNLOADABLE_TARGETS,
  latestUrl,
  objectKey,
  seriesOf,
  versionedUrl,
  versionFromKey,
} from "./releases";

describe("channelOf", () => {
  it("classifies a tag's channel by its suffix", () => {
    expect(channelOf("v0.2.0-nightly.3+2026.10.19")).toBe("nightly");
    expect(channelOf("v0.2.0-pre.1")).toBe("preview");
    expect(channelOf("v0.1.1")).toBe("stable");
  });
});

describe("seriesOf", () => {
  it("names the minor series a version belongs to", () => {
    expect(seriesOf("v0.1.1")).toBe("v0.1.x");
    expect(seriesOf("v0.2.0-nightly.3+2026.10.19")).toBe("v0.2.x");
  });
});

describe("objectKey", () => {
  it("matches the uploader's layout exactly", () => {
    expect(objectKey("stable", "v0.1.1", "macos", "arm64", "dmg")).toBe(
      "stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg",
    );
  });

  it("refuses a channel that disagrees with the version's own channel", () => {
    expect(() => objectKey("stable", "v0.2.0-pre.1", "macos", "arm64", "dmg")).toThrow(
      /names channel preview, not stable/,
    );
  });
});

describe("versionFromKey", () => {
  it("round-trips every downloadable target's key back to its version", () => {
    const version = "v0.2.0-nightly.3+2026.10.19";
    for (const target of DOWNLOADABLE_TARGETS) {
      for (const extension of target.extensions) {
        const key = objectKey("nightly", version, target.os, target.arch, extension);
        expect(versionFromKey(key)).toBe(version);
      }
    }
  });

  it("round-trips a macOS updater payload and signature, not only the installer", () => {
    const version = "v0.1.1";
    const payload = objectKey("stable", version, "macos", "arm64", "app.tar.gz");
    const signature = objectKey("stable", version, "macos", "arm64", "app.tar.gz.sig");
    expect(versionFromKey(payload)).toBe(version);
    expect(versionFromKey(signature)).toBe(version);
  });

  it("reports null for a key this scheme did not produce", () => {
    expect(versionFromKey("stable/v0.1.x/macos/arm64/not-ours.dmg")).toBeNull();
  });
});

describe("latestUrl and versionedUrl", () => {
  it("names no version in the alias, and exactly the resolved one in the versioned URL", () => {
    expect(latestUrl("stable", "macos", "arm64", "dmg")).toBe(
      "/api/v1/releases/stable/latest/macos/arm64/waypoints-latest.dmg",
    );
    expect(versionedUrl("stable", "v0.1.1", "macos", "arm64", "dmg")).toBe(
      "/api/v1/releases/stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg",
    );
  });

  it("names a different file than the alias it was resolved from — a download taken through the alias saves under the version, not `latest`", () => {
    const alias = latestUrl("stable", "macos", "arm64", "dmg");
    const resolved = versionedUrl("stable", "v0.1.1", "macos", "arm64", "dmg");
    expect(resolved).not.toBe(alias);
    expect(resolved.endsWith("waypoints-v0.1.1.dmg")).toBe(true);
    expect(alias.endsWith("waypoints-latest.dmg")).toBe(true);
  });
});

describe("compareVersionsDescending", () => {
  it("orders by numeric major/minor/patch rather than lexically", () => {
    const versions = ["v0.1.9", "v0.1.10", "v0.2.0", "v0.1.1"];
    expect([...versions].sort(compareVersionsDescending)).toEqual([
      "v0.2.0",
      "v0.1.10",
      "v0.1.9",
      "v0.1.1",
    ]);
  });
});
