# Agent Guide — `nix/`

Nix lives here: the devenv modules that describe the shell, and the package
definitions that build the app.

## Layout

| Path              | Holds                                                                  |
| ----------------- | ---------------------------------------------------------------------- |
| `nix/devenv/`     | devenv modules — one per concern, imported by `nix/devenv/default.nix` |
| `nix/devenv/bin/` | scripts nix deploys **as devenv tasks**                                |
| `nix/packages/`   | package definitions, built by the flake and the overlay                |
| `nix/overlay.nix` | the overlay downstream consumers apply                                 |

## Where a script goes

The deciding question is **who runs it**, not what it does.

- `nix/devenv/bin/` — the script is wired to a `tasks."…"` entry and runs
  because entering the shell ran it. It never appears on PATH; devenv reads it
  with `builtins.readFile` and executes it under the task's `package`.
- `tooling/bin/` — a person or a `just` recipe runs it. `.env` puts
  `tooling/bin` on PATH, so it is callable by bare name.

A script that stops being a devenv task should move out of `nix/devenv/bin`;
staying there implies an automation that no longer exists.

## Generated versus hand-maintained

devenv writes a `files."…"` entry on **shell entry**, which has two
consequences worth knowing before adding one:

- Anything read _before_ that is too late to generate. mise's config-discovery
  file is the live example: it must exist in a fresh clone before the shell has
  ever been entered, so it is hand-maintained.
- Generated files cannot be linted. `.config/treefmt.toml` excludes them,
  because tombi would reformat what devenv writes and `check-generated` would
  never converge. Schemas are therefore for hand-edited config only; a
  generated file relies on its consumer parsing it or failing loudly.

So project only what nix computes. A constant does not become generated just
because it sits near generated things.

## Cleanup does not cover copied files

`devenv:files:cleanup` removes a file it has stopped generating **only if that
file is a symlink into the nix store** — deliberately, so it cannot delete
something a human edited. A `copyMode = "copy"` file is permanently outside its
reach, and `check-generated` cannot see a leftover either: it diffs the files
devenv _currently_ declares, and a leftover is by definition not among them.

Anything generated with `copyMode = "copy"` whose set can shrink needs its own
prune task. See `tasks."mise:prune-environments"`.

## Tasks that write what they watch

A task listing a file in `execIfModified` that it also writes re-runs itself on
every entry. If it writes over the network, a transient failure becomes a
silent downgrade — the mise lockfile did exactly this, and the partial result
was reported as success. Entry-time tasks should read; writing belongs in
`tooling/bin` behind a `just` recipe.
