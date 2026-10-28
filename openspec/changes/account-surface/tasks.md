Every task leaves the per-commit hooks green: treefmt formats with no remaining diff, `just website::test` passes, and `just website::typecheck` reports no error in a file the task touched. The website's typecheck carries roughly a hundred pre-existing Nuxt UI typing errors elsewhere. No task claims to clear them, and each task compares against `main`'s error list before attributing one. Persistence tests run against a real libSQL engine in memory through `drizzle-orm/libsql`, with the schema module applied by drizzle-kit. No test asserts on SQL text.

Tasks are written to design.md's recommendations on its two open questions: several identities per account, and no grace window. A different answer changes 3.5 and 5.4 only.

## 1. Test harness and contract types

- [ ] 1.1 Add a vitest helper that creates an in-memory libSQL database, applies `server/database/schema.ts` through drizzle-kit's programmatic push, and returns a drizzle handle of the production type. Verify a smoke test inserts and reads back a `user` row through it.
- [ ] 1.2 Add the account API's request, response, and error-code types under `website/shared/`, covering every endpoint in the spec's contract table. Verify `just website::typecheck` reports no error in the new module.

## 2. Session guard and the closed deletion path

- [ ] 2.1 Add a server guard that resolves the session through `auth.api.getSession`, refuses a missing session with 401 and an error code, and exposes a recency check (session created within ten minutes) that refuses with a re-authenticate code. Verify vitest covers no session, a fresh session, and an eleven-minute-old session, against the real engine.
- [ ] 2.2 Make the `/api/auth` catch-all refuse `/delete-user` and `/delete-user/callback`. Verify a test asserts both paths are refused while `/list-sessions` still reaches better-auth.
- [ ] 2.3 Add a better-auth `hooks.before` that applies the recency check to `/two-factor/disable`, and set `twoFactor({ allowPasswordless: true })`. Verify tests show a stale session cannot disable a second factor, and a provider-only account can enrol one without a password.
- [ ] 2.4 Add the contract test that confirms every better-auth endpoint the spec names exists on the configured instance. Verify it fails when one name is altered.

## 3. Identity binding

- [ ] 3.1 Add `identity_binding` (member public key unique across accounts, account id, state bound or pending, created and confirmed timestamps) and `identity_challenge` (id, account id, 32-byte nonce, expiry, consumed flag) to the schema. Verify the harness from 1.1 applies them and the uniqueness constraint rejects a second account binding the same key.
- [ ] 3.2 Add the binding-message builder and verifier as plain functions. The verifier uses WebCrypto Ed25519 and rejects small-order and non-canonical keys before verification. Verify vitest round-trips a WebCrypto-generated keypair, and refuses a signature over a different origin, a different account, or with the tag omitted or replaced by `outpost/vessel-cert/v1`.
- [ ] 3.3 Add `POST /api/v1/account/identities/challenges`, returning each message field separately. Verify a test shows challenges are session-only and expire after five minutes.
- [ ] 3.4 Add `POST /api/v1/account/identities/challenges/{id}/response`. It consumes the challenge on first use and binds at once on a session of the issuing account, or creates a pending binding without one. Verify tests cover a bound result, a pending result, a second response refused, an expired challenge, a bad signature that consumes the challenge, and a key bound elsewhere refused without naming the other account.
- [ ] 3.5 Add `GET /api/v1/account/identities`, `POST …/{member}/confirm` (pending within ten minutes of the response, else lapsed), and `DELETE …/{member}` (recency-gated), with a cap of eight identities per account counting pending entries. Verify tests cover confirm, lapse, unlink-then-rebind, and refusal at the cap.

## 4. Deletion cascade

