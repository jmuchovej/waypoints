import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, test } from "node:test";
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
    assert.equal(dateStamp(new Date("2026-07-26T23:59:59Z")), "20260726");
  });
});

describe("nextNightlyTag", () => {
  test("stamps the base version", async () => {
    assert.equal(await nextNightlyTag("0.2.0", "20260726", never), "v0.2.0-nightly.20260726");
  });

  test("strips a pre-release suffix from the base", async () => {
    assert.equal(await nextNightlyTag("0.2.0-pre.3", "20260726", never), "v0.2.0-nightly.20260726");
  });

  test("disambiguates a same-day collision", async () => {
    const taken = new Set(["v0.2.0-nightly.20260726"]);
    assert.equal(
      await nextNightlyTag("0.2.0", "20260726", async (t) => taken.has(t)),
      "v0.2.0-nightly.20260726.2",
    );
  });

  test("walks past multiple same-day collisions", async () => {
    const taken = new Set(["v0.2.0-nightly.20260726", "v0.2.0-nightly.20260726.2"]);
    assert.equal(
      await nextNightlyTag("0.2.0", "20260726", async (t) => taken.has(t)),
      "v0.2.0-nightly.20260726.3",
    );
  });

  test("rejects a non-semver version", async () => {
    await assert.rejects(nextNightlyTag("not-a-version", "20260726", never), /not valid semver/);
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
    assert.equal(await latestNightlyTag(octokit(tags), "o", "r"), "v0.2.0-nightly.20260726.2");
  });

  test("orders across base versions", async () => {
    const tags = ["v0.9.0-nightly.20260101", "v0.10.0-nightly.20251231"];
    assert.equal(await latestNightlyTag(octokit(tags), "o", "r"), "v0.10.0-nightly.20251231");
  });

  test("returns null when there are no nightly tags", async () => {
    assert.equal(await latestNightlyTag(octokit(["v0.1.0", "v0.2.0-pre.1"]), "o", "r"), null);
  });
});

describe("commitsSince", () => {
  const octokit = (aheadBy: number) =>
    stubOctokit({
      rest: {
        repos: { compareCommitsWithBasehead: async () => ({ data: { ahead_by: aheadBy } }) },
      },
    });

  test("returns ahead_by from the comparison", async () => {
    assert.equal(await commitsSince(octokit(0), "o", "r", "base", "head"), 0);
    assert.equal(await commitsSince(octokit(3), "o", "r", "base", "head"), 3);
  });
});

describe("tagExists", () => {
  test("true when the ref resolves", async () => {
    const octokit = stubOctokit({ rest: { git: { getRef: async () => ({ data: {} }) } } });
    assert.equal(await tagExists(octokit, "o", "r", "v1"), true);
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
    assert.equal(await tagExists(octokit, "o", "r", "v1"), false);
  });

  test("rethrows any non-404 error", async () => {
    const octokit = stubOctokit({
      rest: {
        git: {
          getRef: async () => {
            throw { status: 500 };
          },
        },
      },
    });
    await assert.rejects(tagExists(octokit, "o", "r", "v1"), (error: unknown) => {
      assert.deepEqual(error, { status: 500 });
      return true;
    });
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

    assert.partialDeepStrictEqual(calls.createTag, {
      owner: "o",
      repo: "r",
      tag: "v0.2.0-nightly.20260726",
      object: "COMMITSHA",
      type: "commit",
    });
    // The ref must point at the tag *object*, not the commit.
    assert.partialDeepStrictEqual(calls.createRef, {
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
    assert.equal(await workspaceVersion(dir), "0.3.1");
  });

  test("throws when the version is missing", async () => {
    const dir = await fixture("[workspace]\nmembers = []\n");
    await assert.rejects(workspaceVersion(dir), /no \[workspace.package\].version/);
  });
});
