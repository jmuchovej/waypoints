## Why

The website can create an account — email and password or GitHub, through better-auth over Turso — but a signed-in user lands on `/account`, which is an empty page. Nobody can see what the account holds, manage how they sign in, or delete it. That is a usability gap on the website and a policy blocker on Google Play: an app in scope of Play's account-deletion policy must offer account deletion both inside the app and through a working web resource that needs no app, and the Data safety form asks for that web link and whether data can be deleted on its own. Play counts an app as in scope when it lets a user create an account, including by sending them to a creation flow outside the app — so the moment the app points anyone at the website's sign-up, Waypoints is in scope, and today there is nothing to link to.

The account is also about to carry more than login: it needs a way to know which Waypoints identity belongs to it. Today the website knows nothing about identities, and an identity — an Ed25519 keypair derived from the user's recovery phrase — is deliberately not an account. This change gives the account an explicit, signed binding to identities, so that whatever is later attached to an account has an identity to attach to, without making the keypair depend on the account.

## What Changes

- **An account management surface on the website.** `/account` becomes four sections: **profile** (name, the email the account signs in with, when it was created), **security** (password, passkeys, two-factor, active sessions, linked sign-in providers), **linked identities** (Waypoints identities bound to the account, with unlink), and a **danger zone** (delete data, delete account). The surface requires a session and is never prerendered.
- **A public, unauthenticated `/account/delete` page.** It names Waypoints as it appears on the store listing, states the steps to delete an account, lists what deletion removes and what it never touches, states that nothing is retained after deletion, and links to sign-in with a return to the danger zone. It also names a route for someone who cannot sign in. This is the web resource the Play listing and the Data safety form point at, and the app's settings entry opens.
- **A versioned account API under `/api/v1/account/*`.** Session-authenticated, JSON, and the only place account-level operations Waypoints defines live: the account summary and profile update, identity challenges and bindings, data deletion, and account deletion. The website's pages call it today; the app can call the same endpoints later without a second implementation. Security primitives stay where better-auth serves them (`/api/auth/*`), and the contract names which of those are part of it rather than re-wrapping them.
- **Identity binding by signed challenge.** The website issues a short-lived, single-use challenge bound to the account; the holder of an identity key signs a domain-separated message carrying the challenge; the website verifies the Ed25519 signature and stores the identity's public key on the account. The challenge reaches the app through the path reservation — this change defines the challenge, the signed message, and the verification, not the transport.
- **One deletion cascade, two scopes.** Every account-scoped store registers a purger in a single registry, tagged as either _data_ or _account_. **Delete data** runs the data purgers — linked identities, outstanding challenges, and any account-scoped stored blobs — and keeps the account signed in. **Delete account** runs every purger, then better-auth's own user, session, and provider-account deletion. A test fails if any table referencing the user escapes the registry. Both actions say plainly that data on the user's devices is untouched, because the website never held it.
- **Destructive actions require a recent sign-in.** Deleting data, deleting the account, unlinking an identity, and removing a second factor require that the session was created within a short window; otherwise the user signs in again. Account deletion additionally requires typing the account's email. better-auth's own `/api/auth/delete-user` is closed so the account API is the only route to deletion.
- **A two-factor verification page.** The auth client already redirects to `/auth/2fa` when a sign-in needs a second factor, but that path renders the ordinary sign-in form. Enabling two-factor from the security section without it would lock users out, so the page ships here.
- **Sign-in honours a return path.** The sign-in page sends a signed-in user to a same-origin path given in its query (falling back to `/account`), so `/account/delete` and the re-authentication prompt return the user where they started.

## Capabilities

**No registered spec domain fits, so this change proposes a new one: `accounts`.** The registry's domains are the data model, the editor, the Bundle format, the Tauri shell, local storage, the sync protocol and its UX, distribution, and the toolchain. A website account is none of them:

