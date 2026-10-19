import { describe, expect, it } from "vitest";
import {
  channelOf,
  compareVersionsDescending,
  DOWNLOADABLE_TARGETS,
  objectKey,
  seriesOf,
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
