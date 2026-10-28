## Purpose

Governs what a Waypoints account stores when its user opts into key escrow: passphrase-wrapped ciphertext the server cannot open, addressed by account, identity and tier, readable and deletable only by that account, and gone when the account is.

## ADDED Requirements

### Requirement: The server stores only ciphertext it cannot open

An escrow blob SHALL be stored as exactly these fields: account, identity public key, tier, format version, key-derivation parameters, salt, nonce, ciphertext, created-at, and last-fetched-at. The server SHALL NOT accept, store or log a passphrase, a derived key, or any plaintext key material, and no server path SHALL attempt to decrypt a blob. The server SHALL reject a blob whose tier or format version it does not recognize, or whose salt, nonce or ciphertext length falls outside the bounds that tier and version allow, so the store holds escrow blobs and nothing else.

#### Scenario: A stored blob holds only the declared fields

- **WHEN** a blob is accepted
- **THEN** its stored record contains the declared fields and nothing else

#### Scenario: A malformed blob is rejected

- **WHEN** a request carries an unknown tier, an unknown format version, or a ciphertext longer than its tier allows
- **THEN** the request is rejected and nothing is stored

#### Scenario: A database copy reveals no key

- **WHEN** the escrow table is read in full by someone without the passphrase
- **THEN** it yields no recovery phrase, seed or KEM key, and the only path to one is guessing the passphrase through the recorded derivation

### Requirement: A blob is addressed by account, identity and tier

At most one blob SHALL exist for each combination of account, identity public key and tier. Storing a blob for a combination that already has one SHALL replace it entirely; the replaced blob SHALL NOT be retained. The two tiers SHALL be independent: storing, replacing or deleting one SHALL NOT affect the other.

#### Scenario: A replacement overwrites

- **WHEN** a full-tier blob is stored for a combination that already has one
- **THEN** a later fetch returns the new blob, and the earlier blob is retrievable from nowhere

#### Scenario: Tiers are independent

- **WHEN** an account's read-only blob is deleted
- **THEN** its full-tier blob is unchanged

### Requirement: Only an identity bound to the account is escrowed

The server SHALL accept a blob only when its identity public key is an identity currently bound to the authenticated account. A store for an identity that is not bound to the account SHALL be refused. When an identity is unlinked from an account, every blob for that identity under that account SHALL be deleted as part of the unlink.

#### Scenario: A blob for another identity is refused

- **WHEN** an account stores a blob whose identity public key is not its bound identity
- **THEN** the request is rejected and nothing is stored

#### Scenario: Unlinking deletes the identity's blobs

- **WHEN** an identity is unlinked from an account
- **THEN** every blob for that identity under that account is deleted, and blobs for the account's other identities are unchanged

### Requirement: Every escrow request is authenticated to the owning account

Escrow operations SHALL be served by the account API and SHALL follow its contract: a request without a valid session SHALL be refused with status 401, and every refusal SHALL carry a stable machine-readable error code. Storing, fetching, deleting and reading the status of blobs SHALL require a session for the account that owns them. A request SHALL NOT reach another account's blobs, and a response to one account SHALL NOT reveal whether another account has a blob. Fetching a blob's ciphertext SHALL additionally require a recent sign-in, under the same recency rule the account API applies to destructive actions. Status responses SHALL report, per bound identity and tier, whether a blob exists, its created-at and its last-fetched-at, and SHALL NOT carry ciphertext, salt or nonce. No escrow response SHALL be cacheable.

#### Scenario: An unauthenticated request is refused

- **WHEN** an escrow request carries no valid session
- **THEN** it is refused and reveals nothing about any blob

#### Scenario: One account cannot reach another's blob

- **WHEN** an authenticated account requests a blob addressed to a different account
- **THEN** the response is indistinguishable from a request for a blob that does not exist

#### Scenario: A stale session cannot fetch ciphertext

- **WHEN** a session that does not satisfy the recency rule fetches a blob
- **THEN** the fetch is refused with the sign-in-again error code, and status remains readable

#### Scenario: Status shows the last fetch

- **WHEN** an account reads its escrow status after its full-tier blob was fetched
- **THEN** the status reports when that fetch happened

### Requirement: Fetches are rate-limited per account

Fetches that return ciphertext SHALL be limited per account to a fixed number within a rolling window. A fetch beyond the limit SHALL be refused with the time after which fetching is allowed again, and SHALL NOT update last-fetched-at. Status reads, stores and deletions SHALL NOT count toward the fetch limit.

#### Scenario: The limit refuses excess fetches

- **WHEN** an account exceeds its fetch limit within the window
- **THEN** further fetches are refused with a retry time until the window allows them again

#### Scenario: Status is not rate-limited by fetches

- **WHEN** an account has exhausted its fetch limit
- **THEN** it can still read status and delete its blobs

### Requirement: Deletion is immediate and complete

Deleting a blob SHALL remove it at once: a subsequent fetch or status read SHALL find no blob, and no copy, tombstone or soft-deleted record holding its salt, nonce or ciphertext SHALL remain in the escrow store.

#### Scenario: A deleted blob is gone

- **WHEN** an account deletes its full-tier blob
- **THEN** the next fetch finds no blob, and the escrow store holds no record of its ciphertext

### Requirement: Escrow is removed by data deletion and by account deletion

Escrow blobs and fetch-limit records SHALL be enrolled in the account deletion cascade as data, so that both deleting an account's data and deleting the account remove every escrow blob and fetch-limit record belonging to it. Their removal SHALL NOT depend on the database propagating deletions.

#### Scenario: Data deletion removes escrow

- **WHEN** an account with blobs in both tiers deletes its data
- **THEN** no escrow blob or fetch record for that account remains, and the account is still signed in

#### Scenario: Account deletion removes escrow

- **WHEN** an account with blobs in both tiers is deleted
- **THEN** no escrow blob or fetch record for that account remains

#### Scenario: Removal holds without foreign-key enforcement

- **WHEN** the database does not enforce foreign keys
- **THEN** data deletion and account deletion still remove every escrow row for the account
