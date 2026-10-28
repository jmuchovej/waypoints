## Why

A Waypoints identity is a BIP-39 mnemonic, and that mnemonic is the only backup there is: `KeyRoot::generate` hands the phrase back exactly once, the app writes it to `mnemonic.txt` on the device that minted it, and the Emergency Kit is a printout of the same phrase. A user who loses every device and the printout loses the identity, and with it every Territory they belong to — no Waypoints server can help, by design. This change gives that user a second, opt-in way back that keeps the design's promise: Waypoints stores a copy of the key material that only the user's passphrase can open.

It also separates two things the mnemonic currently fuses. Reading history needs only the identity's KEM key, which unwraps every epoch read key sealed to the identity; authoring needs the signing key. Escrowing the KEM seed on its own yields a key-holding artifact that can be handed to something that should read but never sign.

## What Changes

- **Escrow is off until the user turns it on, one tier at a time.** Nothing is uploaded at onboarding or by default. Each tier is enabled separately, behind copy that says what Waypoints' servers can do with the blob (store, return to the signed-in account, delete — never open) and what anyone holding the passphrase can do with it.
- **Two tiers.** _Full_ wraps the mnemonic's BIP-39 entropy, which restores the identity outright: signing key, KEM key, every device key. _Read-only_ wraps the identity's 32-byte X-Wing KEM seed alone, which opens every read key sealed to the identity and authors nothing.
- **Wrapping and unwrapping happen on the device, in Rust.** The key is Argon2id over a user passphrase with a per-blob random salt and parameters stored beside the blob; the cipher is XChaCha20-Poly1305, which the protocol crate already uses, with the tier, the identity public key and the KDF parameters bound in as associated data. The Nuxt frontends receive no mnemonic, seed, KEM seed or wrapping key from any escrow operation, and the passphrase never leaves the device.
- **The passphrase has a strength floor and is not the account password.** The app offers a generated passphrase by default; a user-chosen one must clear an estimated-guesses floor computed in Rust. The passphrase is never sent to Waypoints in any form.
- **The website stores ciphertext and nothing it could open.** One row per (account, identity public key, tier): ciphertext, nonce, salt, KDF parameters, format version, tier, identity public key, created-at, last-fetched-at. It is served through the account API, authenticates every request, accepts a blob only for an identity bound to the account, requires a recent sign-in and a rate limit to fetch ciphertext, overwrites on replace, and deletes immediately — on request, when the identity is unlinked, and on data or account deletion.
- **Restoring is an onboarding path.** "Restore from escrow" signs in, picks the identity if the account has more than one with a full-tier blob, fetches that blob, takes the passphrase, unwraps on-device, checks that the restored identity's `MemberID` equals the identity the blob was bound to, and continues through the same import path a typed recovery phrase takes.
- **Changing the passphrase re-wraps on-device** and replaces the stored blob; the key material does not change.

## Capabilities

This change lands in two places, split at the device/server seam.

### New Capabilities

- `accounts/mnemonic-escrow` _(contingent)_: what the website stores for an escrow blob, who may write, read and delete it, the fetch rate limit, overwrite-on-replace, immediate deletion, deletion on unlink, and enrolment in both deletion scopes — and the standing rule that the server holds nothing it can decrypt. **No registered domain fits.** `sync` is client-side by its registry entry (code: cartograph, app, domain) and `sync-protocol` covers Notices, ACL, epochs and the Station/Vessel contracts; neither owns website accounts, and nothing in protocol or Territory data references an account. The sibling change `account-surface` proposes `accounts` as a tree for website accounts, the account API and the account-to-identity binding, with its own area at `accounts/account-surface`. This delta lands at `accounts/mnemonic-escrow` only if that domain is accepted; if it is not, this capability needs a home chosen before this change can archive, and none is minted here.

Two sentences in `account-surface`'s draft spec contradict escrow and need amending there before both changes can stand: "The website SHALL NOT hold an identity's recovery phrase, private key, or any value from which either can be derived" — an escrow blob is exactly such a value, derivable only with a passphrase the website never receives; and "Unlinking SHALL remove the binding and nothing else" — this change deletes the unlinked identity's blobs. Its design's case against a deletion grace window also assumes every account-scoped blob "is a copy of something the user has", which a full-tier blob stops being once the user's devices are gone.

