//! Step constructors shared across workflows. Each returns a labelled step
//! ready to drop into a job.

use super::{
    Artifact, Channel, Platform, TestSuite,
    helpers::{
        self, CI_CACHE_BUCKET, CiScope, GITHUB_TOKEN, Resolution, WP_BOTSWAIN_APP_ID,
        WP_BOTSWAIN_PRIVATE_KEY,
    },
};
use gh_workflow::{Expression, FluentBuilder, JobOutput, Run, Step, StepOutput, Use};
use indoc::indoc;

/// A `uses:` reference — `{owner}/{repo}@{sha}`. Pinning to the hash rather
/// than a tag means a retagged (compromised) release cannot reach CI on its
/// own. Each call site carries a `// {version}` comment beside the sha naming
/// the release it was resolved from; the custom manager in
/// `.github/renovate.json5` matches the pair and moves them together.
pub struct Action {
    owner: &'static str,
    repo: &'static str,
    sha: &'static str,
}

impl Action {
    pub const fn new(owner: &'static str, repo: &'static str, sha: &'static str) -> Self {
        Self { owner, repo, sha }
    }

    /// A `Step<Use>` invoking this action, labelled `name`.
    pub fn step(&self, name: impl ToString) -> Step<Use> {
        Step::new(name).uses(self.owner, self.repo, self.sha)
    }
}

impl From<Action> for Step<Use> {
    fn from(a: Action) -> Self {
        Step::new(a.repo).uses(a.owner, a.repo, a.sha)
    }
}

pub fn checkout() -> Step<Use> {
    Action::new(
        "actions",
        "checkout",
        "d23441a48e516b6c34aea4fa41551a30e30af803", // v6
    )
    .step("Checkout")
    .with(("persist-credentials", false))
    .id("checkout")
}

pub fn install_nix() -> Step<Use> {
    Action::new(
        "cachix",
        "install-nix-action",
        "13d8dd58da0234aa297dedd986986ccb8e7f3e24", // v31
    )
    .step("Install Nix")
    .id("setup-nix")
}

/// Reads (and, given a token, writes) the named Cachix binary cache.
pub fn cachix(cache: &str) -> Step<Use> {
    Action::new(
        "cachix",
        "cachix-action",
        "5f2d7c5294214f71b873db4b969586b980625e71", // v17
    )
    .step(format!("Use the {cache} binary cache"))
    .id(format!("setup-{cache}-cachix"))
    .add_with(("name", cache))
}

/// Installs flake outputs into the ambient profile, for tools needed *before*
/// the devenv shell can be entered.
pub fn nix_profile_install(packages: &[&str]) -> Step<Run> {
    let refs = packages
        .iter()
        .map(|package| format!("nixpkgs#{package}"))
        .collect::<Vec<_>>()
        .join(" ");

    Step::new(format!("Install {}", packages.join(" and ")))
        .run(format!("nix profile install {refs}"))
        .shell("bash")
        .id("setup-nix-profile")
}

/// Runs a `just` recipe, the way it runs locally: inside the devenv shell.
///
/// The wrapper is not a convenience. `just` resolves every `mod` file at *parse*
/// time, and `src/justfile` opens with `env("TAURI_APP_PATH")`, so a bare
/// `just <recipe>` dies before any recipe body executes — however carefully that
/// body shells into devenv itself. The failure names the module, not the recipe,
/// which is what hid it: `check-generated` was red from 2026-09-25 on this
/// alone, so the check that would have caught stale generated files was itself
/// the stale one.
pub fn just(recipe: &str) -> Step<Run> {
    Step::new(format!("just {recipe}"))
        .run(format!("devenv shell -- just {recipe}"))
        .shell("bash")
        .id(recipe)
}

/// Runs an `xtask` subcommand via the `cargo xtask` alias in
/// `.cargo/config.toml`.
///
/// Values are quoted: several carry a `${{ }}` expression, which is substituted
/// before the shell sees the line.
pub fn cargo_xtask(subcommand: &str, args: &[(&str, Option<&str>)]) -> Step<Run> {
    let command = args.iter().fold(
        format!("cargo xtask {subcommand}"),
        |command, (flag, value)| {
            if let Some(arg) = value {
                format!("{command} {flag} \"{arg}\"")
            } else {
                format!("{command} {flag}")
            }
        },
    );

    Step::new(format!("cargo xtask {subcommand}"))
        .run(command)
        .shell("bash")
}

