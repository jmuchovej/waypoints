## Context

See `proposal.md` — Why. The current-state facts the approach rests on, each verified against the pinned secretspec (0.19.1, from `pkgs.secretspec` and projected as `aqua:cachix/secretspec`) and, where it says so, against the real vaults:

- **The vaults exist and are populated.** `wp-prod` holds five items: `Apple Notary` (Document: `AuthKey_2DR9MVAL3V.p8` attached; fields `key-id`, `issuer-id`), `Android Keystore` (Document: `waypoints.jks` attached; fields `alias`, `password`), `Google Play Store` (Document: `google-play-store-key.json` attached), `Tauri Updater` (Login; fields `private-key`, `password`), and `botswain` (SSH Key; `private key`, plus field `app-id`). `wp-web` holds `GitHub PAT` (custom field `token` populated; the built-in `credential` field is empty), `Nuxt OG Image` and `Better Auth` (Password), `Turso Database` (Server; fields `URL`, `password`), `GitHub OAuth App` (Secure Note; fields `app-id`, `client-id`, `client-secret`), `Cloudflare Turnstile` (Login; fields `site-key`, `secret-key`), and `GitHub OAuth Key` (SSH Key, standard fields only).
- **The website's runtime reads** `github.token`, `github.clientId`, `github.clientSecret`, `public.github.appId`, `betterAuthSecret`, `turso.databaseUrl`, `turso.authToken` (`website/nuxt.config.ts`), plus the Turnstile and OG-image module secrets. No private key. `GitHub OAuth Key` therefore has no consumer.
- **Not migrated**: `apple-certificate`, `apple-certificate-password`, `apple-distribution-certificate`, `apple-distribution-password`, `apple-distribution-profile`. None of the five appears in any `[scopes.*]` list, so no CI resolution has ever read them.
- **Vault titles must fit `op`'s reference grammar.** secretspec builds `op://<vault>/<item>/<field>` and hands it to `op read`, and 1Password's reference syntax admits only alphanumerics, `-`, `_`, `.`, and space. The vaults were first titled with parentheses and `op` itself refused every reference (`invalid character in secret reference: '('`); IDs worked in that position. The maintainer renamed the vaults to `wp-prod` and `wp-web`, which the grammar accepts, and the aliases name those titles. Item and field names in use are all within the grammar.
- **Attached files resolve by naming the file as the `field`.** `ref = { item = "Google Play Store", field = "google-play-store-key.json" }` with `as_path = true` delivered exactly 2232 bytes, matching `op read`; the `.p8` delivered exactly 257. A Document referenced with no `field` resolves to nothing.
- **Binary attachments do not survive the 1Password provider, on either secretspec line.** `op read --out-file` delivers `waypoints.jks` as 1130 bytes of binary. Through secretspec 0.19.1's `as_path` the same attachment arrives as 1718 bytes containing 400 U+FFFD replacement characters: that version holds every value as a UTF-8 string and decodes lossily. 0.21.0 made secret values byte-preserving in general (its release notes: "Keep binary secrets intact"), but its 1Password provider still decodes `op`'s stdout as text and now fails closed — 0.21.1 run against the same item reports `1Password CLI returned non-UTF-8 output: invalid utf-8 sequence`. PEM and JSON are valid UTF-8 and pass through both versions unchanged. So the keystore must be stored as base64 text regardless of which secretspec is pinned.
- **The SSH Key item returns PKCS#1.** `botswain`'s `private key` field comes back with a `-----BEGIN RSA PRIVATE KEY-----` header, which is the format GitHub issues App keys in and what `actions/create-github-app-token` consumes.
- **Per-provider `refs` works on 0.19.1.** A scratch manifest with `refs = { op = { item = "Apple Notary", field = "key-id" }, sops = { item = "apple-notary-key-id" } }` resolved from the real vault with `op` as the profile default and from a scratch sops file under `SECRETSPEC_PROVIDER=sops`, returning the sops file's value in the second case. `ref` and `refs` are mutually exclusive on one secret; a provider with no `refs` entry falls through to the `{project}/{profile}/{key}` convention, which is what a keyring uses.
- **Profile `defaults.providers` selects the store when nothing overrides it**, `SECRETSPEC_PROVIDER` overrides it (devenv injects `env`), and a non-default profile with `required = true` makes every inherited secret required unless `inherit = false`. All three measured; the last one shows in the real `automation` profile today.
- **`release` depends on inheritance** for `encoding`, `as_path`, and descriptions. `automation` and the new `website` profile do not.
- **The workflow-level stamp is breaking CI today.** `named_workflow` sets `SECRETSPEC_PROVIDER: sops` on every workflow; the weekly `devenv-update` run has failed on every schedule since at least 2026-09-07 because its first step enters `devenv shell`, reads no secret, and dies on `The 'sops' CLI is not installed`.
- **GitHub has three environments** — `App Release`, `CI`, `Website` — each holding `OP_SERVICE_ACCOUNT_TOKEN`, plus a repo-level copy of the same name. `App Release` and `CI` currently hold the same service account. Generated jobs declare no `environment:` today.
- **`onepassword+token://` is accepted by 0.19.1** and, with no token present, falls back to desktop-app authentication, so one alias serves CI and the maintainer's shell.
- **The clean-room build recipes drop the selection.** `macOS`, `android`, and `iOS` build under `env -i` with a whitelist that forwards `SOPS_AGE_KEY` and not `SECRETSPEC_PROVIDER`.
- **jj tracks every new file unless ignored**, so a local sops file at the conventional path must be gitignored, not merely untracked.
- **`op` is available on both sides of the projection**: `pkgs._1password-cli` (2.39.0) and `aqua:1password/cli`.
- **`secretspec export` has `shell`, `dotenv`, `json`, and `gha` formats**, covering the three `website/justfile` recipes one-for-one.

