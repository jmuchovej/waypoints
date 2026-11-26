## ADDED Requirements

### Requirement: Key escrow is off until the user enables a tier

No key material SHALL leave a device for escrow unless the user has enabled escrow for that tier on that identity. Escrow SHALL have two tiers, _full_ and _read-only_, each enabled and disabled independently. Creating or importing an identity SHALL NOT enable either tier. Before a tier's first upload the app SHALL show, and the user SHALL confirm, what the tier contains, that Waypoints' servers can store, return to the signed-in account and delete the blob but cannot open it, and what anyone holding both the blob and the passphrase can do with it: restore the identity in full, for the full tier; read everything the identity can read, now and later, and author nothing, for the read-only tier.

#### Scenario: A new identity uploads nothing

- **WHEN** a user creates or imports an identity and does not enable escrow
- **THEN** no escrow blob for that identity exists on any server

#### Scenario: Enabling one tier leaves the other off

- **WHEN** a user enables the read-only tier
- **THEN** a read-only blob is uploaded and no full blob is created

#### Scenario: Consent precedes the first upload

- **WHEN** a user starts enabling a tier
- **THEN** nothing is uploaded until the user has confirmed that tier's statement of what the server and a passphrase holder can do

### Requirement: The full tier restores the identity; the read-only tier cannot author

A full-tier blob SHALL contain the identity's recovery-phrase entropy and nothing else, so that opening it reconstructs the identical recovery phrase and therefore the same identity signing key, the same KEM key, and every device key. A read-only blob SHALL contain the identity's KEM seed and nothing else, so that opening it yields the KEM key that read keys are sealed to and yields no key able to sign a Notice, an ACL record or a device certificate.

#### Scenario: A full blob reconstructs the recovery phrase

- **WHEN** a full-tier blob made from a 12-word or a 24-word recovery phrase is opened with its passphrase
- **THEN** the result is that exact recovery phrase, and it derives the same `MemberID` as the identity that made the blob

#### Scenario: A read-only blob yields the identity's KEM key

- **WHEN** a read-only blob is opened with its passphrase
- **THEN** the key it yields has the same public key as the identity's KEM key, and opens a read key sealed to that identity

#### Scenario: A read-only blob cannot author

- **WHEN** a read-only blob has been opened
- **THEN** nothing it yields can produce a signature that verifies under the identity's signing key

### Requirement: Escrow key material is wrapped and unwrapped only on the device

Wrapping and unwrapping SHALL happen in the device's core, never on a server and never in the app's frontend. An escrow operation SHALL send to a server only the ciphertext, nonce, salt, key-derivation parameters, format version, tier and identity public key. The app's frontend SHALL receive no recovery phrase, seed, KEM seed or derived wrapping key as the result of any escrow operation; it SHALL receive only per-tier status. The passphrase SHALL pass from the user's input to the core for the single operation that needs it, SHALL NOT be persisted, and SHALL NOT be sent to any server in any form, including hashed.

#### Scenario: An upload carries only the blob and its metadata

- **WHEN** a tier is enabled
- **THEN** the request carries the ciphertext, nonce, salt, derivation parameters, format version, tier and identity public key, and no other field derived from the passphrase or the key material

#### Scenario: The frontend sees status, not keys

- **WHEN** the frontend enables, replaces, disables or reads the status of a tier
- **THEN** every response it receives is per-tier status and contains no key material

#### Scenario: The passphrase does not outlive the operation

- **WHEN** an escrow operation that took a passphrase completes or fails
- **THEN** the passphrase is not present in any file, setting or store the device keeps

### Requirement: Escrow keys come from a memory-hard derivation with recorded parameters

The wrapping key SHALL be derived from the passphrase with Argon2id, under a random salt unique to each blob, with the derivation's memory, iteration and parallelism parameters recorded beside the blob. The passphrase SHALL be Unicode-normalized before derivation, so the same passphrase typed on any platform derives the same key. A device SHALL refuse to wrap under parameters below the published floor, and SHALL refuse to open a blob whose recorded parameters are below the floor or above the published ceiling, before running the derivation.

#### Scenario: Two tiers under one passphrase do not share a key

- **WHEN** both tiers are wrapped under the same passphrase
- **THEN** each blob carries its own salt, and the key that opens one does not open the other

#### Scenario: Weakened parameters are refused before deriving

- **WHEN** a fetched blob records parameters below the floor
- **THEN** the device refuses it without running the derivation and reports the blob as unusable

#### Scenario: Excessive parameters are refused before deriving

- **WHEN** a fetched blob records a memory cost above the ceiling
- **THEN** the device refuses it without allocating that memory

#### Scenario: Composed and decomposed input derive the same key

- **WHEN** a passphrase containing an accented letter is entered once in composed and once in decomposed Unicode form
- **THEN** both derive the same wrapping key

### Requirement: A blob is bound to its tier, its identity and its parameters

