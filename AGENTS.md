# Agent Guide — Waypoints

Entry point for agents working in this repo.

## Commits

Follow the commit conventions in [`CONTRIBUTING.md`](/docs/CONTRIBUTING.md) exactly — they are the single source of truth for both humans and agents. In short:
- `type(scope): summary — optional elaboration` (see `CONTRIBUTING.md` for the full type table and rules).
- The scope is `region[/subscope]`. The **region** (first segment) is fixed — `domain`, `cartograph`, `bedrock`, `outpost`, `app`, `website` — and always tells you where the change lives. The optional **subscope** (second segment) is an open-ended theme that groups work across regions, e.g. `feat(outpost/sync): …`, `feat(app/editor): …`.
- Commit messages feed changelog and release-note generation — a wrong or missing `type`/`scope` drops the change from the notes. Get it right.
- Every commit is signed.
