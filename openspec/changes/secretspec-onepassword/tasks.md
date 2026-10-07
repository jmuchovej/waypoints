## 1. Vault fixes (outside the repo)

- [x] 1.1 Add a base64-encoded text field (`base64`) to the `Android Keystore` item; verified by resolving it through a scratch manifest with `encoding = "base64"` and `as_path = true`: the delivered file is 1130 bytes and `cmp` against the attached `waypoints.jks` (fetched with `op read --out-file`) reports them identical
- [ ] 1.2 Verify the `Website` environment's `OP_SERVICE_ACCOUNT_TOKEN` belongs to a service account that can read only the website vault, by running `op vault list` under that token and checking it lists one vault

## 2. Manifest

- [x] 2.1 Keep the `op` (`onepassword+token://wp-prod`) and `op-web` (`onepassword+token://wp-web`) aliases already in the working copy, add `sops-web` (`sops://website/secrets.sops.yaml`), and keep `sops`; verify `secretspec check --profile automation` resolves both botswain secrets through `op`
- [x] 2.2 Rewrite every `release` and `automation` secret from `ref` to `refs` with both coordinate sets from design D8, point `[profiles.release.defaults].providers` at `op`, and drop `encoding` from `GOOGLE_PLAY_JSON_KEY_PATH`; verify `secretspec check --profile release --scope <s>` reports every secret found for each of `macos`, `linux`, `windows`, `android`, and `ios`
- [ ] 2.3 Verify the sops coordinates still work: with `SECRETSPEC_PROVIDER=sops` and the maintainer's age key, `secretspec check --profile release --scope macos` and `--profile automation` report every secret found from the local `secrets.sops.yaml`. **Needs the maintainer's shell**: the agent session has no age identity (`SOPS_AGE_KEY` empty, no `keys.txt`), so both local files fail to decrypt there
- [x] 2.4 Remove `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_DISTRIBUTION_CERTIFICATE`, `APPLE_DISTRIBUTION_PASSWORD`, and `APPLE_DISTRIBUTION_PROFILE` from `default` and `release`; verify every remaining `release` secret appears in at least one `[scopes.*]` list and an unscoped `secretspec check --profile release` fails only on the inherited `GH_TOKEN` and `WP_BOTSWAIN_*`
- [x] 2.5 Set `inherit = false` on `automation`, giving its two secrets their descriptions inline; verify `secretspec check --profile automation` lists exactly `WP_BOTSWAIN_APP_ID` and `WP_BOTSWAIN_PRIVATE_KEY`
- [x] 2.6 Add the `website` profile per design D9 — ten secrets with `refs` for `op-web` and `sops-web`, `inherit = false`, `required = true`, `providers = ["op-web"]`; verified `secretspec check --profile website` reports all ten found from the vault. The `sops-web` half is covered by 2.3's maintainer-shell note
- [x] 2.7 Verify the delivered files are exact: through `secretspec run --profile release --scope android -- sh -c 'wc -c < "$GOOGLE_PLAY_JSON_KEY_PATH"; wc -c < "$ANDROID_KEYSTORE"'` expect 2232 and 1130 bytes, and through `--scope macos` that `APPLE_API_KEY_PATH` is 257 bytes with a `-----BEGIN PRIVATE KEY-----` header
- [x] 2.8 Run `treefmt` over the manifest and verify `prek run` is green

## 3. Workflow generator

- [ ] 3.1 In `tooling/xtask/src/tasks/workflows/_helpers.rs`, remove `SECRETSPEC_PROVIDER` from `named_workflow`, delete the `SOPS_AGE_KEY` handle and env pair, and add constants for the `op` alias, the `OP_SERVICE_ACCOUNT_TOKEN` secret, and the two environment names (`App Release`, `CI`); verify `cargo check -p xtask` is clean
- [ ] 3.2 In `_steps.rs`, make `SecretSpec` steps and the platform `build` step set `SECRETSPEC_PROVIDER` and `OP_SERVICE_ACCOUNT_TOKEN`; verify by regenerating and reading the YAML that every `secretspec` step and every `build-recipe` step carries both and nothing else does
- [ ] 3.3 In `_jobs.rs` (or wherever the release, publish, release-plan, and devenv-update jobs are assembled), set `environment:` to `App Release` on every job that resolves `release` and to `CI` on every job that resolves `automation`; verify the generated `release-*.yaml` build and publish jobs declare `App Release`, `release-plan.yaml` and `devenv-update.yaml` declare `CI`, and `pull-request.yaml` and `check-generated.yaml` declare none
- [ ] 3.4 Add a generator test asserting that every job containing a step that sets `OP_SERVICE_ACCOUNT_TOKEN` declares an environment, and that no job without one declares an environment; verify `cargo test -p xtask` passes
- [ ] 3.5 Run the `devenv-update` export step inside `devenv shell` so `secretspec` and `op` come from the same place as the regenerate step before it; verify in the generated YAML that the command is wrapped and the step env carries the alias and token only
- [ ] 3.6 Regenerate with `cargo xtask workflows` and verify `rg -n "SOPS_AGE_KEY|SECRETSPEC_PROVIDER: sops" .github/workflows` is empty, no workflow has a top-level `env.SECRETSPEC_PROVIDER`, `just check-generated` passes, and `zizmor` reports nothing new

