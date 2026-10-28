## Context

See proposal.md — Why. The mechanics that constrain the approach:

**Derivation.** `outpost-protocol/src/identity.rs` derives everything from a BIP-39 mnemonic through SLIP-10 Ed25519 (hardened only). `KeyRoot` holds the 64-byte BIP-39 seed (PBKDF2 of the phrase, empty BIP-39 passphrase). `KeyRoot::identity()` derives the signing node at `m/44'/9297'/0'` and takes its child `0'` as the KEM seed — so the KEM seed is the 32-byte node key at **`m/44'/9297'/0'/0'`**. Device keys sit at `m/44'/9297'/1'/n'` and are certified by the identity signing key. Every device holds the same identity signing key and KEM seed. `identity.rs` already assigns "changeable protection" to "the at-rest wrapping layer (app custody, escrow), not … derivation", which is why there is no BIP-39 passphrase parameter.

**What the KEM key opens.** `crypto.rs`: `KEMSecret` _is_ the 32-byte X-Wing seed; it expands to a decapsulation key per use (`x_wing::DecapsulationKey::from(seed)`) and exposes no expanded form. `acl.rs`: every read key reaches a member as a box sealed to its KEM public key — `Root.epoch0_key_wrapped`, `Approve.current_key_wrapped`, `Remove.new_key_wrapped` — and `Remove.prev_key_wrapped` chains each epoch's key under the next. A holder of the KEM seed therefore opens the current key and walks back through every epoch the identity was ever admitted to. It cannot sign: Notices, ACL records and `DeviceCert`s need the identity signing key, and a Station authenticates a device by its identity-signed cert. `sync-protocol`'s main spec does not state any of this as a requirement today; this design relies on the code.

**App custody.** `cartograph/src/sync/service.rs` stores the phrase in `mnemonic.txt` (0600, plaintext) and re-derives `KeyRoot` from it on each use. `identity_create` refuses to overwrite; `identity_import` validates the phrase, writes it, and picks a random hardened device index. `identity_mnemonic` returns the phrase across IPC — the Settings → Sync reveal and the Emergency Kit (compiled in the webview from `src/app/utils/emergency-kit/`) both receive it. The pairing payload also carries the phrase. There is no mnemonic rotation and no identity removal.

**Website.** better-auth on Turso via drizzle (`website/server/lib/auth.ts`, `server/database/schema.ts`); every account-owned table references `user.id` with `onDelete: "cascade"`. There is no rate-limit facility, no identity binding, and no way for the app to hold an account session. The sibling `account-surface` draft adds the account API under `/api/v1/account/*`, identity binding by signed challenge (several identities per account, each identity on at most one account), a ten-minute session-recency guard on destructive actions, and a deletion registry in which every account-scoped store enrols an explicit purger under a _data_ or _account_ scope, with a test that fails on any table referring to `user.id` outside it. It explicitly leaves how a session reaches the app to a later change, and no open change supplies one. Schema reaches the database through `just website::db push`; there is no migrations directory.

**Crates already present.** `chacha20poly1305 0.11`, `bip39 2`, `hmac 0.13`, `sha2 0.11`, `zeroize` are workspace dependencies. `argon2 0.5.3` is in `Cargo.lock` only transitively (through `pgp`); no workspace crate depends on it directly. There is no password-strength estimator and no HTTP client in `cartograph`.

## Goals / Non-Goals

**Goals:**

- One wrapping format, implemented once in Rust, shared by both tiers and by restore, with known-answer vectors pinning it.
- A server that is a store with access control, so its compromise is an offline-guessing problem bounded by the passphrase and the recorded derivation cost.
- A restore path that reuses `identity_import`, so an escrowed identity is indistinguishable afterwards from one typed in.

**Non-Goals:**

- Hardening `mnemonic.txt` or the existing reveal/Emergency Kit flows, which already hand the phrase to the frontend. This change adds no new path by which key material reaches the frontend; it does not remove the existing ones.
- Defending against a compromised device. A device that can read `mnemonic.txt` already holds everything escrow protects.

## Decisions

### Escrow is off by default and enabled per tier (2026-10-26)