- `sync-protocol` and `sync` own identities as _keypairs_ — `outpost-protocol` defines an identity as "a keypair, not an account", and a `MemberID` as the identity's public key with "no account behind it" — and their guarantees hold with no website at all. An account binding an identity is a claim the website records about a key, not a protocol object; putting it in `sync-protocol` would make the protocol cite a server it must never depend on.
- `distribution` covers how releases are built and reached. The download surface happens to live on the same website, but accounts are not packaging, installation, or update delivery.
- `app-shell` and `storage` describe the Tauri command surface and local typed storage; the account lives in the website's Turso database and is served over HTTP.

`accounts` is proposed as a tree, like `distribution` and `toolchain`, so later account concerns take their own area beside this one. The registry in `openspec/config.yaml` needs a matching entry (`accounts: website accounts — sign-in, account management, identity binding, deletion. code: website/server, website/app`) before this change archives; this change does not edit the registry itself.

### New Capabilities

- `accounts/account-surface`: what an account is and what a user can do with it. The account's independence from the identity; the account API's contract and its authentication; the security operations the contract adopts from better-auth; identity binding by signed challenge and the shape of the signed message; the two deletion scopes and the registry that guarantees nothing account-scoped survives them; the re-authentication gate on destructive actions; and the public deletion page's obligations.

### Modified Capabilities

None. No published capability's requirements change. Identity derivation and the meaning of a `MemberID` are untouched — the binding consumes an identity's public key and signature, and its domain tag is chosen so that no binding signature is ever a valid protocol signature. `distribution/release-delivery` shares the website and the `/api/v1` prefix but none of its requirements move.

## Non-goals

- **The app's account UI.** The app's settings pages stay as they are. This change states what the app must expose for Play — a settings entry that opens `/account/delete` in the system browser — and which endpoints it may call later, and nothing more.
- **How the app obtains a session.** The API is session-authenticated; how a session reaches the app belongs to the path reservation and a later change.
- **Hosted-sync entitlements, billing, and server-side export.** Each is a separate, later change. The deletion registry is how such stores will join the cascade; none is added here. The app's "Billing" menu item stays unrouted.
- **A shared Nuxt layer or component library between the website and the app.** Two UIs share one API contract; each renders it independently.
- **Email delivery.** The website has no email transport, so this change adds no email confirmation, email change, password reset, or email verification. Re-authentication stands in for email confirmation of destructive actions.
- **Deleting anything on the user's devices.** The website never held local data, cannot reach it, and says so.
- **What is stored in an account-scoped blob.** The cascade purges such blobs generically; their contents, encryption, and API belong to the change that introduces them.
- **A deletion grace period.** Deletion is immediate once confirmed; see design.md's open questions.
- **Clearing the website's pre-existing typecheck errors.** `just website::typecheck` reports roughly a hundred Nuxt UI typing errors that predate this change; the change adds none and fixes none.

## Impact

- `website/server/lib/auth.ts` — `user.deleteUser` enabled with a `beforeDelete` that runs the account-scope cascade; two-factor's `allowPasswordless` so GitHub-only accounts can manage a second factor.
- `website/server/routes/api/auth/[...all].ts` — refuses `/delete-user` and `/delete-user/callback`, so deletion is reachable only through the account API.
- `website/server/database/schema.ts` — two tables: identity bindings (public key unique across accounts) and identity challenges. Applied with `just website::db push`, as the existing schema is.
- `website/server/routes/api/v1/account/**` — the account API.
- `website/server/utils/` — the session guard with its recency check, the purge registry, and the challenge/verification module; plain functions over the database handle so they test without Nuxt.
- `website/app/pages/account/**` — the four sections and the public `/account/delete`; `website/app/pages/auth/` — the two-factor page and the return-path handling; `website/nuxt.config.ts` — route rules (prerender `/account/delete`, never the rest of `/account/**`).
- `website/` tests — vitest suites for the cascade, the binding verifier, and the guard, run against a real libSQL engine in memory through the same drizzle driver production uses.
- Play Console, outside the repo: the store listing and the Data safety form gain the `/account/delete` URL once it is live.
- `openspec/config.yaml` — the `accounts` registry entry, at archive time.