/// Refreshes the pinned Nix inputs, then rebuilds everything downstream of
/// them.
///
/// `devenv update` moves `devenv.lock`, which moves tool versions, which moves
/// devenv's own generated files — and, because `WorkflowContext` reads the
/// Android NDK and JDK versions back out of devenv, the workflows too. Entering
/// the shell is what regenerates devenv's outputs, so the `cargo xtask` call
/// covers both halves. Committing the lockfile alone would open a pull request
/// that fails its own `check-generated`.
pub fn devenv_update() -> Step<Run> {
    let cmd = indoc! {r#"
        devenv update
        devenv shell cargo xtask workflows
    "#}
    .trim_end();

    Step::new("Update devenv inputs")
        .run(cmd)
        .shell("bash")
        .add_env(("SECRETSPEC_REASON", "devenv update refreshes locked inputs and regenerates devenv- and xtask-managed files; no secret values are read"))
        .id("devenv-update")
}

/// Mints a short-lived installation token for the `wp-botswain` GitHub App,
/// narrowed to `permissions` — pairs of a `permission-<name>` suffix and the
/// level to grant it.
///
/// Automation authenticates as the App rather than through `GITHUB_TOKEN`
/// because a pull request opened by `GITHUB_TOKEN` deliberately raises no
/// `pull_request` event — the checks that justify the pull request would never
/// run on it. Scoping is not optional: an unnarrowed token inherits the whole
/// installation's grant, which `zizmor` fails the workflow over.
pub fn app_token(permissions: &[(&str, &str)]) -> Step<Use> {
    permissions.iter().fold(
        Action::new(
            "actions",
            "create-github-app-token",
            "bcd2ba49218906704ab6c1aa796996da409d3eb1", // v3.2.0
        )
        .step("Mint a wp-botswain token")
        .id("app-token")
        .add_with(("app-id", WP_BOTSWAIN_APP_ID))
        .add_with(("private-key", WP_BOTSWAIN_PRIVATE_KEY)),
        |step, (scope, level)| step.add_with((format!("permission-{scope}"), *level)),
    )
}

/// Opens a pull request from whatever the preceding steps left in the tree,
/// restricted to `paths`.
///
/// `branch` is reused across runs, so a still-unmerged update is refreshed in
/// place instead of stacking a second pull request behind the first. Commits go
/// through the API (`sign-commits`), which is what makes them show up verified
/// rather than as an unsigned push from a bot.
pub fn create_pull_request(
    token: &StepOutput,
    branch: &str,
    title: &str,
    body: &str,
    paths: &[String],
) -> Step<Use> {
    Action::new(
        "peter-evans",
        "create-pull-request",
        "5f6978faf089d4d20b00c7766989d076bb2fc7f1", // v8.1.1
    )
    .step("Open a pull request")
    .id("create-pull-request")
    .add_with(("token", token.to_string()))
    .add_with(("branch", branch))
    .add_with(("delete-branch", true))
    .add_with(("sign-commits", true))
    .add_with(("title", title))
    .add_with(("body", body))
    .add_with(("commit-message", title))
    .add_with(("add-paths", paths.join("\n")))
}

/// Installs the whole toolchain — node, pnpm, just, sops, age, yq, nu, and
/// rust with every cross-compilation target, read from `rust-toolchain.toml`.
///
/// Pinned to the mise devenv provides — the one that writes
/// `.config/mise.lock`. Unpinned, the action installs whatever released most
/// recently, so a committed lockfile is only as reproducible as an upstream
/// release schedule.
pub fn setup_mise(with_cache: Option<bool>, version: &str) -> Step<Use> {
    Action::new(
        "jdx",
        "mise-action",
        "c2a87611a18de5b3828c5652fe268e992400cb5c", // v4.3.0
    )
    .step("Setup mise")
    .add_with(("version", version.to_owned()))
    .add_with(("cache", with_cache.unwrap_or(true)))
}

/// Restores compiled registry dependencies between runs.
///
/// Written only from `main`: caches are branch-scoped with fallback to the
/// default branch, so one warm entry there serves every branch.
pub(crate) fn rust_cache() -> Step<Use> {
    Action::new(
        "Swatinem",
        "rust-cache",
        "6323deb102c322ba6fcbdcafc7e3dddab59af2b6", // v2
    )
    .step("Cache Rust dependencies")
    .add_with(("save-if", "${{ github.ref == 'refs/heads/main' }}"))
}

pub fn pnpm_install() -> Step<Run> {
    Step::new("Install JS deps")
        .run("pnpm install --frozen-lockfile")
        .shell("bash")
        .id("pnpm-install")
}

/// Reclaims preinstalled toolchains the Ubuntu runner image ships and this
/// workspace never uses. A debug test build of the whole workspace is large
/// enough to exhaust a stock image part-way through linking, which surfaces as
/// `No space left on device` from whichever crate happened to be writing.
///
/// `drop_android` is a parameter rather than an assumption: the Android build
/// job also runs on Linux, and the runtime this removes is exactly what it
/// needs. Callers that build for Android pass `false`.
///
/// Every input is set explicitly rather than left to the action's defaults, so
/// bumping the pin cannot change what gets deleted without the diff saying so.
/// The two that are off:
///
/// - `preinstalled-runtimes` removes Node, Go, Python and Ruby, which would
///   strand any `setup-*` action on a re-download and possibly a version that
///   is no longer published.
/// - `swap-storage` removes the swapfile; upstream warns that jobs needing swap
///   are then killed under memory pressure with no error naming the cause,
///   which is a poor trade against Rust link steps.
///
/// `large-packages` costs a couple of minutes of apt removals. Worth it while
/// the alternative is a job that cannot finish.
pub fn free_disk_space(drop_android: bool) -> Step<Use> {
    Action::new(
        "jlumbroso",
        "free-disk-space",
        "ceedf095f4ec1a097402bc6bd80831f2e1a6fde6", // v2.0.0
    )
    .step("Free runner disk space")
    .add_with(("android", drop_android.to_string()))
    .add_with(("dotnet", "true"))
    .add_with(("haskell", "true"))
    .add_with(("large-packages", "true"))
    .add_with(("docker-images", "true"))
    .add_with(("preinstalled-runtimes", "false"))
    .add_with(("swap-storage", "false"))
    .id("free-disk")
}

/// Tauri's Linux backend is WebKitGTK, which the runner image does not carry.
pub fn install_linux_deps() -> Step<Run> {
    Step::new("Install system deps (Linux)")
        .run(
            indoc! {"
            sudo apt-get update
            sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
        "}
            .trim_end(),
        )
        .shell("bash")
        .id("linux-deps")
}

pub fn setup_java(major: &str) -> Step<Use> {
    Action::new(
        "actions",
        "setup-java",
        "b6effb05e454b25005698d916606bdc6ffcbf961", // v5
    )
    .step("Setup Java")
    .add_with(("distribution", "zulu"))
    .add_with(("java-version", major))
    .id("setup-java")
}

pub fn setup_android_sdk() -> Step<Use> {
    Action::new(
        "android-actions",
        "setup-android",
        "40fd30fb8d7440372e1316f5d1809ec01dcd3699", // v4
    )
    .step("Setup Android SDK")
    .add_with(("accept-android-sdk-licenses", "true"))
    .add_with(("log-accepted-android-sdk-licenses", "true"))
    .add_with(("packages", "platform-tools"))
    .id("setup-android-sdk")
}

pub fn install_ndk(version: &str) -> Step<Run> {
    Step::new("Install Android NDK")
        .run(format!("sdkmanager \"ndk;{version}\""))
        .id("setup-android-ndk")
}

/// Builds via the same `just` recipe used locally.
///
/// That recipe wraps the build in `secretspec run`, so signing material reaches
/// only the build process and never `$GITHUB_ENV`, which later steps inherit.
/// CI supplies just the credential that unlocks the vault, through the
/// environment — the only channel that reaches a `secretspec` call inside a
/// justfile.
pub fn build(channel: Channel, platform: Platform) -> Step<Run> {
    let step = Step::new(format!("Build {platform}"))
        .run(format!(
            "just tauri::build {slug} {flavor}",
            slug = platform.slug(),
            flavor = channel.flavor(),
        ))
        .shell("bash")
        .id("build-recipe");
    helpers::resolving(step)
}

/// Starts the sccache server for the job, holding the store credential.
///
/// The server is what reads and writes the bucket; the client cargo invokes as
/// its rustc wrapper only talks to it over a local port. Starting it here, under
/// a `secretspec run` of the cache scope, is what lets the build itself run
/// with no credential at all — see [`with_sccache`]. The store coordinates are
/// configuration and sit on the step where a reader can see them; the endpoint
/// and key pair arrive through the wrap and are visible to nothing else.
///
/// A failed start is tolerated: sccache 0.17 refuses to start when its storage
/// check fails, and a cache outage must cost time, not the release. With no
/// server listening, the first `rustc` auto-starts one from the build's
/// credential-free environment, which caches to local disk — the stats step
/// shows `Cache location  Local disk` instead of the bucket, which is how a
/// degraded run is told from a healthy one. `SCCACHE_IDLE_TIMEOUT=0` keeps the
/// server alive through the steps between here and the first compile; the
/// stats step stops it.
pub(crate) fn sccache_start() -> Step<Run> {
    // Single quotes: an escaped double quote inside a YAML scalar is rewritten
    // by the formatter, which would leave the generated file never in sync.
    let spec = SecretSpec::run(concat!(
        "sccache --start-server || echo ",
        "'::warning::sccache server failed to start; this build compiles without the remote cache'",
    ))
    .resolving(Resolution::Ci(CiScope::Cache))
    .with_reason(concat!(
        "the compiler cache server reads and writes the ci-cache bucket for the length ",
        "of the job; it alone holds the credential",
    ));

    Into::<Step<Run>>::into(spec)
        .name("Start the compiler cache")
        .id("sccache-start")
        .add_env(("SCCACHE_BUCKET", CI_CACHE_BUCKET))
        .add_env(("SCCACHE_S3_KEY_PREFIX", "sccache/"))
        .add_env(("SCCACHE_REGION", "auto"))
        .add_env(("SCCACHE_IDLE_TIMEOUT", "0"))
}

/// Routes the build's compilation through the sccache server [`sccache_start`]
/// left running. The only place the wrapper is enabled, so a job cannot wrap
/// the compiler without the server start the guard looks for.
///
/// The wrapper name is all the client needs: it carries no store configuration.
/// Cargo reads it as `RUSTC_WRAPPER` or `CARGO_BUILD_RUSTC_WRAPPER`; the
/// `CARGO_` spelling is the one the Tauri CLI forwards into the cargo Xcode
/// runs for iOS, which inherits nothing else of this step's environment. The
/// client's other setting — compile directly if the server goes away, rather
/// than fail — is `SCCACHE_IGNORE_SERVER_IO_ERROR` in `.config/cargo.toml`'s
/// `[env]`, which reaches every rustc and build-script process on that path
/// too.
pub(crate) fn with_sccache(step: Step<Run>) -> Step<Run> {
    step.add_env(("CARGO_BUILD_RUSTC_WRAPPER", "sccache"))
}

/// Prints what the compiler cache did — hits, misses, errors, and the store it
/// used — then stops the server so the credential it held is gone before the
/// upload steps run. Unconditional: a build that failed because of the store is
/// exactly the one whose report is wanted. Needs no credential; both commands
/// talk to the local port.
pub(crate) fn sccache_stats() -> Step<Run> {
    Step::new("Report the compiler cache")
        .run(
            indoc! {"
            sccache --show-stats
            sccache --stop-server
        "}
            .trim_end(),
        )
        .shell("bash")
        .if_condition(Expression::new("always()"))
        .id("sccache-stats")
}

/// What a directory snapshot's key hashes.
pub(crate) enum CacheKey {
    /// Committed files that pin the directory's contents, hashed by the runner
    /// with `hashFiles()` so the key is computed identically on every platform
    /// and never baked into the YAML.
    Files(&'static [&'static str]),
    /// A version string, for a directory pinned by a declared version rather
    /// than a manifest.
    Literal(String),
}

/// A directory the build would otherwise fetch, snapshotted into the store as
/// `dirs/<key>.tar.gz` under a key derived from what pins its contents. An
/// entry is written once and never overwritten, so a key names one immutable
/// snapshot — a faithful stand-in for the fetch only because every manifest
/// here carries content hashes (pnpm's integrity fields, `Cargo.lock`'s
/// checksums, Gradle's verification metadata, `sdkmanager`'s package digests).
pub(crate) struct CachedDir {
    /// Names the step ids (`cache-restore-<name>`, `cache-save-<name>`) and
    /// leads the key.
    pub name: &'static str,
    /// Where the directory lives on the runner, as a bash expression.
    pub path: String,
    pub key: CacheKey,
}

impl CachedDir {
    /// `<name>-<os>-<arch>-<hash or version>`: an object name in the bucket.
    fn key(&self) -> String {
        let pin = match &self.key {
            CacheKey::Files(files) => {
                let quoted = files
                    .iter()
                    .map(|file| format!("'{file}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("${{{{ hashFiles({quoted}) }}}}")
            }
            CacheKey::Literal(version) => version.clone(),
        };
        format!(
            "{name}-${{{{ runner.os }}}}-${{{{ runner.arch }}}}-{pin}",
            name = self.name
        )
    }

    fn restore_id(&self) -> String {
        format!("cache-restore-{}", self.name)
    }
}

/// The directories a `platform` build caches, keyed by the manifests that pin
/// them. The pnpm store is pinned to `$RUNNER_TEMP` by the install step (see
/// `jobs::build`) so its location is one expression on every platform.
pub(crate) fn cached_dirs(platform: Platform, ndk_version: &str) -> Vec<CachedDir> {
    let mut dirs = vec![
        CachedDir {
            name: "pnpm-store",
            path: "${RUNNER_TEMP}/pnpm-store".to_owned(),
            key: CacheKey::Files(&["pnpm-lock.yaml"]),
        },
        CachedDir {
            name: "cargo-registry",
            path: "${CARGO_HOME:-$HOME/.cargo}/registry".to_owned(),
            key: CacheKey::Files(&["Cargo.lock"]),
        },
    ];
    if platform.is_android() {
        dirs.push(CachedDir {
            name: "gradle",
            path: "${HOME}/.gradle/caches/modules-2".to_owned(),
            key: CacheKey::Files(&[
                "src-tauri/gen/android/gradle/verification-metadata.xml",
                "src-tauri/gen/android/gradle/libs.versions.toml",
                "src-tauri/gen/android/gradle/wrapper/gradle-wrapper.properties",
                "src-tauri/gen/android/settings.gradle",
                "src-tauri/gen/android/build.gradle.kts",
                "src-tauri/gen/android/app/build.gradle.kts",
            ]),
        });
        dirs.push(CachedDir {
            name: "android-ndk",
            path: format!("${{ANDROID_HOME}}/ndk/{ndk_version}"),
            key: CacheKey::Literal(ndk_version.to_owned()),
        });
    }
    dirs
}

/// The one line in each cache script that contacts the store, under a
/// `secretspec run` of the cache scope so the key pair exists only for the
/// `rclone` process. The endpoint travels as `RCLONE_S3_ENDPOINT` inside that
/// shell — an unquoted URL inside the connection string would be cut at the
/// `:` of `https://`. Takes the source and destination as positional
/// arguments so nothing is re-quoted through two shells.
///
/// `--s3-no-check-bucket`: the token is scoped to the one bucket and cannot
/// list buckets, which is what rclone's pre-upload existence check does; it
/// then falls through to `CreateBucket`, which the token refuses too. Skipping
/// the check is what lets a bucket-scoped token write at all.
const RCLONE_COPY: &str = concat!(
    "secretspec run --profile ci --scope cache -- ",
    "bash -c 'RCLONE_S3_ENDPOINT=\"$SCCACHE_ENDPOINT\" exec rclone copyto --s3-no-check-bucket \"$@\"' rclone",
);

/// Restores `dir` from the store, publishing `hit` for the save step. A miss or
/// a store failure logs and exits 0 — the consumer then fetches as it would
/// without a cache.
///
/// The path is spliced into the script rather than carried on the step `env`:
/// it is a shell expression (`${RUNNER_TEMP}`, `${CARGO_HOME:-$HOME/.cargo}`),
/// and an `env` value reaches the shell as a literal, never expanded.
pub(crate) fn cache_restore(dir: &CachedDir) -> Step<Run> {
    let script = indoc! {r#"
        path="{dir}"
        archive="${RUNNER_TEMP}/${CACHE_KEY}.tar.gz"
        remote=":s3,provider=Cloudflare,env_auth=true:{bucket}/dirs/${CACHE_KEY}.tar.gz"
        if {rclone} "${remote}" "${archive}" \
          && mkdir -p "$(dirname "${path}")" \
          && tar -xzf "${archive}" -C "$(dirname "${path}")"; then
          echo "hit=true" >> "${GITHUB_OUTPUT}"
          echo "restored ${CACHE_KEY} into ${path}"
        else
          echo "hit=false" >> "${GITHUB_OUTPUT}"
          echo "no entry for ${CACHE_KEY}; ${path} will be fetched"
        fi
        rm -f "${archive}"
    "#}
    .trim_end()
    .replace("{dir}", &dir.path)
    .replace("{bucket}", CI_CACHE_BUCKET)
    .replace("{rclone}", RCLONE_COPY);

    let step = Step::new(format!("Restore {}", dir.name))
        .run(script)
        .shell("bash")
        .add_env(("CACHE_KEY", dir.key()))
        .add_env((
            "SECRETSPEC_REASON",
            format!(
                "restoring the {} snapshot from the ci-cache bucket; the store client alone holds the credential",
                dir.name
            ),
        ))
        .id(dir.restore_id());
    helpers::resolving(step)
}

/// Saves `dir` to the store under its key when the restore missed. Write-once:
/// `--ignore-existing` leaves an entry that appeared in the meantime alone, so
/// a key always names the first snapshot written under it. Runs after the
/// consumer even if the build failed — what was fetched is still what the
/// manifest pins — but never on a cancelled job, and never fails the job.
pub(crate) fn cache_save(dir: &CachedDir) -> Step<Run> {
    let script = indoc! {r#"
        path="{dir}"
        if [ ! -d "${path}" ]; then
          echo "nothing at ${path} to save"
          exit 0
        fi
        archive="${RUNNER_TEMP}/${CACHE_KEY}.tar.gz"
        remote=":s3,provider=Cloudflare,env_auth=true:{bucket}/dirs/${CACHE_KEY}.tar.gz"
        if tar -czf "${archive}" -C "$(dirname "${path}")" "$(basename "${path}")" \
          && {rclone} --ignore-existing "${archive}" "${remote}"; then
          echo "saved ${CACHE_KEY}"
        else
          echo "::warning::could not save ${CACHE_KEY}"
        fi
        rm -f "${archive}"
    "#}
    .trim_end()
    .replace("{dir}", &dir.path)
    .replace("{bucket}", CI_CACHE_BUCKET)
    .replace("{rclone}", RCLONE_COPY);

    let step = Step::new(format!("Save {}", dir.name))
        .run(script)
        .shell("bash")
        .if_condition(Expression::new(format!(
            "!cancelled() && steps.{restore}.outputs.hit != 'true'",
            restore = dir.restore_id()
        )))
        .add_env(("CACHE_KEY", dir.key()))
        .add_env((
            "SECRETSPEC_REASON",
            format!(
                "saving the {} snapshot to the ci-cache bucket; the store client alone holds the credential",
                dir.name
            ),
        ))
        .id(format!("cache-save-{}", dir.name));
    helpers::resolving(step)
}

/// Runs a fastlane lane with the platform's secrets injected for its lifetime.
///
/// Gated on the tag like [`upload_to_release`]: a build-only run has none,
/// and its artifacts must stop at the workflow — the store build number is
/// the date, so a dispatch that reached the store would take the day's slot
/// from the scheduled run.
pub fn fastlane(platform: Platform, channel: Channel, tag: Option<&JobOutput>) -> Step<Run> {
    let spec = SecretSpec::run(format!(
        "fastlane {lane} {flavor}",
        lane = platform.slug(),
        flavor = channel.flavor()
    ))
    .resolving(Resolution::Release(platform));

    Into::<Step<Run>>::into(spec)
        .when(platform.is_android(), |step| {
            step.name("Upload to the Play Store")
        })
        .when(platform.is_ios(), |step| {
            step.name("Upload to the App Store")
        })
        .if_condition(Channel::tag_condition(tag))
        .id("publish-mobile-app")
}

/// Uploads everything the platform's build produced, under a name naming both
/// the platform and the channel.
pub fn upload_artifact(channel: Channel, platform: Platform) -> Step<Use> {
    Action::new(
        "actions",
        "upload-artifact",
        "ea165f8d65b6e75b540449e92b4886f43607fa02", // v4
    )
    .step("Upload artifacts")
    .add_with(("name", Artifact(channel, platform).to_string()))
    .add_with(("path", platform.artifacts()))
    .add_with(("if-no-files-found", "error"))
    .id("upload-artifacts")
}

/// Renders the notes for `tag` with git-cliff: everything since the previous
/// tag, grouped straight from the Conventional Commit history.
///
/// The token is for the template's first-time-contributor section, which
/// asks the GitHub API who opened each pull request; without it git-cliff
/// gets a 404 from the private repository and renders nothing.
pub fn release_notes(tag: Option<&JobOutput>) -> Step<Run> {
    Step::new("Generate release notes")
        .run(r#"git-cliff --latest --output "${RUNNER_TEMP}/notes.md""#)
        .shell("bash")
        .if_condition(Channel::tag_condition(tag))
        .add_env(("GITHUB_TOKEN", GITHUB_TOKEN))
        .id("gen-release-notes")
}

/// Drafts the release for `tag`, carrying the notes from [`release_notes`].
///
/// A draft: the platform builds upload into it, and it goes public only when
/// [`publish_release`] flips it — so a failed build never leaves a
/// half-populated release visible. The `view` guard makes a re-run reuse the
/// draft it left behind; `gh` resolves drafts by tag name.
pub fn create_release(channel: Channel, tag: Option<&JobOutput>) -> Step<Run> {
    let view = r#"gh release view "${TAG}" >/dev/null 2>&1"#;
    let create =
        r#"gh release create "${TAG}" --draft --verify-tag --notes-file "${RUNNER_TEMP}/notes.md""#;
    let create = format!(
        "{view} \\\n|| {create}{prerelease}",
        prerelease = if channel.prerelease() {
            " --prerelease"
        } else {
            ""
        },
    );

    Step::new("Create the draft")
        .run(create)
        .shell("bash")
        .if_condition(Channel::tag_condition(tag))
        .add_env(("GH_TOKEN", GITHUB_TOKEN))
        .add_env(("TAG", Channel::tag(tag)))
        .id("draft-release")
}

/// Uploads the platform's artifacts to the release for `tag`. Skipped entirely
/// on a build-only run, where the tag is empty.
///
/// `tag` is the planner's output on nightly, and `None` elsewhere — where the
/// tag comes from the pushed ref or the dispatch input instead.
pub fn upload_to_release(platform: Platform, tag: Option<&JobOutput>) -> Step<Run> {
    let globs = platform
        .artifacts()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    Step::new("Upload to GitHub Release")
        .run(format!(r#"gh release upload "${{TAG}}" {globs} --clobber"#))
        .shell("bash")
        .if_condition(Channel::tag_condition(tag))
        .add_env(("GH_TOKEN", GITHUB_TOKEN))
        .add_env(("TAG", Channel::tag(tag)))
        .id("upload-release")
}

/// Flips the draft public. When this runs is the publish job's decision.
pub fn publish_release(tag: Option<&JobOutput>) -> Step<Run> {
    Step::new("Publish the release")
        .run(r#"gh release edit "${TAG}" --draft=false"#)
        .shell("bash")
        .add_env(("GH_TOKEN", GITHUB_TOKEN))
        .add_env(("TAG", Channel::tag(tag)))
        .id("publish-release")
}

pub(crate) struct Nextest(Step<Run>);

pub(crate) fn cargo_nextest(suite: TestSuite) -> Nextest {
    let step = Step::new(format!("Run {} tests", suite.describe()))
        .run("cargo nextest run --workspace --no-fail-fast")
        .shell("bash")
        // The release suite compiles the whole workspace cold, and the hosted
        // Linux runner has run out of disk doing it. Debuginfo and incremental
        // artifacts are most of `target/`, and neither is worth anything here.
        .add_env(("CARGO_INCREMENTAL", "0"))
        .add_env(("CARGO_PROFILE_DEV_DEBUG", "0"))
        .id("nextest");
    Nextest(step)
}

impl Nextest {
    /// Adds the filterset, and the guard that turns an empty one into a skip
    /// rather than a failure.
    ///
    /// `cargo nextest -E ""` exits 94 with `failed to parse filterset`, while an
    /// empty plan is the *documented* answer for a change touching no Rust crate
    /// — so the step has to not run, rather than run with nothing. Any push of
    /// only docs, configs or manifests hits this. Bound here rather than at the
    /// call site so a filterset cannot be added without its guard.
    pub(crate) fn with_filter_expr(mut self, filter_expr: &str) -> Self {
        if let Some(command) = self.0.value.run.as_mut() {
            command.push_str(r#" -E "${FILTER_EXPR}""#);
        }
        // `if:` is already an expression context, so the `${{ }}` wrapper the
        // env value needs is a syntax error there.
        let unwrapped = filter_expr
            .trim()
            .trim_start_matches("${{")
            .trim_end_matches("}}")
            .trim();
        self.0 = self
            .0
            .add_env(("FILTER_EXPR", filter_expr))
            .if_condition(Expression::new(format!("{unwrapped} != ''")));
        self
    }
}

impl From<Nextest> for Step<Run> {
    fn from(value: Nextest) -> Self {
        value.0
    }
}

/// How a step hands the secrets it resolves to whatever consumes them.
#[derive(Clone)]
enum Delivery {
    /// `secretspec run -- <cmd>`. The values exist only for that child process,
    /// which is what keeps signing material out of `$GITHUB_ENV` and so out of
    /// every later step in the job.
    Wrap(String),
    /// `secretspec export --format gha`. The values land in `$GITHUB_ENV`, so
    /// later steps read them as `${{ env.NAME }}` — the only route that reaches
    /// an action's `with:` block, which cannot run a command of its own.
    ///
    /// Multi-line values survive intact: the CLI writes them under a random
    /// heredoc delimiter and emits one `::add-mask::` per line, a single mask
    /// over a multi-line value redacting nothing.
    Export,
}

#[derive(Clone)]
pub(crate) struct SecretSpec {
    resolution: Option<Resolution>,
    reason: Option<String>,
    delivery: Delivery,
    inside_devenv: bool,
}

impl SecretSpec {
    /// Wraps `cmd`, so the secrets never outlive it. Prefer this wherever the
    /// consumer is a command; [`SecretSpec::export`] widens the blast radius to
    /// the rest of the job.
    pub(crate) fn run(cmd: impl ToString) -> Self {
        Self::deliver(Delivery::Wrap(cmd.to_string()))
    }

    /// Publishes the resolved secrets into the job environment.
    pub(crate) fn export() -> Self {
        Self::deliver(Delivery::Export)
    }

    fn deliver(delivery: Delivery) -> Self {
        Self {
            resolution: None,
            reason: None,
            delivery,
            inside_devenv: false,
        }
    }

    /// What to resolve: a profile and the scope within it, as one value. A
    /// CI-facing profile is never resolved whole, and this is the only way to
    /// name one, so the generator cannot emit `--profile` without `--scope`.
    pub(crate) fn resolving(mut self, resolution: Resolution) -> Self {
        self.resolution = Some(resolution);
        self
    }

    /// Runs through `devenv shell`, for a job whose `secretspec` and `op` come
    /// from the shell rather than from mise. The provider then travels on the
    /// command line rather than in the step environment: devenv resolves the
    /// shell's own profile before entering the shell and would otherwise pick
    /// the override up and look for `op` on the bare runner.
    pub(crate) fn inside_devenv(mut self) -> Self {
        self.inside_devenv = true;
        self
    }

    /// Recorded by providers that keep an audit log, and demanded outright by
    /// secretspec's `require_reason` policy.
    pub(crate) fn with_reason(mut self, reason: impl ToString) -> Self {
        self.reason = Some(reason.to_string());
        self
    }

    #[allow(dead_code)]
    pub(crate) fn with_cmd(mut self, cmd: impl ToString) -> Self {
        self.delivery = Delivery::Wrap(cmd.to_string());
        self
    }
}

impl From<SecretSpec> for Step<Run> {
    fn from(spec: SecretSpec) -> Self {
        let mut command = String::from(match spec.delivery {
            Delivery::Wrap(_) => "secretspec run",
            Delivery::Export => "secretspec export",
        });
        if let Some(resolution) = spec.resolution {
            command.push_str(&format!(" --profile {}", resolution.profile().name()));
        }
        if spec.inside_devenv {
            command.push_str(&format!(" --provider {}", helpers::PROVIDER_NAME));
        }
        if let Some(resolution) = spec.resolution {
            command.push_str(&format!(" --scope {}", resolution.scope()));
        }
        // Last, so `--` stays at the end of the line where it belongs.
        match &spec.delivery {
            Delivery::Wrap(cmd) => command.push_str(&format!(" -- {cmd}")),
            Delivery::Export => command.push_str(" --format gha"),
        }

        if spec.inside_devenv {
            command = format!("devenv shell -- {command}");
        }

        let name = match spec.delivery {
            Delivery::Wrap(_) => "secretspec".to_owned(),
            Delivery::Export => format!(
                "Resolve the {scope} secrets",
                scope = spec
                    .resolution
                    .map(Resolution::scope)
                    .unwrap_or_else(|| "default".to_owned()),
            ),
        };

        let step = Step::new(name)
            .run(command)
            .shell("bash")
            .when_some(spec.reason, |step, reason| {
                step.add_env(("SECRETSPEC_REASON", reason))
            });
        if spec.inside_devenv {
            helpers::credentialed(step)
        } else {
            helpers::resolving(step)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn rendered(step: impl Into<Step<Run>>) -> Value {
        serde_json::to_value(step.into()).expect("step serializes")
    }

    const STORE_CREDENTIAL: [&str; 3] = [
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "SCCACHE_ENDPOINT",
    ];

    /// Profile and scope travel as one value, so every rendering carries both.
    #[test]
    fn a_resolution_renders_its_profile_and_scope_together() {
        let wrapped = rendered(SecretSpec::run("true").resolving(Resolution::Ci(CiScope::Cache)));
        assert_eq!(
            wrapped["run"],
            "secretspec run --profile ci --scope cache -- true"
        );

        let exported = rendered(
            SecretSpec::export()
                .resolving(Resolution::Ci(CiScope::Botswain))
                .inside_devenv(),
        );
        assert_eq!(
            exported["run"],
            "devenv shell -- secretspec export --profile ci --provider op --scope botswain --format gha"
        );
        assert_eq!(exported["name"], "Resolve the botswain secrets");

        let release =
            rendered(SecretSpec::run("fastlane").resolving(Resolution::Release(Platform::iOS)));
        assert_eq!(
            release["run"],
            "secretspec run --profile release --scope ios -- fastlane"
        );
    }

    /// The server step is where the store is configured and where the
    /// credential arrives; a failed start must not fail the job.
    #[test]
    fn the_server_start_carries_the_store_configuration_and_tolerates_failure() {
        let step = rendered(sccache_start());
        let run = step["run"].as_str().expect("run");
        assert!(
            run.starts_with(
                "secretspec run --profile ci --scope cache -- sccache --start-server || echo "
            ),
            "{run}"
        );
        for (key, value) in [
            ("SCCACHE_BUCKET", "ci-cache"),
            ("SCCACHE_S3_KEY_PREFIX", "sccache/"),
            ("SCCACHE_REGION", "auto"),
            ("SCCACHE_IDLE_TIMEOUT", "0"),
            ("SECRETSPEC_PROVIDER", "op"),
        ] {
            assert_eq!(step["env"][key], value, "{key}");
        }
        assert!(step["env"].get("OP_SERVICE_ACCOUNT_TOKEN").is_some());
        assert!(step["env"].get("SECRETSPEC_REASON").is_some());
        for secret in STORE_CREDENTIAL {
            assert!(
                step["env"].get(secret).is_none(),
                "{secret} is configuration"
            );
        }
        assert_eq!(step["id"], "sccache-start");
    }

    /// The client needs the wrapper name and nothing of the store; its
    /// failure mode travels through cargo's config, not the step.
    #[test]
    fn the_wrapper_adds_exactly_the_client_variables() {
        let step = rendered(with_sccache(Step::new("x").run("true").shell("bash")));
        let env = step["env"].as_object().expect("env");
        assert_eq!(env.len(), 1, "{env:?}");
        assert_eq!(env["CARGO_BUILD_RUSTC_WRAPPER"], "sccache");
    }

    /// One constructor enables the wrapper, so the guard in `workflows.rs`
    /// that looks for a preceding server start cannot be sidestepped.
    #[test]
    fn only_the_wrapper_constructor_names_rustc_wrapper() {
        let source = include_str!("_steps.rs");
        assert_eq!(source.matches("(\"CARGO_BUILD_RUSTC_WRAPPER\"").count(), 1);
        assert_eq!(source.matches("(\"RUSTC_WRAPPER\"").count(), 0);
    }

    #[test]
    fn the_stats_step_always_runs_and_stops_the_server() {
        let step = rendered(sccache_stats());
        assert_eq!(step["if"], "always()");
        let run = step["run"].as_str().expect("run");
        assert!(run.contains("sccache --show-stats") && run.contains("sccache --stop-server"));
        assert!(step.get("env").is_none(), "needs no credential: {step}");
    }

    /// Each snapshot is keyed by what pins it, every store call is wrapped, the
    /// save waits on its own restore, and the credential is never on the step.
    #[test]
    fn cache_steps_key_on_the_manifest_and_wrap_every_store_call() {
        let android = cached_dirs(Platform::Android, "30.0.1");
        let names: Vec<_> = android.iter().map(|dir| dir.name).collect();
        assert_eq!(
            names,
            ["pnpm-store", "cargo-registry", "gradle", "android-ndk"]
        );
        assert_eq!(cached_dirs(Platform::macOS, "30.0.1").len(), 2);

        assert_eq!(
            android[0].key(),
            "pnpm-store-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('pnpm-lock.yaml') }}"
        );
        assert_eq!(
            android[3].key(),
            "android-ndk-${{ runner.os }}-${{ runner.arch }}-30.0.1"
        );
        assert!(android[2].key().contains("verification-metadata.xml"));

        for dir in &android {
            for (step, kind) in [(cache_restore(dir), "restore"), (cache_save(dir), "save")] {
                let step = rendered(step);
                let run = step["run"].as_str().expect("run");
                let store_calls: Vec<&str> =
                    run.lines().filter(|line| line.contains("rclone")).collect();
                assert!(
                    !store_calls.is_empty(),
                    "{kind} {}: no store call",
                    dir.name
                );
                for line in store_calls {
                    assert!(
                        line.contains("secretspec run --profile ci --scope cache -- "),
                        "{kind} {}: unwrapped store call: {line}",
                        dir.name
                    );
                    assert!(
                        line.contains("--s3-no-check-bucket"),
                        "{kind} {}: a bucket-scoped token cannot pass rclone's bucket check: {line}",
                        dir.name
                    );
                }
                assert_eq!(step["env"]["CACHE_KEY"], dir.key());
                assert!(
                    run.contains(&format!("path=\"{}\"", dir.path)),
                    "{kind} {}: the path is a shell expression and must be expanded by the script, not carried as an env literal: {run}",
                    dir.name
                );
                assert!(step["env"].get("CACHE_PATH").is_none());
                assert!(step["env"].get("OP_SERVICE_ACCOUNT_TOKEN").is_some());
                assert_eq!(step["env"]["SECRETSPEC_PROVIDER"], "op");
                for secret in STORE_CREDENTIAL {
                    assert!(
                        step["env"].get(secret).is_none(),
                        "{kind} {}: {secret}",
                        dir.name
                    );
                }
            }

            let save = rendered(cache_save(dir));
            assert_eq!(
                save["if"],
                format!(
                    "!cancelled() && steps.cache-restore-{}.outputs.hit != 'true'",
                    dir.name
                )
            );
            assert!(
                save["run"]
                    .as_str()
                    .expect("run")
                    .contains("--ignore-existing")
            );
        }
    }
}