## 4. Recipes

- [ ] 4.1 In `src-tauri/.just/macOS.justfile`, `android.justfile`, and `iOS.justfile`, extend the `env -i` whitelists to forward `SECRETSPEC_PROVIDER`, `OP_SERVICE_ACCOUNT_TOKEN`, `SOPS_AGE_KEY_FILE`, and `SOPS_AGE_KEY`, each defaulting to empty; verify with `SECRETSPEC_PROVIDER=env` that the clean-room build's `secretspec run` reports the release secrets missing from `env` rather than contacting a vault
- [ ] 4.2 Verify the local-store path on one clean-room platform: with `SECRETSPEC_PROVIDER=sops` and the maintainer's age key, `just tauri::macos::build` (or `android`) resolves from `secrets.sops.yaml` and never prompts for 1Password
- [ ] 4.3 Rewrite `website/justfile`'s `gen-dev-vars` over `secretspec export --profile website --format dotenv --reason "…"`, dropping the `SOPS_AGE_KEY` guard; verify the rendered `.dev.vars` carries the ten keys, `nuxi dev --dotenv .dev.vars` reads every value, and the umask/tmp/move sequence is intact
- [ ] 4.4 Rewrite `secrets-push` over `--format json | wrangler secret bulk` and `secrets-export` over `--format shell`, each with a reason; verify `wrangler secret list` shows the ten names afterwards and `eval "$(just website::secrets-export)"` exports them
- [ ] 4.5 Verify `secretspec run --profile website` without `--reason` is refused, confirming the `require_reason` policy covers the new profile

## 5. Toolchain

- [ ] 5.1 In `devenv.nix`, add `op` (`pkgs._1password-cli`, `aqua:1password/cli`, base membership) and declare `sops` and `age` shell-only per `tool-projection`'s no-membership form; verify a shell entry regenerates `.config/mise.toml` with `op` present and `sops`/`age` absent from every `.config/mise.*.toml`, while `which sops age` still resolve inside the devenv shell
- [ ] 5.2 Run `just mise-lock` and verify `.config/mise.lock` carries `op` entries for linux, macos, and windows, then `just mise-lock-check` and `just check-generated` pass

## 6. Take the sops files out of version control

- [ ] 6.1 Add `secrets.sops.yaml` and `website/secrets.sops.yaml` to `.gitignore` and untrack both with `jj file untrack`; verify `jj status` shows them removed from the change while both files remain on disk and still decrypt with the maintainer's key
- [ ] 6.2 Remove the `waypoints-ci` recipient from `.sops.yaml` and re-encrypt both local files to the maintainer's key alone (`sops updatekeys`); verify `sops -d` still succeeds locally and `rg waypoints-ci` across the tree is empty
- [ ] 6.3 Land groups 2 through 6 as one commit per group (the maintainer asked for reviewable steps rather than one change), each with per-commit hooks green; verify the combined `jj diff --stat main..@` touches the manifest, `.gitignore`, `.sops.yaml`, generator, workflows, recipes, devenv config, mise projection, and the two untracked files, and nothing else. The intermediate commits are not individually coherent for CI — the manifest points at the vault before the workflows carry the token — which is acceptable because nothing between them is pushed

## 7. Cutover

- [ ] 7.1 Dispatch `devenv-update` by hand and verify it reaches the pull-request step and opens or refreshes the `devenv/update` PR — the first successful run of that workflow since September
- [ ] 7.2 Run a nightly or preview release and verify every platform job's `build-recipe` step resolves its scope from the vault — signed desktop bundles on macOS, Linux, and Windows, and the fastlane lanes for iOS and Android completing through their `publish-mobile-app` steps — with each job shown under `App Release` in the run's environment view
- [ ] 7.3 Run `just website::gen-dev-vars` and `just website::secrets-list` by hand, once through the desktop app and once under `SECRETSPEC_PROVIDER=sops-web`, and verify both succeed with no service-account token in the shell
- [ ] 7.4 Remove the repo-level `OP_SERVICE_ACCOUNT_TOKEN` and `SOPS_AGE_KEY` from GitHub Actions secrets; verify `gh secret list` shows neither while `gh secret list --env "App Release"`, `--env CI`, and `--env Website` each still show the token, then re-dispatch `devenv-update` and verify it still succeeds
- [ ] 7.5 Retire the `waypoints-ci` age identity from wherever the maintainer keeps it, and verify no reference to it remains in the repository