## Goals / Non-Goals

**Goals**

- One declaration per secret, two coordinate sets. The manifest changes only where the stores genuinely differ.
- A generated workflow's YAML shows, per job, which environment it runs in, and per step, which store it reads with which credential. Nothing is inherited from workflow scope.
- The local sops path is a first-class way to run a release build, not a leftover: same declarations, same recipes, own key.
- Splitting `App Release` from `CI` later is a 1Password and GitHub change only.

**Non-Goals**

- Caching, fallback chains, or offline resolution for CI. Offline work is what the local sops store is for.
- Generalising the generator over providers. It targets 1Password; if the CI store changes again, the alias and the credential variable change with it.
- Managing vaults, items, or service accounts from the repo.

## Decisions

### D1 (2026-10-04): The spec lands in `toolchain/secret-provisioning`

`distribution` is defined as delivery outside the Tauri bundler pipeline, and the signing material here feeds that pipeline. What the change actually constrains is how the repo's CI and shells obtain credentials — a reproducibility property of the build environment, the same kind of thing `tool-projection` specifies. One mechanism, one home.

Rejected: `distribution` for the signing material and `toolchain` for the automation token. The invariants are identical for both profiles; splitting them would state each twice and leave the website profile with no home.

### D2 (2026-10-04, revised): One production vault; GitHub environments are the credential boundary

`release` and `automation` share the `wp-prod` vault and one service account. The reach boundary moves from the vault to the GitHub environment: every secret-resolving job declares `environment:`, reads the token from it, and the repo-level token is removed. Today `App Release` and `CI` carry the same account, so the boundary is latent — but the generator partitions jobs by the work they do, so when the maintainer issues a second service account and points `CI` at a narrower vault, no workflow changes.

The website vault stays separate with its own account in the `Website` environment. Nothing automated reads it yet; the account exists so that a deploy job, when one is written, declares `Website` and nothing else.

This supersedes the earlier draft (a vault and a service account per profile), which the maintainer's single production vault made moot: environment-level credentials already express which jobs may hold which token.

Rejected: keep the repo-level `OP_SERVICE_ACCOUNT_TOKEN` alongside the environment copies. A job that forgets to declare an environment would silently resolve through it, which is exactly the leak environments exist to prevent.

### D3 (2026-10-04, revised): Aliases name the vaults by title; credentials arrive by environment variable

Two 1Password aliases, `op = "onepassword+token://wp-prod"` and `op-web = "onepassword+token://wp-web"`, both already in the working copy. The titles are within `op`'s reference grammar, so they work in the reference secretspec builds. The `+token` scheme states the CI intent and still falls back to the desktop app locally. Steps set `OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}`, which resolves against the job's declared environment.