- [ ] 4.1 Add the purge registry: each entry declares a table, a scope (`data` or `account`), a description, and a purge function. Enrol `identity_binding` and `identity_challenge` as `data`, and `passkey`, `two_factor`, and the `verification` rows naming the user as `account`. Keep the descriptions in a `shared/` module. Verify the module typechecks and the registry lists exactly those stores.
- [ ] 4.2 Add the completeness test. It walks every schema table, finds columns referencing `user.id` plus the tables keyed on the user another way, and fails naming any table that is neither enrolled nor removed by better-auth itself. Verify it fails when a scratch table with a user reference is added unenrolled.
- [ ] 4.3 Add `POST /api/v1/account/data/delete`, recency-gated, running the `data` purgers in one batch. Verify against the real engine that bindings, pending entries, and challenges are gone while the user, sessions, passkeys, and second factor remain, and that the same session still authenticates.
- [ ] 4.4 Enable `user.deleteUser` with a `beforeDelete` that runs every purger, data before account, in one batch. Add `POST /api/v1/account/delete`, recency-gated, which requires the body's email to match exactly and then calls `auth.api.deleteUser`. Verify against the real engine with foreign keys off that no row in any table refers to the deleted user afterwards, every session is invalid, the email can sign up anew, and a mismatched email deletes nothing.
- [ ] 4.5 Verify an interrupted deletion: make a purger throw partway through account deletion, and assert the account still exists. Then assert a retry completes the deletion and leaves no rows.

## 5. Website pages

- [ ] 5.1 Make the sign-in page honour a `redirect` query parameter that is a same-origin path, falling back to `/account`. Verify a unit test of the path check refuses absolute URLs, protocol-relative URLs, and backslash tricks.
- [ ] 5.2 Add `/auth/2fa`, accepting a TOTP or a backup code through the two-factor client. Verify by signing in to an account with a second factor in `just website::dev` and reaching `/account` only after the code is accepted.
- [ ] 5.3 Replace the `/account` stub with a sectioned layout: profile, security, identities, and danger zone. Profile reads `GET /api/v1/account` and edits the name through `PATCH`. Security covers password change, passkeys, two-factor enrolment and removal, sessions with revoke, and linked GitHub, all through the named better-auth endpoints. Unauthenticated visitors are sent to sign-in with a return path. Verify by exercising each section in `just website::dev`.
- [ ] 5.4 Build the identities section to list bound and pending identities with fingerprints, confirm pending entries, and unlink. Build the danger zone with data deletion and account deletion (typed email). Both show the device-data statement before the confirm control, and a re-authenticate refusal routes through sign-in and back. Verify each path in `just website::dev`, including a stale-session refusal.
- [ ] 5.5 Add the public `/account/delete` page. It names Waypoints, lists the deletion steps, renders deleted data per scope from the shared descriptions, says nothing is retained and that device data is untouched, gives the route for users who cannot sign in, and links to `/auth/login?redirect=/account/danger`. Verify it renders with no session and no prior visit.
- [ ] 5.6 Add route rules that prerender `/account/delete` and no other `/account/**` route. Verify with `just website::build` that `.output/public` contains a prerendered `/account/delete` and no other account page. If the build cannot run outside a deployed environment, because of the Cloudflare bindings, record that and verify the rules by inspection instead.

## 6. App entry point

- [ ] 6.1 Add a settings entry in the app that opens the website's `/account/delete` in the system browser through the shell plugin, reachable without an account. Verify on an Android build that it opens the page in the system browser rather than embedding it, and that `just typecheck` adds no app error beyond `main`'s pre-existing ones.

## 7. Closing checks

- [ ] 7.1 Run `just website::test`, `just website::typecheck` (comparing against `main`'s pre-existing error list), and treefmt. Verify no new typecheck error and no formatting diff.
- [ ] 7.2 Add the `accounts` domain to `openspec/config.yaml`'s registry, and verify `openspec validate account-surface --strict` passes.
- [ ] 7.3 Before the deploy that serves the new API, apply the two tables with `just website::db push`. After the deploy, confirm `/account/delete` is reachable with no session. Then hand the URL to the Play Console listing and the Data safety form, which happens outside the repo.
