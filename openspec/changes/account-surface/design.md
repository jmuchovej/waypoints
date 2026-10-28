## Context

See proposal.md — Why. The mechanics that constrain the approach:

**Sign-in.** `website/server/lib/auth.ts` builds better-auth (1.6.x) lazily per isolate with email and password, GitHub as a social provider, and the `twoFactor` and `passkey` plugins, over a drizzle adapter (`provider: "sqlite"`) on the Turso engine reached through `@tursodatabase/serverless/compat`. Every better-auth endpoint is served by one catch-all, `server/routes/api/auth/[...all].ts`. `user.deleteUser` is not enabled, so `/delete-user` currently answers 404. No email transport is configured: there is no `sendVerificationEmail`, no `sendResetPassword`, and no email credential in `website/secretspec.toml`.

**Schema.** `website/server/database/schema.ts` declares `user`, `session`, `account`, `verification`, `two_factor`, and `passkey`. `session`, `account`, `two_factor` and `passkey` reference `user.id` with `onDelete: "cascade"`; `verification` has no user column — its rows are keyed by an `identifier` and carry a `value`, which for better-auth's deletion token is the user id. The schema is applied with `drizzle-kit push` (`just website::db push`); there is no migrations directory.

**What better-auth's deletion does.** `/delete-user` checks the password or the session's age against `session.freshAge` (default one day), runs `beforeDelete(user, request)`, then removes the user's sessions, `account` rows and `user` row through its internal adapter, then runs `afterDelete`. It does not touch `two_factor` or `passkey` rows itself; those go only if the database propagates the foreign-key cascade.

**Pages.** `app/pages/account/index.vue` is an empty stub. `app/pages/auth/[...slug].vue` renders sign-in or sign-up and, once a session exists, navigates to `/account` unconditionally. `app/composables/useAuth.ts` wraps the better-auth Vue client with `twoFactorClient({ twoFactorPage: "/auth/2fa" })` — but `/auth/2fa` falls into the same catch-all and renders the sign-in form, so an account with a second factor enabled could not finish signing in today. There is no route middleware. `nuxt.config.ts`'s route rules prerender `/download/**` and `/releases/**` and nothing under `/account`.

**Tests.** `just website::test` runs vitest with no config file; the one suite (`shared/utils/releases.test.ts`) is pure. `@libsql/client` is already a dependency, and the production driver is `drizzle-orm/libsql`.

**Identity.** `crates/outpost-protocol/src/identity.rs` derives an identity's Ed25519 signing key from a BIP-39 mnemonic; `MemberID` is that public key, 32 bytes, displayed as 64 lowercase hex digits. The protocol domain-separates every identity signature: `outpost/vessel-cert/v1` for device certificates, `outpost.join.v1` and `outpost.order.v1` in the ACL, `outpost.keychain.v1` as AEAD associated data.

**The app.** `src/app/pages/settings/**` is scaffolded from the Nuxt UI dashboard template with placeholder data, and the app ships `tauri-plugin-shell`, which can open a URL in the system browser.

## Goals / Non-Goals

**Goals:**

- One server implementation of every account operation, reachable by any client holding a session.
- A deletion cascade whose completeness is enforced by a test rather than by remembering.
- An identity binding whose signature can never be mistaken for, or replayed as, a protocol signature.
- Persistence tests against a real database engine.

**Non-Goals:**

- Choosing how the app will present a session to the API. The API reads the session through better-auth, so whichever transport better-auth accepts is the one the API accepts; adopting a bearer transport is a later change.
- Rate limiting beyond better-auth's built-in limiter. Challenge issuance is session-gated and the identity cap bounds what a session can create.

## Decisions

### Accounts serve billing and hosted services; identity stays a keypair (2026-10-26)

An account exists so that things which need a server-side principal — paying, using a hosted service — have one. The app needs none of it: everything local, including sync between a user's own devices, keys on the identity. So the account never becomes a precondition, and the identity never learns about the account. The two meet only through a binding the account records: _this public key proved control to this account_.

Rejected: deriving the account from the identity (signing in with the identity key). It makes the keypair the login, so a lost recovery phrase becomes a lost account, and it puts the website in the business of authenticating with protocol keys. Also rejected: storing the identity on the account at sign-up. The website would have to be told the key without proof, and a user with no identity yet could not create an account.

### Binding is a signed challenge with a pending state for sessionless responses (2026-10-26)

The website issues a challenge bound to the account; the identity holder signs a message built from the challenge's fields; the website verifies with Ed25519 (WebCrypto's `Ed25519` in the Worker) and stores the public key. The full message layout is in the spec.

The message names the origin, the account id and the account's email so that a signature is useless anywhere else, and so the signer can show the user _which_ account it is about to link before signing. The signer constructs the bytes itself from the challenge's fields, never signing bytes the server hands it: if it signed opaque server bytes, a hostile server could have it sign a Notice. The tag `waypoints/account-binding/v1` shares no prefix with the protocol's `outpost` tags.