Rejected: the vault IDs. They also work and survive renames, but a title reads in the manifest and a rename fails loudly at the first resolution rather than silently; the maintainer chose titles.

Rejected: `credentials = { service_account_token = … }` on the alias. It would commit a credential source into the manifest that every reader shares; the environment route is what the provider documents for CI and what the generator already wires.

### D4 (2026-10-04): Selection and credential are set on the resolving step; `named_workflow` sets neither

`named_workflow` drops `SECRETSPEC_PROVIDER`. The `SecretSpec` step and the platform `build` step each set `SECRETSPEC_PROVIDER=op` and `OP_SERVICE_ACCOUNT_TOKEN`. Setting the provider on the step is strictly necessary only inside `devenv shell` (devenv injects `env`); under mise the profile default would already select `op`. It is set unconditionally so the generated YAML reads the same for every resolving step.

Jobs gain `environment:` through the existing job constructors in `_jobs.rs`: a job's environment is derived from which profile its steps resolve (`release` → `App Release`, `automation` → `CI`), so a step cannot end up in a job that declares the wrong one.

Rejected: keep the workflow-level variable and rename its value. That is the current breakage with `op` substituted for `sops`.

Rejected: `--provider` on the command line. The `build` step's `secretspec run` is inside a justfile; the environment is the only channel that reaches it, and the same channel the clean-room whitelists forward.

### D5 (2026-10-04): `automation` and `website` set `inherit = false`; `release` keeps inheriting

Both profiles declare their secrets in full and stop inheriting, so checking or exporting either requires exactly its own secrets. `release` keeps inheriting because its encoding and path semantics live on the `default` declarations. Its inherited `GH_TOKEN` and `WP_BOTSWAIN_*` stay harmless because every CI resolution of `release` is scoped by platform, which the spec now states as a requirement.

Rejected: duplicating `encoding`/`as_path` into `release`. Violates "declared once" for a profile whose inherited extras are already excluded by scope.

### D6 (2026-10-04, revised): sops stays as the local development store, addressed through `refs`

Every `release`, `automation`, and `website` secret replaces `ref` with `refs` carrying both stores' coordinates. The sops coordinates are the existing root-key names, so a sops file written for today's manifest keeps working. The `sops` alias keeps pointing at `secrets.sops.yaml` and a `sops-web` alias points at `website/secrets.sops.yaml`; both files become gitignored and leave version control. `.sops.yaml` stays committed with only the maintainer's recipient, so `sops -e` on a fresh local file does the right thing for the maintainer and a contributor substitutes their own recipient.

Selecting the store is `SECRETSPEC_PROVIDER=sops` (or `sops-web`) in the shell, which overrides the profile default. The clean-room whitelists forward `SOPS_AGE_KEY_FILE` and `SOPS_AGE_KEY` alongside the 1Password token so the selection carries through.

Rejected: a sops-backed `development` profile. The declarations live in `release`/`automation`/`website`; what changes locally is the store, not the secret set, and `refs` expresses exactly that on the existing profiles.

Rejected: dropping sops entirely in favour of keyring for contributors. Keyring still works (it falls through to the convention path), but a sops file is portable between machines, reviewable, and what the maintainer already has.

### D7 (2026-10-04, revised): `op` joins the mise base; `sops` and `age` become shell-only

`op` is added to `waypoints.mise.tools` with `package = pkgs._1password-cli`, `spec = "aqua:1password/cli"`, and base membership, because every CI lane that resolves secrets needs it. `sops` and `age` are declared shell-only — present in the devenv shell for local development, absent from every projected environment — because no CI job decrypts a sops file any more. `tool-projection` already provides the shell-only form; this is a declaration edit under it.

Rejected: keeping `sops`/`age` in the base. Every CI job would install two tools no job runs, and the projection would say sops is part of CI when it is not.

### D8 (2026-10-04, revised): Production coordinates, both stores

