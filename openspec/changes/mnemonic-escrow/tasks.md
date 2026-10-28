Every Rust task runs its checks inside the devenv shell with a stated reason, e.g. `SECRETSPEC_REASON="mnemonic-escrow: run cartograph tests; no secret values are read" devenv shell -- cargo nextest run -p cartograph`, and leaves `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run` for the touched crates, and `treefmt --fail-on-change` green before it is checked off. Website tasks leave `just website::test` and `just website::typecheck` green on the files they touch; app tasks leave `just app::typecheck` no worse than `main` (it carries pre-existing errors). Persistence tests run against a real database engine through the same driver the code uses — never against asserted SQL strings. Section 4 starts once `account-surface` has landed; 5.1's HTTP half and sections 7–8 wait on an app sign-in that no open change provides yet.

## 1. Protocol accessor

- [ ] 1.1 Add a `KEMSecret` accessor returning its 32-byte seed by reference, documented as for custody layers only, and verify a unit test in `crypto.rs` that `KEMSecret::from_seed(*secret.seed())` has the same `public_key()` as `secret` and that the accessor is the only new public item in `outpost-protocol` (`cargo nextest run -p outpost-protocol`)
- [ ] 1.2 Add a test in `identity.rs` pinning the KEM seed path: for the all-`abandon … about` phrase, the seed `Identity` uses equals the SLIP-10 node key at `m/44'/9297'/0'/0'`, and verify it fails if the path is changed

## 2. The escrow format