No blob exists until the user enables a tier, and each tier is a separate confirmation with its own copy. The copy states three things: what the tier contains; that Waypoints can store, return and delete it but cannot open it; and what a passphrase holder can do. The read-only copy says plainly that it grants reading _from now on_, not only of history, because the KEM key never rotates while the identity lives.

Rejected: enabling the full tier during onboarding with an opt-out. The point of an identity that is a keypair rather than an account is that Waypoints holds nothing; inverting that should be a decision the user makes after onboarding, not a default they skip past. Rejected: a single switch for both tiers. They grant different things, and the read-only tier exists precisely so that one can be held without the other.

### The full tier wraps the mnemonic's BIP-39 entropy (2026-10-26)

The plaintext is `Mnemonic::to_entropy()` — 16 bytes for 12 words, 32 for 24 — and restore rebuilds the phrase with `Mnemonic::from_entropy()`, then hands it to the `identity_import` path.

App custody is phrase-shaped end to end: `mnemonic.txt`, `identity_import`, `identity_mnemonic`, the pairing payload and the Emergency Kit all carry the phrase. The 64-byte seed is PBKDF2 output and cannot be turned back into a phrase, so a device restored from the seed could not reveal its recovery phrase, print a kit, or pair another device — a second-class identity. Entropy over the phrase string: it is canonical and fixed-length, independent of word-list language, case and whitespace, and reconstructs the identical phrase.

Rejected: the 64-byte seed (above; also four times the plaintext for no gain). Rejected: the phrase as UTF-8 text — round-trips, but carries whitespace and normalization variance into the plaintext for no benefit.

### The read-only tier wraps the 32-byte KEM seed (2026-10-26)