| Secret                               | `refs.op`                                                              | `refs.sops` (item)           | Declaration change                                  |
| ------------------------------------ | ---------------------------------------------------------------------- | ---------------------------- | --------------------------------------------------- |
| `APPLE_API_KEY_PATH`                 | `{ item = "Apple Notary", field = "AuthKey_2DR9MVAL3V.p8" }`           | `apple-notary-key`           | none (`as_path`)                                    |
| `APPLE_API_KEY`                      | `{ item = "Apple Notary", field = "key-id" }`                          | `apple-notary-key-id`        | none                                                |
| `APPLE_API_ISSUER`                   | `{ item = "Apple Notary", field = "issuer-id" }`                       | `apple-notary-issuer-id`     | none                                                |
| `TAURI_SIGNING_PRIVATE_KEY`          | `{ item = "Tauri Updater", field = "private-key" }`                    | `updater-private-key`        | none                                                |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | `{ item = "Tauri Updater", field = "password" }`                       | `updater-password`           | none                                                |
| `ANDROID_KEYSTORE`                   | `{ item = "Android Keystore", field = "base64" }`                      | `android-keystore`           | none (`base64`, `as_path`)                          |
| `ANDROID_KEYSTORE_ALIAS`             | `{ item = "Android Keystore", field = "alias" }`                       | `android-keystore-alias`     | none                                                |
| `ANDROID_KEYSTORE_PASSWORD`          | `{ item = "Android Keystore", field = "password" }`                    | `android-keystore-password`  | none                                                |
| `GOOGLE_PLAY_JSON_KEY_PATH`          | `{ item = "Google Play Store", field = "google-play-store-key.json" }` | `google-play-store-json-key` | drop `encoding`; the local store holds raw JSON too |
| `WP_BOTSWAIN_APP_ID`                 | `{ item = "botswain", field = "app-id" }`                              | `botswain-app-id`            | none                                                |
| `WP_BOTSWAIN_PRIVATE_KEY`            | `{ item = "botswain", field = "private key" }`                         | `botswain-private-key`       | none                                                |

`ANDROID_KEYSTORE_PATH` stays composed from the three keystore values. The `base64` field was added by the maintainer and verified to decode byte-identical to the attached `waypoints.jks` (1130 bytes, `cmp` clean) through `encoding = "base64"` and `as_path`; the binary attachment stays on the item for human use, but nothing in the repo reads it.

The `encoding` on the Play Store key must be the same in both stores because it is declared once: dropping it means a local sops file carries the raw JSON, which is also what the vault holds.

Rejected: referencing the binary attachment directly. 0.19.1 corrupts it and 0.21.1 refuses it (both measured); the 1Password provider's stdout path is text on every line.

### D9 (2026-10-04, revised): Website coordinates, both stores

| Secret                           | `refs.op-web`                                             | `refs.sops-web` (item)           |
| -------------------------------- | --------------------------------------------------------- | -------------------------------- |
| `NUXT_GITHUB_TOKEN`              | `{ item = "GitHub PAT", field = "token" }`                | `NUXT_GITHUB_TOKEN`              |
| `NUXT_GITHUB_APP_ID`             | `{ item = "GitHub OAuth App", field = "app-id" }`         | `NUXT_GITHUB_APP_ID`             |
| `NUXT_GITHUB_CLIENT_ID`          | `{ item = "GitHub OAuth App", field = "client-id" }`      | `NUXT_GITHUB_CLIENT_ID`          |
| `NUXT_GITHUB_CLIENT_SECRET`      | `{ item = "GitHub OAuth App", field = "client-secret" }`  | `NUXT_GITHUB_CLIENT_SECRET`      |
| `NUXT_OG_IMAGE_SECRET_KEY`       | `{ item = "Nuxt OG Image", field = "password" }`          | `NUXT_OG_IMAGE_SECRET_KEY`       |
| `NUXT_BETTER_AUTH_SECRET`        | `{ item = "Better Auth", field = "password" }`            | `NUXT_BETTER_AUTH_SECRET`        |
| `NUXT_TURSO_DATABASE_URL`        | `{ item = "Turso Database", field = "URL" }`              | `NUXT_TURSO_DATABASE_URL`        |
| `NUXT_TURSO_AUTH_TOKEN`          | `{ item = "Turso Database", field = "password" }`         | `NUXT_TURSO_AUTH_TOKEN`          |
| `NUXT_PUBLIC_TURNSTILE_SITE_KEY` | `{ item = "Cloudflare Turnstile", field = "site-key" }`   | `NUXT_PUBLIC_TURNSTILE_SITE_KEY` |
| `NUXT_TURNSTILE_SECRET_KEY`      | `{ item = "Cloudflare Turnstile", field = "secret-key" }` | `NUXT_TURNSTILE_SECRET_KEY`      |

