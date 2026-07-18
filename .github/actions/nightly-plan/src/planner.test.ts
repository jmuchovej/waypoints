import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, test } from "bun:test";
import {
  commitsSince,
  createTag,
  dateStamp,
  latestNightlyTag,
  nextNightlyTag,
  type Octokit,
  tagExists,
  workspaceVersion,
} from "./planner.ts";

/** A partial Octokit stub — only the fields a given test exercises. */
function stubOctokit(shape: unknown): Octokit {
  return shape as Octokit;
}

const never = async () => false;

describe("dateStamp", () => {
  test("formats a UTC date as YYYYMMDD", () => {
    expect(dateStamp(new Date("2026-07-26T23:59:59Z"))).toBe("20260726");
  });
});

describe("nextNightlyTag", () => {
  test("stamps the base version", async () => {
    expect(await nextNightlyTag("0.2.0", "20260726", never)).toBe("v0.2.0-nightly.20260726");
  });

  test("strips a pre-release suffix from the base", async () => {
    expect(await nextNightlyTag("0.2.0-pre.3", "20260726", never)).toBe("v0.2.0-nightly.20260726");
  });

  test("disambiguates a same-day collision", async () => {
    const taken = new Set(["v0.2.0-nightly.20260726"]);
    expect(await nextNightlyTag("0.2.0", "20260726", async (t) => taken.has(t))).toBe(
      "v0.2.0-nightly.20260726.2",
    );
  });

  test("walks past multiple same-day collisions", async () => {
    const taken = new Set(["v0.2.0-nightly.20260726", "v0.2.0-nightly.20260726.2"]);
    expect(await nextNightlyTag("0.2.0", "20260726", async (t) => taken.has(t))).toBe(
      "v0.2.0-nightly.20260726.3",
    );
  });

  test("rejects a non-semver version", () => {
    expect(nextNightlyTag("not-a-version", "20260726", never)).rejects.toThrow(/not valid semver/);
  });
});

describe("latestNightlyTag", () => {
  const octokit = (names: string[]) =>
    stubOctokit({
      rest: { repos: { listTags: () => {} } },
      paginate: async () => names.map((name) => ({ name })),
    });

  test("picks the highest nightly by semver, ignoring non-nightly tags", async () => {
    const tags = [
      "v0.2.0-nightly.20260725",
      "v0.1.0",
      "v0.2.0-nightly.20260726",
      "v0.2.0-nightly.20260726.2",
      "v0.2.0-pre.1",
    ];
    expect(await latestNightlyTag(octokit(tags), "o", "r")).toBe("v0.2.0-nightly.20260726.2");
  });

  test("orders across base versions", async () => {
    const tags = ["v0.9.0-nightly.20260101", "v0.10.0-nightly.20251231"];
    expect(await latestNightlyTag(octokit(tags), "o", "r")).toBe("v0.10.0-nightly.20251231");
  });

  test("returns null when there are no nightly tags", async () => {
    expect(await latestNightlyTag(octokit(["v0.1.0", "v0.2.0-pre.1"]), "o", "r")).toBeNull();
  });
});

describe("commitsSince", () => {
  const octokit = (aheadBy: number) =>
    stubOctokit({
      rest: { repos: { compareCommitsWithBasehead: async () => ({ data: { ahead_by: aheadBy } }) } },
    });

  test("returns ahead_by from the comparison", async () => {
    expect(await commitsSince(octokit(0), "o", "r", "base", "head")).toBe(0);
    expect(await commitsSince(octokit(3), "o", "r", "base", "head")).toBe(3);
  });
});

describe("tagExists", () => {
  test("true when the ref resolves", async () => {
    const octokit = stubOctokit({ rest: { git: { getRef: async () => ({ data: {} }) } } });
    expect(await tagExists(octokit, "o", "r", "v1")).toBe(true);
  });

  test("false on a 404", async () => {
    const octokit = stubOctokit({
      rest: {
        git: {
          getRef: async () => {
            throw { status: 404 };
          },
        },
      },
    });
    expect(await tagExists(octokit, "o", "r", "v1")).toBe(false);
  });

  test("rethrows any non-404 error", () => {
    const octokit = stubOctokit({
      rest: {
        git: {
          getRef: async () => {
            throw { status: 500 };
          },
        },
      },
    });
    expect(tagExists(octokit, "o", "r", "v1")).rejects.toEqual({ status: 500 });
  });
});

describe("createTag", () => {
  test("creates an annotated tag object, then a ref pointing at it", async () => {
    const calls: { createTag?: unknown; createRef?: unknown } = {};
    const octokit = stubOctokit({
      rest: {
        git: {
          createTag: async (args: unknown) => {
            calls.createTag = args;
            return { data: { sha: "TAGSHA" } };
          },
          createRef: async (args: unknown) => {
            calls.createRef = args;
            return { data: {} };
          },
        },
      },
    });

    await createTag(octokit, "o", "r", "v0.2.0-nightly.20260726", "COMMITSHA");

    expect(calls.createTag).toMatchObject({
      owner: "o",
      repo: "r",
      tag: "v0.2.0-nightly.20260726",
      object: "COMMITSHA",
      type: "commit",
    });
    // The ref must point at the tag *object*, not the commit.
    expect(calls.createRef).toMatchObject({
      owner: "o",
      repo: "r",
      ref: "refs/tags/v0.2.0-nightly.20260726",
      sha: "TAGSHA",
    });
  });
});

describe("workspaceVersion", () => {
  async function fixture(contents: string): Promise<string> {
    const dir = await mkdtemp(join(tmpdir(), "nightly-plan-"));
    await writeFile(join(dir, "Cargo.toml"), contents);
    return dir;
  }

  test("reads [workspace.package].version", async () => {
    const dir = await fixture('[workspace.package]\nversion = "0.3.1"\n');
    expect(await workspaceVersion(dir)).toBe("0.3.1");
  });

  test("throws when the version is missing", async () => {
    const dir = await fixture("[workspace]\nmembers = []\n");
    expect(workspaceVersion(dir)).rejects.toThrow(/no \[workspace.package\].version/);
  });
});