### Modified Capabilities

- `sync`: gains requirements for escrow's device side — default-off, per-tier opt-in, what each tier wraps, on-device wrapping with no key material crossing to the frontend, the passphrase's strength floor and its separation from the account password, the restore-from-escrow onboarding path with its `MemberID` check, and on-device re-wrap on passphrase change. No existing requirement changes. The main `sync` spec currently states nothing about identity custody or onboarding; these are its first requirements on the subject. They rely on a property of `outpost-protocol` that `sync-protocol` does not yet state as a requirement — that read keys reach a member only as boxes sealed to its identity KEM key, and that signing needs the separate identity signing key — so the design cites the code (`acl.rs`, `crypto.rs`, `identity.rs`) for it rather than a spec.

## Non-goals

- **A service that consumes the read-only tier.** This change creates, replaces and deletes read-only blobs and proves they round-trip; nothing server-side unwraps or uses one.
- **Server-side decryption, ever.** No endpoint, job or administrative path receives a passphrase or a derived key.
- **Passphrase recovery.** Lose the passphrase and the blob is useless; the blob can be deleted and a new one made from a device that still has the identity.
- **Entitlements or billing.** Escrow is not gated on a plan.
- **The account API, the identity binding, and the deletion cascade.** `account-surface` defines all three; this change uses them and adds an escrow store to the cascade.
- **App sign-in.** The app has no way to obtain an account session today, and neither `account-surface` (which leaves "how a session reaches the app" to a later change) nor any other open change provides one. Escrow's app side needs a session held by the Rust core; that is a prerequisite this change names and does not design.
- **Mnemonic rotation.** No path rotates an identity's mnemonic today — a different mnemonic is a different identity — and this change adds none.
- **A local, passphrase-wrapped export file.** The Emergency Kit remains the offline backup; see design.md's open questions.
- **OS-keychain custody of `mnemonic.txt`.** On-device at-rest protection is a separate concern from escrow and is untouched.
- **Passkey (WebAuthn PRF) wrapping.** Considered and rejected for this change; see design.md.

## Impact

- `crates/outpost-protocol/src/crypto.rs` — `KEMSecret` gains one narrow accessor returning its seed for custody layers; nothing else in the protocol crate changes.
- `crates/cartograph/src/sync/escrow.rs` (new) — the escrow format: Argon2id derivation, XChaCha20-Poly1305 wrap and unwrap, associated-data layout, parameter floor and ceiling, passphrase strength check, passphrase generator.
- `crates/cartograph/src/sync/service.rs`, `types.rs`, `error.rs` — enable, replace, delete and status per tier; restore-from-escrow; re-wrap on passphrase change; the Rust HTTP client for the website's escrow endpoints.
- `crates/cartograph/Cargo.toml`, workspace `Cargo.toml` — `argon2` (RustCrypto; already in `Cargo.lock` transitively, not yet a direct dependency), `zxcvbn`, an HTTP client; `bip39` and `chacha20poly1305` become direct cartograph dependencies from the workspace set.
- `src-tauri/src/commands/sync.rs` — thin command wrappers over the new service methods.
- `src/app/pages/settings/sync.vue`, `settings/security.vue`, `pages/onboarding.vue`, and the identity setup component — opt-in cards per tier, passphrase entry, restore-from-escrow.
- `website/server/database/schema.ts` — `escrow_blob` and `escrow_fetch`, applied with `just website::db push` as the existing schema is.
- `website/server/routes/api/v1/account/identities/[member]/escrow/` (new) — status, store, fetch and delete under the account API, using `account-surface`'s session guard and recency check; the fetch rate limit.
- `website/server/utils/` — escrow purgers enrolled in `account-surface`'s deletion registry under the _data_ scope, and deletion of an identity's blobs on unlink.
- Depends on `account-surface` (account API, binding, deletion registry, recency guard) and on an app sign-in that no open change provides.