The plaintext is the seed at `m/44'/9297'/0'/0'`, the same bytes `KEMSecret` holds. In this codebase the seed _is_ the X-Wing private key — `KEMSecret` stores nothing else and expands on use — and the expanded decapsulation key (ML-KEM-768's 2,400 bytes plus X25519) is a pure function of it. Wrapping the expansion would be larger, would tie the format to `x-wing`'s pre-release internal layout (`=0.1.0-rc.0`), and would add no capability.

To export the seed, `KEMSecret` gains one accessor returning `&[u8; 32]`; nothing else in `outpost-protocol` changes. Restore of a read-only blob in tests goes through the existing `KEMSecret::from_seed`.

Rejected: the derived X-Wing decapsulation key (above). Rejected: the SLIP-10 node including its chain code — the chain code would let a holder derive children of the KEM node, which no current path uses but which is more than "read only" needs to grant.

### Argon2id into XChaCha20-Poly1305, with everything bound as associated data (2026-10-26)

- **Key**: Argon2id (v0x13), 32-byte output, over the passphrase after NFKC normalization, under a 16-byte random salt fresh for every blob.
- **Cipher**: XChaCha20-Poly1305 with a 24-byte random nonce — the primitive `crypto.rs` already uses for payloads and key wraps, so no new AEAD enters the tree.
- **Associated data**: `waypoints.escrow.v1` ‖ tier byte ‖ identity public key (32) ‖ Argon2 memory KiB, iterations, parallelism (each u32 big-endian) ‖ salt. Changing any stored field makes the tag fail; the server cannot relabel a tier, move a blob between identities, or alter parameters without detection.
- **After opening**: for the full tier, the device derives the identity from the plaintext through `KeyRoot` and refuses the blob unless its public key equals the one in the AAD. A KEM seed cannot derive the signing key, so for the read-only tier the AAD binding is the identity check; where the device also holds the identity (as in the round-trip tests), it compares the opened KEM public key with `Identity::public_key()`.
- **Randomness**: every randomized function takes the RNG as a parameter, as `outpost-protocol` does, so known-answer tests are deterministic.

The module lives at `cartograph/src/sync/escrow.rs`. `service.rs` already declares key custody an app concern, and `identity.rs` names escrow as part of the wrapping layer rather than derivation.

Rejected: placing the format in `outpost-protocol`. It would keep the KEM seed accessor private, but it would compile a memory-hard KDF into every Station and put an account-addressed storage format in the crate whose design rule is that nothing in protocol data references an account. Rejected: one Argon2 run per passphrase with HKDF subkeys per tier — saves one derivation on enable but couples the tiers through a shared intermediate; independent salts give the same separation with no extra mechanism. Rejected: AES-256-GCM (a second AEAD for no reason), scrypt (weaker trade-off and no stronger argument than Argon2id, which RFC 9106 standardizes), PBKDF2 (not memory-hard).

### Argon2id parameters: default 128 MiB / t=3 / p=4, floor 64 MiB / t=3 / p=1, ceiling 512 MiB (2026-10-26)

Parameters are fixed per format version rather than calibrated per device. A blob wrapped on a desktop may be opened on a phone, and calibration lets the weakest device choose the security level. The default is twice RFC 9106's second recommended option (64 MiB, t=3, p=4) in memory; the floor is that option's memory and passes with parallelism relaxed to 1 for very low-memory devices; the ceiling (memory ≤ 512 MiB, iterations ≤ 10, parallelism ≤ 8) keeps a hostile server from making a restoring phone allocate without bound. The recorded parameters let a later format raise the default without invalidating existing blobs; a device re-wraps at the current default whenever it replaces a blob.

Rejected: RFC 9106's first recommendation (2 GiB, t=1). Not runnable inside a mobile app's memory budget.

### The passphrase is generated by default and floored when chosen (2026-10-26)

- **Generated**: seven words from the EFF large wordlist (7,776 words; ≈90.5 bits), hyphen-joined. The EFF list, not BIP-39's, and never 12/15/18/21/24 words, so `setup_code::classify` can never route an escrow passphrase as a recovery phrase and the user is not shown two lists of BIP-39 words that mean different things.
- **Chosen**: accepted when `zxcvbn`'s estimate is ≥ 10^14 guesses (≈46.5 bits) with the account email and display name passed as user inputs, and refused outright if it parses as a BIP-39 mnemonic. At the floor parameters and a thousand guesses per second across an attacker's hardware, 10^14 guesses is about 3,000 years; the estimate is a heuristic, so the floor is set well above what the derivation alone would require.
- **Not the account password**: the passphrase never reaches the server, so the server cannot check equality and must not be asked to. The app never prefills or offers the password, and when `account-surface`'s app sign-in passes the password through the Rust core, the core compares in memory during that session. Otherwise the generated default is the enforcement.

Rejected: a minimum length or character-class rule — rewards `Password1!`-shaped inputs. Rejected: server-side comparison against the password hash — would send the passphrase to the server. Rejected: no custom passphrases — users who keep a password manager should be able to store an entry of their own.

### The website is a dumb, access-controlled store (2026-10-26)

One table, `escrow_blob`: `user_id` (FK → `user.id`), `identity_public_key`, `tier`, `format_version`, `kdf_memory_kib`, `kdf_iterations`, `kdf_parallelism`, `salt`, `nonce`, `ciphertext`, `created_at`, `last_fetched_at`, unique on (`user_id`, `identity_public_key`, `tier`). Stores are upserts on that key; deletes are hard. A second table, `escrow_fetch`, holds the fetch-limit window per user. Both enrol in `account-surface`'s deletion registry as _data_ purgers that delete explicitly — the registry's rule, because whether the Turso engine enforces foreign keys on this connection is not something removal should depend on — and the unlink endpoint deletes the unlinked identity's rows in the same batch as the binding. Ciphertext length is bounded per tier (full: 16 or 32 bytes of entropy plus the 16-byte tag; read-only: 32 plus 16), salt is exactly 16 bytes and nonce exactly 24, so the table cannot be used as general storage.

Routes sit under the account API, nested by identity because an account may bind several: `GET /api/v1/account/escrow` (status for every bound identity and tier, no ciphertext), and under `/api/v1/account/identities/{member}/escrow/{tier}` — `PUT`, `GET` (ciphertext; recent sign-in; rate-limited; updates `last_fetched_at`), `DELETE`. They use `account-surface`'s session guard and error-code shape, refuse a `{member}` not bound to the session's account with the same response as a missing blob, and set `Cache-Control: no-store`. The identity in the associated data is the path's `{member}`; a body naming a different identity is refused.

`last_fetched_at` is beyond the brief's minimum field set. It is the one signal a user can see if someone else fetches their blob, and costs a column.

Rejected: requiring an identity signature on every store, to stop a session-only attacker overwriting the blob. Such an attacker can delete it anyway, and a blob wrapped under a passphrase other than the user's fails to open on restore, so the signature buys nothing escrow promises. Rejected: soft deletion — a retained ciphertext is a retained offline-guessing target.

### Fetches are limited to 10 per account per rolling hour, counted in the database; recent sign-in only (2026-10-26)

The limit does not bound offline guessing — one fetch gives an attacker everything they need — and is not meant to. It bounds harvesting and makes repeated fetches visible. The counter lives in Turso beside the blob, so it is consistent, testable on the real engine, and deleted by the same cascade. Ciphertext fetches reuse `account-surface`'s recency guard — session created within the last ten minutes — rather than defining a second window; restore fetches immediately after sign-in, so a legitimate restore never meets it.

Rejected: Cloudflare's Workers rate-limiting binding — its period is 10 or 60 seconds and its counters are per location, too short and too loose for an hourly per-account limit. Rejected: KV — eventually consistent.

### Restore reuses the import path and checks the identity before writing (2026-10-26)

`restore_from_escrow(passphrase)` fetches the full blob, refuses parameters outside floor/ceiling, derives, opens, rebuilds the phrase, derives `KeyRoot`, and compares its `MemberID` to the blob's identity public key. Only then does it call the same code `identity_import` calls, which writes `mnemonic.txt` and chooses a random device index. A device with an identity refuses (`SyncError::IdentityExists`, as `identity_import` does). Plaintext buffers are `Zeroizing`.

Rejected: a separate write path for escrowed identities. Two paths to `mnemonic.txt` would drift.

### Changing the passphrase re-wraps every enabled tier (2026-10-26)

The device holds the identity, so it re-wraps from local key material, not by opening the old blobs, and needs no old passphrase. Each tier gets a fresh salt and nonce and is replaced independently; the result is reported per tier. Mnemonic rotation is not a thing in this codebase — `identity_create` refuses to overwrite and a different phrase is a different `MemberID` — so there is nothing to re-escrow on rotation. A user who starts a new identity binds it beside the old one and enables escrow for it; unlinking the old identity deletes its blobs.

### Passphrase wrapping, not passkey (WebAuthn PRF) wrapping (2026-10-26)

`outpost-design.md` §14.1 records passkey-wrapped blobs as the carried direction for hosted custody. Not taken here: PRF support differs across the Tauri webviews (WKWebView, WebView2, Android WebView) and their passkey providers; the PRF output is produced in the webview's JavaScript, so key material would enter the frontend; and a user who has lost every device has usually lost a device-bound passkey too, unless a platform vendor syncs it — which moves the trust to that vendor. A passphrase is portable and stays in Rust. The format's versioned KDF fields leave room for a PRF-derived key later.

## Threat model

- **The website database is copied.** The attacker gets, per user, ciphertext, nonce, salt, parameters, tier, identity public key and the account it belongs to (the account–identity link `account-surface` already stores). Each passphrase guess costs one Argon2id at the recorded parameters, ≥ 64 MiB and three passes. A generated passphrase (≈2^90) is out of reach; a chosen one is ≥ 10^14 guesses by estimate. Weak points: the estimator can overrate a passphrase; one blob per user means per-user attack cost, not per-database.
- **Waypoints itself, or someone controlling the website.** Can delete, withhold, or serve an old blob; can lower parameters (the device refuses below the floor, and the tag fails anyway); can relabel tier or identity (the tag fails); can serve a blob it made under a passphrase it knows (the user's passphrase fails to open it, so nothing is restored). Cannot read a blob. The website never asks for the escrow passphrase, and the app's copy says so.
- **The account is taken over without the passphrase.** One fetch reduces to the database case for that user. The attacker can delete or overwrite (availability). `last_fetched_at` in status and the 10-minute freshness gate make silent fetching harder, not impossible.
- **The passphrase is phished.** Useless without the blob, which needs the account (password plus second factor, if enabled) or the database. Phishing both is a full compromise of the full tier. The copy says the passphrase is entered only in the app, never on the website.
- **A device is compromised.** Out of scope: `mnemonic.txt` is already readable there. Escrow adds exposure only at the moments the passphrase is typed.
- **A read-only blob is opened.** The holder reads every epoch the identity has been admitted to, and every future epoch until the identity is removed from that Territory. Deleting the blob does not revoke an opened copy; only a new identity rotates the KEM key.
- **Explicitly not protected: availability.** Waypoints can delete or withhold a blob at any time. Escrow is a second path back, not a replacement for the recovery phrase; the copy says to keep the Emergency Kit.

## Risks / Trade-offs

- **[A user forgets the passphrase and has deleted the Emergency Kit]** → Data loss, stated plainly in the enabling copy. No recovery exists by design.
- **[Immediate deletion destroys the only remaining copy]** → Once a user's devices are gone, the full-tier blob is irreplaceable, and data deletion, account deletion and unlinking all remove it at once. The deletion copy names escrow explicitly so the user is told what they are giving up; whether this warrants a grace window is `account-surface`'s open question, and this change does not add one.
- **[Argon2id at 128 MiB is slow or killed on a low-end phone]** → Floor at 64 MiB/p=1 and ceiling at 512 MiB bound it; restore shows progress; tasks measure on the slowest supported Android target before the default is final.
- **[`zxcvbn` overrates a chosen passphrase]** → Generated is the default; the 10^14 floor is well above what the KDF alone would need.
- **[The two tiers drift to different passphrases after a partial change]** → Reported per tier; the next change re-wraps both.
- **[No change gives the app an account session]** → Escrow's device side cannot reach the website until one does, and if the session ends up in the webview rather than the Rust core, the Rust client has no credential to present. Sections 1–4 of tasks.md stand without it; sections 5.1 and 7–8 wait on it.
- **[Website tests run on a local engine that is not the Turso engine]** → Local tests use the same drizzle driver against a real local database; a divergence in upsert or cascade semantics would surface only against the hosted engine. Covered by a task that runs the same suite against a Turso development database.
- **[The existing reveal and Emergency Kit flows already put the phrase in the frontend]** → Unchanged by this change; noted so the "frontend never sees key material" requirement is read as scoped to escrow operations.

## Migration Plan

No data to migrate.

1. Land the `KEMSecret` accessor and `escrow.rs` with vectors. Nothing calls them.
2. Once `account-surface` has landed, add `escrow_blob` and `escrow_fetch`, their purgers and the routes. `just website::db push` creates the tables; nothing writes to them.
3. Land the service methods and Tauri commands, then — once the app can hold a session — the settings cards and the onboarding restore path.

Rollback: removing the routes leaves the tables inert; removing the UI leaves the service unreachable. Blobs already stored remain readable by a later reinstatement, and a user can delete them through the account deletion cascade regardless.

## Open Questions

Each has a recommendation; the specs and tasks assume it.

1. **KEM seed or derived X-Wing key for read-only?** Seed. The seed is the private key in this codebase's representation; the expansion is larger, derivable, and coupled to a pre-release crate's internals.
2. **Escrow before an account exists, as a local passphrase-wrapped file?** Not in this change. The Emergency Kit already covers offline backup; a wrapped file is an offline-guessing target wherever it is copied, with no account gate, rate limit or deletion; and the self-describing format means a file export can be added later without a format change.
3. **Rotation.** Passphrase change re-wraps on the device (above). Mnemonic rotation does not exist; a new identity is bound beside the old one, and unlinking the old one deletes its blobs. That needs `account-surface`'s "unlinking removes the binding and nothing else" amended.
4. **One passphrase for both tiers?** Allow it. Separate salts keep the keys independent, and the passphrase never leaves the device. A user who wants a read-only passphrase they can share without exposing the full tier picks a different one; the copy says so.
5. **Numbers.** 10 fetches per rolling hour; 10-minute freshness; Argon2id default 128 MiB/t=3/p=4; strength floor 10^14. Each is a constant in one place and can move without changing behavior's shape.
