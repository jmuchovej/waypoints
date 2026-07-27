import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import * as core from "@actions/core";
import * as github from "@actions/github";
import semver from "semver";
import { parse as parseToml } from "smol-toml";

export type Octokit = ReturnType<typeof github.getOctokit>;

/**
 * Resolve the release tag for a Nightly workflow run.
 *
 * - push (a `v*-nightly.*` tag): the tag already exists → pass it through.
 * - workflow_dispatch: use the supplied tag (empty ⇒ build-only, no release).
 * - schedule: skip when nothing landed since the last nightly; otherwise derive
 *   `v{base}-nightly.{date}` from the on-disk workspace version and create it.
 *
 * The base version comes from the maintained workspace version (what we're
 * building *toward*), stripped of any pre-release suffix — so a nightly built
 * during the `0.2.0` cycle is `v0.2.0-nightly.<date>`.
 *
 * GitHub is reached through the REST API (@actions/github), not shelled-out
 * git — so the job needs only a shallow checkout (for Cargo.toml), and tag
 * creation needs no committer identity or push credentials.
 */
export async function run(): Promise<void> {
  const event = process.env.GITHUB_EVENT_NAME ?? "";

  if (event === "push") {
    return emit(process.env.GITHUB_REF_NAME ?? "", true);
  }

  if (event === "workflow_dispatch") {
    return emit(core.getInput("dispatch-tag"), true);
  }

  // Scheduled run.
  const octokit = github.getOctokit(core.getInput("token", { required: true }));
  const { owner, repo } = github.context.repo;
  const sha = github.context.sha;

  const last = await latestNightlyTag(octokit, owner, repo);
  if (last && (await commitsSince(octokit, owner, repo, last, sha)) === 0) {
    return emit("", false);
  }

  const version = await workspaceVersion();
  const tag = await nextNightlyTag(version, dateStamp(), (candidate) =>
    tagExists(octokit, owner, repo, candidate),
  );
  await createTag(octokit, owner, repo, tag, sha);
  core.info(`Tagged nightly ${tag} at ${sha}.`);
  return emit(tag, true);
}

/** The maintained `[workspace.package].version` — our single source of truth. */
export async function workspaceVersion(root: string = process.cwd()): Promise<string> {
  const raw = await readFile(resolve(root, "Cargo.toml"), "utf8");
  const manifest = parseToml(raw) as { workspace?: { package?: { version?: string } } };
  const version = manifest.workspace?.package?.version;
  if (!version) throw new Error("Cargo.toml has no [workspace.package].version");
  return version;
}

/** Today's UTC date as `YYYYMMDD`. */
export function dateStamp(now: Date = new Date()): string {
  return now.toISOString().slice(0, 10).replace(/-/g, "");
}

/**
 * Derive the next nightly tag: `v{base}-nightly.{stamp}`, where `base` is the
 * version with any pre-release suffix stripped. A same-day collision (e.g.
 * manual + cron) gets `.2`, `.3`, … — a valid, higher-sorting pre-release id.
 *
 * Pure aside from the injected `exists` predicate, so it is unit-testable.
 */
export async function nextNightlyTag(
  version: string,
  stamp: string,
  exists: (tag: string) => Promise<boolean>,
): Promise<string> {
  const parsed = semver.parse(version);
  if (!parsed) throw new Error(`workspace version "${version}" is not valid semver`);

  const base = `${parsed.major}.${parsed.minor}.${parsed.patch}`;
  let tag = `v${base}-nightly.${stamp}`;
  if (await exists(tag)) {
    let n = 2;
    while (await exists(`${tag}.${n}`)) n += 1;
    tag = `${tag}.${n}`;
  }
  return tag;
}

/** The most recent `*-nightly.*` tag, by semver precedence (the date lives in the tag). */
export async function latestNightlyTag(
  octokit: Octokit,
  owner: string,
  repo: string,
): Promise<string | null> {
  const tags = await octokit.paginate(octokit.rest.repos.listTags, { owner, repo, per_page: 100 });
  const [latest] = tags
    .map((tag) => tag.name)
    .filter((name) => /-nightly\./.test(name))
    .sort((a, b) => semver.rcompare(a.replace(/^v/, ""), b.replace(/^v/, "")));
  return latest ?? null;
}

/** Commits on `head` not reachable from `base` — 0 means nothing new landed. */
export async function commitsSince(
  octokit: Octokit,
  owner: string,
  repo: string,
  base: string,
  head: string,
): Promise<number> {
  const { data } = await octokit.rest.repos.compareCommitsWithBasehead({
    owner,
    repo,
    basehead: `${base}...${head}`,
  });
  return data.ahead_by;
}

export async function tagExists(
  octokit: Octokit,
  owner: string,
  repo: string,
  tag: string,
): Promise<boolean> {
  try {
    await octokit.rest.git.getRef({ owner, repo, ref: `tags/${tag}` });
    return true;
  } catch (error) {
    if (isStatus(error, 404)) return false;
    throw error;
  }
}

/** Create an annotated tag object and the ref that points at it. */
export async function createTag(
  octokit: Octokit,
  owner: string,
  repo: string,
  tag: string,
  sha: string,
): Promise<void> {
  const { data: object } = await octokit.rest.git.createTag({
    owner,
    repo,
    tag,
    message: `Nightly ${tag}`,
    object: sha,
    type: "commit",
  });
  await octokit.rest.git.createRef({ owner, repo, ref: `refs/tags/${tag}`, sha: object.sha });
}

function isStatus(error: unknown, status: number): boolean {
  return (
    typeof error === "object" && error !== null && "status" in error && error.status === status
  );
}

function emit(tag: string, shouldBuild: boolean): void {
  core.setOutput("tag", tag);
  core.setOutput("should-build", String(shouldBuild));
  core.info(
    shouldBuild
      ? `Resolved tag: ${tag || "(build-only, no release)"}`
      : "No new commits since the last nightly; skipping tonight's build.",
  );
}