`GitHub OAuth Key` is deliberately absent: `nuxt.config.ts` reads no private key. The three recipes become `secretspec export --profile website --format dotenv` (into the existing tmp-then-move sequence), `--format json | wrangler secret bulk`, and `--format shell`, each with a `--reason`. The `SOPS_AGE_KEY` guard goes; selecting `sops-web` locally is the same `SECRETSPEC_PROVIDER` mechanism as everywhere else.

Rejected: calling `op` or `sops` directly from the recipes. Reintroduces backend-specific addressing with no `require_reason` or audit.

### D10 (2026-10-04): Five un-migrated secrets are removed from the manifest

`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, and the three `APPLE_DISTRIBUTION_*` declarations are deleted from `default` and `release`. They belong to no scope, so no job resolves them; with `required = true` on `release` they would only make an unscoped `secretspec check --profile release` fail against items that do not exist. The spec's "no unscoped release secret" scenario makes this a standing invariant.

Rejected: keeping them as optional. A declaration nothing resolves and no store holds is documentation pretending to be configuration.

### D11 (2026-10-04, revised): Cutover order

1. Vault fix: add the base64 keystore field (maintainer, outside the repo).
2. Commit A: gitignore both sops files and untrack them (they stay on disk as the maintainer's local store); `.sops.yaml` loses the CI recipient; manifest with `refs`; generator with environments; regenerated workflows; recipes and whitelists; toolchain swap. One coherent state — any subset leaves either the manifest or the workflows pointing at a store CI can no longer reach.
3. Verify: `workflow_dispatch` of `devenv-update`; a nightly or preview release across every platform scope; a hand-run of the three website recipes against each store.
4. Outside the repo: remove the repo-level `OP_SERVICE_ACCOUNT_TOKEN` and `SOPS_AGE_KEY`; retire the `waypoints-ci` age identity.

Rollback from a failed step 3 is reverting commit A. The sops files are still on disk and still decrypt with the maintainer's key, so the old CI path is one revert away until step 4.

## Risks / Trade-offs

- [`App Release` and `CI` share one service account, so the latent boundary is not enforced] → Accepted by the maintainer for now; the generator's per-job `environment:` means enforcing it later is a 1Password and GitHub change only.
- [A job that forgets `environment:` resolves nothing rather than leaking] → Intended once the repo-level token is gone (task 7.4). Task 3.4 asserts in the generator's tests that every job containing a resolving step declares an environment.
- [A contributor's local sops file drifts from the manifest's sops coordinates] → `secretspec check --provider sops --profile release` names every missing key; the coordinates are the root keys, so the file is self-describing.
- [A future vault rename reintroduces a character outside the reference grammar] → The alias fails at the first resolution with `op`'s own `invalid character in secret reference` message, which names the offending title; nothing silently degrades.
- [`secretspec export --format dotenv` quotes values differently from the `yq … @json` form the website writes today] → Task 4.3 diffs the rendered `.dev.vars` key set and confirms `nuxi dev --dotenv` reads every value.
- [The `devenv-update` export step runs `secretspec` outside `devenv shell`, so `op` may not be on its PATH] → The step has never executed successfully. Task 3.5 runs it through `devenv shell` like the step before it.
- [Windows runner needs `op` for the Windows signing scope] → `aqua:1password/cli` publishes Windows builds; task 5.2 confirms the lockfile carries a Windows entry.
- [`devenv update` later moves secretspec to 0.21, where the 1Password provider fails closed on non-UTF-8 instead of mangling] → Nothing here hands it non-UTF-8: the keystore is base64 text and every other value is PEM, JSON, or a short string. A new binary secret added later must follow the same rule, which the spec's "binary material is carried as text" scenario states.

## Migration Plan

See D11. The only irreversible steps are the GitHub secret removals and retiring the age identity, which come after the verification runs.