A blob SHALL open only under the tier, identity public key, format version, salt and derivation parameters it was made with: the authentication of the ciphertext SHALL cover all of them, so a change to any one makes the blob fail to open. After opening a full-tier blob, the device SHALL check that the restored identity's public key equals the identity public key the blob was bound to, and SHALL treat a mismatch as a failure to open.

#### Scenario: A relabelled tier does not open

- **WHEN** a read-only blob is presented as a full blob
- **THEN** it fails to open

#### Scenario: A blob presented under another identity does not open

- **WHEN** a blob is presented with an identity public key other than the one it was made for
- **THEN** it fails to open, and nothing is written to the device

#### Scenario: A substituted blob does not open under the user's passphrase

- **WHEN** the server returns a blob that was wrapped under a passphrase other than the user's
- **THEN** the user's passphrase fails to open it and the device restores nothing

### Requirement: The escrow passphrase clears a strength floor and is not the account password

The app SHALL offer a generated passphrase as the default. A generated passphrase SHALL carry at least 90 bits of entropy and SHALL NOT be classifiable as a recovery phrase. A user-chosen passphrase SHALL be accepted only when the device's estimate of the guesses needed to find it is at least 10^14, with the account's email and display name counted as known to the guesser; a passphrase that parses as a recovery phrase SHALL be refused. The app SHALL NOT prefill, suggest or reuse the account password as the passphrase, and where the device's core holds the account password during the same session it SHALL refuse an equal passphrase. Every refusal SHALL say why.

#### Scenario: The default passphrase is generated

- **WHEN** a user enables a tier and does not choose a passphrase
- **THEN** a generated passphrase of at least 90 bits is used and shown to the user to record

#### Scenario: A generated passphrase is never mistaken for a recovery phrase

- **WHEN** a generated passphrase is pasted into the setup-code field
- **THEN** it is not routed as a recovery phrase

#### Scenario: A weak passphrase is refused

- **WHEN** a user chooses a passphrase whose estimated guess count is below 10^14
- **THEN** enabling is refused with an explanation, and nothing is uploaded

#### Scenario: The recovery phrase is not a passphrase

- **WHEN** a user enters a valid recovery phrase as the passphrase
- **THEN** it is refused

### Requirement: Restore from escrow is an onboarding path

A device with no identity SHALL offer to restore from escrow alongside creating an identity and entering a recovery phrase. Restoring SHALL sign the user in, let the user choose among the account's identities that have a full-tier blob when there is more than one, fetch that identity's full-tier blob, take the passphrase, open the blob on the device, check the restored identity against the blob's identity public key, and then adopt the identity exactly as entering that recovery phrase would — including choosing a fresh device index. Nothing SHALL be written to the device's identity storage until the blob has opened and the check has passed. A device that already has an identity SHALL refuse to restore over it. A read-only blob SHALL NOT be offered as a restore source.

#### Scenario: A restore adopts the escrowed identity

- **WHEN** a user restores from escrow on a device with no identity and enters the right passphrase
- **THEN** the device holds the escrowed identity, reports the same `MemberID` it had before, and uses a device index of its own

#### Scenario: A wrong passphrase writes nothing

- **WHEN** a user restores with the wrong passphrase
- **THEN** the blob does not open, no identity is written, and the user may try again

#### Scenario: Several escrowed identities

- **WHEN** the signed-in account has full-tier blobs for two identities
- **THEN** the app asks which identity to restore before asking for a passphrase

#### Scenario: No full blob

- **WHEN** the signed-in account has no full-tier blob
- **THEN** the app says so and offers recovery-phrase entry instead

#### Scenario: An existing identity is not overwritten

- **WHEN** a device that already has an identity is asked to restore from escrow
- **THEN** the restore is refused and the existing identity is untouched

### Requirement: Changing the passphrase re-wraps on the device

Changing the escrow passphrase SHALL re-wrap every enabled tier on the device under the new passphrase, each with a fresh salt and nonce, and replace each stored blob. The identity's key material SHALL NOT change. A change SHALL require a device that holds the identity. When a replacement fails for any tier, the app SHALL report which tiers hold the new passphrase and which still hold the old one.

#### Scenario: The new passphrase opens, the old does not

- **WHEN** a user changes the passphrase and every replacement succeeds
- **THEN** each enabled tier's blob opens under the new passphrase and not under the old one, and the restored identity's `MemberID` is unchanged

#### Scenario: A partial change is reported per tier

- **WHEN** the full tier is replaced and the read-only replacement fails
- **THEN** the app reports that the full tier uses the new passphrase and the read-only tier still uses the old one

### Requirement: Disabling a tier deletes its blob

Disabling a tier SHALL delete that tier's blob from the server, and the app SHALL report the tier as disabled only once the server has confirmed the deletion. The app SHALL state that deletion cannot recall a copy already fetched, nor revoke a KEM key already opened from a read-only blob.

#### Scenario: Disabling removes the blob

- **WHEN** a user disables the full tier and the server confirms deletion
- **THEN** the tier reports disabled and a later restore finds no full blob

#### Scenario: A failed deletion is not reported as disabled

- **WHEN** the deletion request fails
- **THEN** the tier still reports enabled, with the failure shown
