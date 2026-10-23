import { describe, expect, it } from "vitest";

import {
  channelOf,
  compareVersionsDescending,
  DOWNLOADABLE_TARGETS,
  latestUrl,
  objectKey,
  seriesOf,
  TAURI_UPDATER_TARGETS,
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

  it("round-trips every updater payload and signature", () => {
    const version = "v0.2.0-nightly.3+2026.10.19";
    for (const target of Object.values(TAURI_UPDATER_TARGETS)) {
      for (const extension of [target.payload, target.signature]) {
        const key = objectKey("nightly", version, target.os, target.arch, extension);
        expect(versionFromKey(key)).toBe(version);
      }
    }
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

describe("TAURI_UPDATER_TARGETS", () => {
  // An app installed from a deb or rpm asks for its own format's entry and
  // refuses any other payload, so every Linux and Windows installer needs one.
  // macOS's updater installs the `.app.tar.gz`, never the `.dmg`.
  it("gives every Linux and Windows installer an entry that installs it", () => {
    for (const target of DOWNLOADABLE_TARGETS.filter((target) => target.os !== "macos")) {
      const payloads = Object.values(TAURI_UPDATER_TARGETS)
        .filter((entry) => entry.os === target.os && entry.arch === target.arch)
        .map((entry) => entry.payload);
      for (const extension of target.extensions) {
        expect(payloads, `${target.os}/${extension}`).toContain(extension);
      }
    }
  });

  it("serves a deb install the deb, an rpm install the rpm, and the rest the AppImage", () => {
    expect(TAURI_UPDATER_TARGETS["linux-x86_64-deb"]).toMatchObject({
      payload: "deb",
      signature: "deb.sig",
    });
    expect(TAURI_UPDATER_TARGETS["linux-x86_64-rpm"]).toMatchObject({
      payload: "rpm",
      signature: "rpm.sig",
    });
    expect(TAURI_UPDATER_TARGETS["linux-x86_64"]?.payload).toBe("AppImage");
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