- [ ] 2.1 Add `argon2` (RustCrypto, the release line compatible with the workspace's `chacha20poly1305 0.11` and `zeroize`), and make `bip39`, `chacha20poly1305`, `zeroize` and `unicode-normalization` direct `cartograph` dependencies from the workspace set; verify `cargo tree -p cartograph -i argon2` shows one `argon2` version and `just deny` is clean
- [ ] 2.2 Add `cartograph/src/sync/escrow.rs` with the tier enum, the parameter type with its default, floor and ceiling, NFKC normalization, and Argon2id derivation; verify an RFC 9106 Argon2id test vector reproduces through the module's derivation function, and unit tests show parameters below the floor are refused on wrap and on open, and parameters above the ceiling are refused on open, both before any derivation runs
- [ ] 2.3 Add wrap and unwrap over XChaCha20-Poly1305 with the associated-data layout from design.md, taking the RNG as a parameter; verify round-trip tests for both tiers, for 12- and 24-word phrases (the full tier rebuilds the identical phrase and the same `MemberID`; the read-only tier's opened `KEMSecret` has the identity's `public_key()` and opens a read key sealed with `seal_key_to_member`)
- [ ] 2.4 Commit known-answer vectors — fixed passphrase, seeded RNG, both tiers, at the default parameters and at the floor — as hex in the test module, and verify the test fails if any byte of the associated-data layout, the tier byte or the domain tag changes
- [ ] 2.5 Verify tamper tests: relabelling the tier, substituting another identity public key, changing any parameter, changing the salt, truncating the ciphertext, and opening with a different passphrase each fail to open; a blob wrapped under passphrase A does not open under B even with every other field equal
- [ ] 2.6 Verify that composed and decomposed forms of a passphrase with an accented letter derive the same key, and that plaintext and derived-key buffers are `Zeroizing`

## 3. The passphrase

- [ ] 3.1 Add the passphrase generator — seven hyphen-joined words from the EFF large wordlist, vendored with its licence attribution — and verify tests that its entropy is at least 90 bits, that `setup_code::classify` never returns `Phrase` for 10,000 generated passphrases, and that the generator takes the RNG as a parameter
- [ ] 3.2 Add `zxcvbn` and the strength check — estimated guesses ≥ 10^14 with email and display name as user inputs, refusal of any input that parses as a BIP-39 mnemonic, and a refusal reason for each case; verify unit tests for a passing chosen passphrase, a dictionary phrase that fails, one that fails only because it contains the email, and a valid 12-word mnemonic
- [ ] 3.3 Add the same-session account-password comparison as a check that takes an optional in-memory password, and verify it refuses an equal passphrase when one is supplied and is a no-op when none is

## 4. Website store

- [ ] 4.1 Add `escrow_blob` and `escrow_fetch` to `website/server/database/schema.ts` as specified in design.md, with the unique key on (`user_id`, `identity_public_key`, `tier`), and enrol both in `account-surface`'s deletion registry as _data_ purgers; verify with a vitest suite against a real in-memory libSQL engine through the drizzle driver that a second store for the same key replaces the first, that the two tiers are independent, that data deletion and account deletion each leave no escrow or fetch row with foreign-key enforcement off, and that `account-surface`'s registry-completeness test passes
- [ ] 4.2 Add the store module (upsert, fetch-with-limit, status, delete, delete-by-identity) and verify against the real local database that a hard delete leaves no row, that a fetch beyond 10 in the rolling hour is refused with a retry time and does not move `last_fetched_at`, and that status reads and deletions do not count toward the limit
- [ ] 4.3 Add `GET /api/v1/account/escrow` and `PUT`/`GET`/`DELETE /api/v1/account/identities/{member}/escrow/{tier}` using `account-surface`'s session guard and error-code shape — validating tier, format version and field lengths, refusing a `{member}` not bound to the account as a missing blob, refusing a body naming another identity, gating ciphertext fetches on the recency guard, and setting `Cache-Control: no-store`; verify route tests over the real in-memory engine covering a 401 without a session, a cross-account request indistinguishable from a missing blob, a stale-session fetch refused with the sign-in-again code, a malformed body, and the absence of ciphertext in status
- [ ] 4.4 Make `account-surface`'s unlink endpoint delete the identity's escrow rows in the same batch as the binding, and verify a test that unlinking removes both tiers' rows for that identity and leaves another bound identity's rows intact
- [ ] 4.5 Run the store and route suites once against a Turso development database and record the result in this task, so any engine divergence from the local database in upsert or cascade behavior is caught

## 5. App service

- [ ] 5.1 Add an escrow remote trait in `cartograph` for the routes in 4.3, and — once the app's Rust core can hold an account session — an HTTP implementation presenting it; verify the HTTP implementation against a local website dev server in an `#[ignore]`d integration test that stores, fetches, reads status and deletes a blob
- [ ] 5.2 Add `escrow_status`, `escrow_enable(tier, passphrase)`, `escrow_disable(tier)` and `escrow_change_passphrase(passphrase)` to `SyncService`, returning per-tier status types only; verify service tests against an in-memory remote that a fresh identity has no blobs, that enabling one tier creates only that tier, that a failed delete leaves the tier reported enabled, that a partial passphrase change reports per tier, and that no returned type carries key material
- [ ] 5.3 Add `restore_from_escrow(passphrase)` sharing the write path `identity_import` uses, and verify service tests over a temporary sync root: a correct passphrase writes the identical phrase to `mnemonic.txt` with a fresh device index and reports the same `MemberID`; a wrong passphrase, a `MemberID` mismatch, out-of-range parameters and a missing blob each leave the sync root without `mnemonic.txt`; an existing identity yields `SyncError::IdentityExists` and is untouched
- [ ] 5.4 Verify a test that after each escrow operation no file under the sync root contains the passphrase

## 6. Tauri commands and types

- [ ] 6.1 Add thin `src-tauri/src/commands/sync.rs` wrappers for the service methods in 5.2–5.3 and register them in `lib.rs`; run `just typegen` and verify `just check-generated` reports no diff

## 7. App surfaces

- [ ] 7.1 Add the escrow section to `src/app/pages/settings/sync.vue`: one card per tier with the consent copy from the `sync` spec, generated-by-default passphrase with reveal-and-confirm, chosen-passphrase entry with the core's refusal reasons, last-fetched-at, disable, and change passphrase; verify `just app::typecheck` introduces no new errors and the cards render against the dev server (`cargo xtask dev-server`) with each tier enabled and disabled
- [ ] 7.2 Add "Restore from escrow" to the identity step of onboarding (`SyncIdentitySetup.vue`) beside create and recovery-phrase entry, and verify `just app::typecheck` introduces no new errors and a restore against a local website round-trips an identity created on another sync root
- [ ] 7.3 Verify by reading the IPC payloads in the dev server's request log that no escrow command response contains a phrase, seed, KEM seed or key, and that the passphrase appears only in the request that needs it

## 8. End to end

- [ ] 8.1 On the slowest supported Android target, measure Argon2id at the default and floor parameters, and record the timings and peak memory here; if the default exceeds 3 seconds or the app is killed, lower the default toward the floor in `escrow.rs`, regenerate the vectors in 2.4, and record the change in design.md
- [ ] 8.2 With a local website, create an identity, enable both tiers, change the passphrase, delete the sync root, restore from escrow, and verify the restored `MemberID` matches and the device rejoins its Territory; then delete the account and verify no escrow row remains