A challenge id is a capability: whoever holds it can answer. That is fine when the answer comes over a session for the same account. When it does not — the path reservation hands the challenge to an app that may have no session — a leaked challenge would let a stranger bind _their_ identity to the victim's account. So a sessionless answer only proposes: the identity is listed as pending with a fingerprint, and the account holder confirms it on the website, comparing the fingerprint with what the app shows. A session-bearing answer skips the step because the account holder is already the one answering.

Rejected: requiring a session for every response. The app has no session yet, so binding would be untestable end to end until a later change. Also rejected: no confirmation step. It is simpler and wrong for the leaked-capability case above.

### The account API wraps only what Waypoints defines (2026-10-26)

`/api/v1/account/*` holds the operations whose semantics are Waypoints' own — the summary, identities, both deletions. Password, passkeys, second factor, sessions and linked providers stay at better-auth's `/api/auth/*` paths, and the contract names them. A test calls `auth.api` for each named endpoint (or enumerates the router's paths) and fails if one is missing, so a better-auth upgrade that renames one is caught in CI rather than by a client.

Rejected: wrapping every better-auth endpoint under `/api/v1/account`. It doubles the surface, and each wrapper either passes through (adding nothing) or diverges (adding bugs) — passkey registration in particular is a WebAuthn ceremony whose client library talks to better-auth's paths directly. Also rejected: a better-auth plugin hosting the Waypoints endpoints. It would put Waypoints' contract inside better-auth's namespace and versioning.

### Two UIs share one API, not a component layer (2026-10-26)

The website's account pages and the app's future settings pages are separate implementations against one contract. A shared Nuxt layer would couple the website's Docus/Nuxt Content stack to the app's Tauri-hosted Nuxt build, and the two already diverge in component conventions (the app's settings pages come from the dashboard template). The contract is the shared artifact: its request and response types live in `website/shared/` so the website's pages and its server agree at compile time; the app may vendor or generate them later.

Rejected: a shared Nuxt layer now. Revisit once the app's account UI exists and the duplication is concrete rather than predicted.

### One purge registry, two scopes (2026-10-26)

Each account-scoped store registers `{ table, scope: "data" | "account", describe, purge(db, userId) }` in one ordered registry in `server/utils/`. Data deletion runs the `data` purgers in one database batch. Account deletion runs as follows: the account API checks the gate, then calls `auth.api.deleteUser` server-side, whose `beforeDelete` runs every purger — `data` first, then `account` (passkeys, second-factor rows, verification rows whose value names the user) — in one batch. Then better-auth removes sessions, provider accounts and the user.

The ordering is the invariant that matters. better-auth's removals are separate statements, so the whole is not one transaction. Purging first means the only reachable partial state is an account with less data, which a retry finishes. The reverse order could leave rows that no account can reach and no retry can find.

Purgers delete explicitly rather than trusting `onDelete: "cascade"`. SQLite-family engines enforce foreign keys only when asked, and whether the Turso engine does on this connection is not something the cascade should depend on. The explicit purge also makes the cascade the same code path in tests and production.

Completeness is a test. It walks every table in the drizzle schema, finds each column that references `user.id` (and the tables known to key on the user some other way, `verification` among them), and fails naming any table that is neither in the registry nor removed by better-auth itself. The `describe` strings feed the public page, so the page's list of what is deleted is the registry's list.

Data deletion today removes identity bindings, pending identities and challenges. Account-scoped blob stores join by registering a `data` purger. Nothing in this change creates one, so the registry ships with only the stores that exist. A store holding data whose removal needs more than a row delete — bytes in an object store, say — registers a purger that does that work. The registry does not care what a purger touches.

Rejected: putting the cascade in `afterDelete`. It runs after the user row is gone, which is exactly the orphaning order. Also rejected: soft-delete with a background sweeper. It needs scheduled infrastructure the Worker does not have, and it makes "nothing is retained" false for as long as the sweep takes.

### Re-authentication is session recency, not email (2026-10-26)

The website cannot send email, so "confirm by email" is not available. The gate is instead: the session was created within the last ten minutes, enforced by a server guard on the account API's destructive endpoints. A better-auth `hooks.before` matcher applies the same guard to `/two-factor/disable`. Signing in again takes whatever the account requires — password, GitHub, passkey, plus the second factor if enabled — so the gate is as strong as the account's own sign-in. Account deletion also requires typing the account email, which guards against a misclick rather than an attacker.

better-auth's global `freshAge` stays at its one-day default. Lowering it would also tighten every other fresh-session check better-auth makes, which this change has no reason to touch. The direct `/delete-user` and `/delete-user/callback` paths are refused in the catch-all, so the stricter guard cannot be bypassed by calling better-auth directly. `auth.api.deleteUser` called from the server does not pass through the router and is unaffected.

`twoFactor({ allowPasswordless: true })` lets an account with no password enrol a second factor. Without it, a GitHub-only account could never enrol one. better-auth still requires the password when one exists.

Rejected: adopting an email provider here to get email confirmation. It is a new external dependency and secret, and it would pull in verification, reset and change-email flows that deserve their own change. Recency plus re-sign-in meets the same goal: proof that the person at the keyboard controls the account now.

### The public deletion page is prerendered and session-independent (2026-10-26)

`/account/delete` is prerendered. Its content comes from static copy plus the registry's descriptions, which live in a `shared/` module so the page can import them at build time. It reads no session, so a prerendered copy is the page for everyone. Its sign-in link is `/auth/login?redirect=/account/danger`. The sign-in page honours `redirect` only when it is a same-origin path, which closes off open redirects. Every other `/account/**` route is excluded from prerendering, and its pages redirect to sign-in when the client has no session; the API's 401 is the actual gate.

The page meets Play's web-resource expectations: it names the app as listed, features the deletion steps prominently, lists the data deleted, states that nothing is retained, works without the app, and gives a route for a user who cannot sign in. That last route matters more than usual here: with no email transport there is no password reset, so a user who forgets their password has no self-service way back in.

### The app's in-app path is a link out (2026-10-26)

Play accepts an in-app link to the deletion web resource in place of an in-app flow. The app's settings gain one entry that opens `https://<site>/account/delete` through the shell plugin's system-browser open. Nothing about it needs an account or a session, and nothing is embedded, so the app takes on no auth code to satisfy the policy.

## Risks / Trade-offs

- [Data deletion and account deletion are not one transaction with better-auth's removals] → Purge-first ordering plus idempotent purgers make every partial state retryable; the spec makes this an invariant and a test exercises an interrupted deletion.
- [A forgotten-password user cannot self-serve deletion] → The public page names a manual request route; a later email change adds reset.
- [The recency window annoys a user who signed in eleven minutes ago] → The refusal code drives an immediate re-sign-in that returns to the same page; the action itself is not lost.
- [A better-auth upgrade changes `/delete-user`'s internals, for example starting to delete passkeys itself or changing hook order] → Purgers are idempotent, so double deletion is harmless. The interrupted-deletion and completeness tests run against the real engine on every upgrade.
- [Pending bindings from leaked challenges clutter the list] → Pending entries lapse after ten minutes and count toward the identity cap only while pending.
- [Ed25519 in WebCrypto differs subtly from `ed25519-dalek` on edge-case keys] → Small-order and non-canonical keys are rejected before verification. A later app-side change adds a cross-language vector signed by `outpost-protocol`'s key derivation.
- [`/account/**` excluded from prerendering, but Nuxt's route-rule matching gives `/account/delete` its own rule] → The task verifies the built output prerenders exactly that one account route.

## Migration Plan

The schema gains two tables (`identity_binding`, `identity_challenge`) and changes no existing one. Apply them with `just website::db push` before the deploy that serves the new API. `push` is additive here, and better-auth's tables are untouched.

Enabling `user.deleteUser` and closing its direct paths ship together, in the same deploy. Rollback is a redeploy of the previous Worker. The new tables are inert without the routes that read them, and no existing data changes shape.

After `/account/delete` is live, the Play Console's store listing and Data safety form gain its URL. That step is manual and sits outside the repo.

## Open Questions

**Can one account bind several identities? (open — recommendation: yes, with a cap of 8; an identity binds to at most one account.)** Several identities per account covers a user who keeps separate identities and, more importantly, one who loses a recovery phrase: they can bind the new identity before unlinking the old one, so whatever the account carries never falls into a gap. Holding each identity to one account keeps any future per-identity attribution unambiguous. A user who wants an identity on a different account proves control of both by unlinking it first. The spec is written to this recommendation. If John decides one-identity-per-account instead, the "An identity belongs to at most one account" requirement changes, the "binds a second identity" scenario flips to a refusal, and the cap disappears. Nothing else moves.

**Should deletion have a grace or undo window? (open — recommendation: no, not now.)** Today the server holds nothing a user cannot recreate. Identities live on devices, a binding is re-made in a minute, and an account-scoped blob is a copy of something the user has. Play also requires any retention to be disclosed, and "deleted immediately, nothing retained" is the simplest disclosure that is true. A grace window needs a pending-deletion state every endpoint must respect and a scheduled executor the Worker does not have. The typed-email confirmation and the recency gate already stop the misclick and the stolen-session cases a grace window would catch. Revisit when the account holds something irreplaceable or costly, such as billing state. That change would also bring the scheduled infrastructure. The spec's "no retention period" clause is what would change.

**Where does someone who cannot sign in send a deletion request?** The page needs a concrete route (an address or form). The wording is deferrable; the requirement that one exists is not.
