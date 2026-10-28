## Purpose

Governs what a Waypoints account is and what its holder can do with it: the account's independence from the identity keypair, the versioned account API that every account UI shares, the sign-in security operations that API adopts, binding an identity to an account by signed challenge, and the two deletion scopes — with the public page that tells anyone, signed in or not, how deletion works.

## ADDED Requirements

### Requirement: An account is independent of any identity

An account SHALL be optional for every capability of the app: the app SHALL function fully with no account and with no binding between an account and an identity. The website SHALL NOT hold an identity's recovery phrase, private key, or any value from which either can be derived. An account SHALL record an identity only as its public key, and only after the holder of that identity has proven control of it by signing a binding challenge.

#### Scenario: The website never receives identity secrets

- **WHEN** any account API request or response is inspected, including every request in an identity binding
- **THEN** none carries a recovery phrase, an identity private key, or a seed

#### Scenario: Deleting an account leaves the identity usable

- **WHEN** an account bound to an identity is deleted
- **THEN** the identity continues to sign, sync and pair exactly as before, because nothing about the keypair depended on the account

### Requirement: Account operations are served by one versioned, session-authenticated API

Every account operation Waypoints defines SHALL be served under `/api/v1/account`, SHALL exchange JSON, and SHALL require a valid session for the account it acts on, except where a requirement below names an exception. A request without a valid session SHALL be refused with status 401. A refusal SHALL carry a body naming a stable machine-readable error code, so that any client can act on it without parsing prose. The website's account pages SHALL perform account operations through this API and no page-private route, so that a second client calling the same endpoints obtains the same behavior. The contract comprises at least:

| Method   | Path                                                  | Operation                                                                                                                                             |
| -------- | ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GET`    | `/api/v1/account`                                     | The account summary: id, name, email, creation time, sign-in methods, whether a second factor is enabled, and counts of passkeys and bound identities |
| `PATCH`  | `/api/v1/account`                                     | Update the display name                                                                                                                               |
| `GET`    | `/api/v1/account/identities`                          | Bound and pending identities                                                                                                                          |
| `POST`   | `/api/v1/account/identities/challenges`               | Issue a binding challenge                                                                                                                             |
| `POST`   | `/api/v1/account/identities/challenges/{id}/response` | Submit a signed challenge                                                                                                                             |
| `POST`   | `/api/v1/account/identities/{member}/confirm`         | Confirm a pending binding                                                                                                                             |
| `DELETE` | `/api/v1/account/identities/{member}`                 | Unlink an identity                                                                                                                                    |
| `POST`   | `/api/v1/account/data/delete`                         | Delete data, keep the account                                                                                                                         |
| `POST`   | `/api/v1/account/delete`                              | Delete the account                                                                                                                                    |

#### Scenario: An anonymous request is refused

- **WHEN** any endpoint above other than the challenge response is called without a session
- **THEN** it responds 401 with an error code, and performs nothing

#### Scenario: A session acts only on its own account

- **WHEN** a request carries a valid session
- **THEN** the operation applies to that session's account, and no request parameter can direct it at another account

#### Scenario: Refusals are machine-readable

- **WHEN** any account endpoint refuses a request
- **THEN** the response body names a stable error code distinct for each refusal reason the endpoint has

### Requirement: Sign-in security operations are adopted, not re-implemented

Password change, passkey registration and removal, second-factor enrolment and removal, session listing and revocation, and linking or unlinking a sign-in provider SHALL be served by the sign-in service's own endpoints under `/api/auth`, and the account contract SHALL name exactly which of them it includes: `/change-password`; `/passkey/generate-register-options`, `/passkey/verify-registration`, `/passkey/list-user-passkeys`, `/passkey/update-passkey`, `/passkey/delete-passkey`; `/two-factor/get-totp-uri`, `/two-factor/enable`, `/two-factor/verify-totp`, `/two-factor/disable`, `/two-factor/generate-backup-codes`, `/two-factor/verify-backup-code`; `/list-sessions`, `/revoke-session`, `/revoke-other-sessions`; `/list-accounts`, `/link-social`, `/unlink-account`. An upgrade of the sign-in service that moves or removes a named endpoint SHALL be detected as a contract break before it ships. An account that has no password SHALL still be able to enrol and remove a second factor.

#### Scenario: A named endpoint disappears

- **WHEN** the sign-in service is upgraded and one of the named endpoints no longer exists at its path
- **THEN** the website's test suite fails

#### Scenario: A provider-only account enrols a second factor

- **WHEN** an account that signs in only through a social provider enrols a second factor
- **THEN** enrolment succeeds without a password being requested

#### Scenario: A second factor is completed at sign-in

- **WHEN** a sign-in for an account with a second factor enabled succeeds on its first factor
- **THEN** the user is taken to a page that accepts the second factor or a backup code, and the session is established only once it is verified

### Requirement: Destructive and binding-sensitive actions require a recent sign-in

Deleting data, deleting the account, unlinking an identity, and removing a second factor SHALL require that the requesting session was created within the last ten minutes. A request on an older session SHALL be refused with an error code that tells the client to have the user sign in again, and SHALL change nothing. Signing in again SHALL return the user to the page that asked for it. Account deletion SHALL additionally require the request to carry the account's email address exactly as recorded, and SHALL be refused when it does not match.

#### Scenario: A stale session cannot delete

- **WHEN** a session created more than ten minutes ago requests account deletion
- **THEN** the request is refused as requiring a fresh sign-in, and the account is unchanged

#### Scenario: Re-authentication returns to the action

- **WHEN** a user is asked to sign in again before a destructive action and does so
- **THEN** they return to the page they started from, with the action not yet performed

#### Scenario: A mistyped confirmation does nothing

- **WHEN** an account deletion request carries an email other than the account's
- **THEN** the request is refused and nothing is deleted

#### Scenario: Deletion has one entry point

- **WHEN** the sign-in service's own user-deletion endpoint is called directly
- **THEN** it is refused, so every account deletion passes through the account API's gate

### Requirement: An identity is bound by signing a challenge the account issued

A binding challenge SHALL be issued only to a session, SHALL be bound to that session's account, SHALL carry 32 random bytes as its nonce, SHALL expire five minutes after issue, and SHALL be consumed by its first response whether or not that response verifies. A response SHALL carry the identity's Ed25519 public key and a signature over the binding message. The website SHALL verify the signature under that key and SHALL refuse a response to an expired, consumed, or unknown challenge, a malformed key, or a signature that does not verify. A verified response submitted on a session of the issuing account SHALL bind the identity at once. A verified response submitted without such a session SHALL leave the identity _pending_: it is listed to the account holder with its fingerprint — the first sixteen hexadecimal digits of its `MemberID`, in groups of four — and becomes bound only when the account holder confirms it within ten minutes of the response; an unconfirmed pending identity SHALL lapse.

#### Scenario: A signed challenge from a signed-in client binds at once

- **WHEN** a client holding a session for the issuing account submits a valid signature over an unexpired challenge
- **THEN** the identity is bound to the account and listed as bound

#### Scenario: A response without a session waits for confirmation

- **WHEN** a valid signature over an unexpired challenge is submitted without a session
- **THEN** the identity is listed as pending with its fingerprint, and is not bound until the account holder confirms it

#### Scenario: A challenge answers once

- **WHEN** a second response is submitted for a challenge that already received one
- **THEN** it is refused, whether or not the first verified

#### Scenario: An expired challenge is refused

- **WHEN** a response arrives more than five minutes after its challenge was issued
- **THEN** it is refused and nothing is bound

#### Scenario: A bad signature binds nothing

- **WHEN** a response's signature does not verify under the key it presents
- **THEN** it is refused and the challenge is consumed

#### Scenario: An unconfirmed pending identity lapses

- **WHEN** a pending identity is not confirmed within ten minutes of its response
- **THEN** it is no longer listed and cannot be confirmed

### Requirement: The binding message is domain-separated and self-describing

The bytes an identity signs to answer a challenge SHALL be, in order: the ASCII tag `waypoints/account-binding/v1`; a zero byte; the website's origin in UTF-8; a zero byte; the account's id in UTF-8; a zero byte; the account's email in UTF-8; a zero byte; the challenge's 32-byte nonce; and the challenge's expiry as milliseconds since the Unix epoch, an unsigned 64-bit big-endian integer. A challenge SHALL expose each of these fields separately so that a signer constructs the message itself; no field SHALL contain a zero byte. The tag SHALL differ from every domain tag the sync protocol signs under, so that no binding signature is valid as any protocol signature and no protocol signature is valid as a binding.

#### Scenario: A signature over another origin is refused

- **WHEN** a response's signature covers a message naming a different origin than the website verifying it
- **THEN** it does not verify and nothing is bound

#### Scenario: A signature for another account is refused

- **WHEN** a signature over one account's challenge fields is submitted against a challenge another account issued
- **THEN** it does not verify and nothing is bound

#### Scenario: A protocol signature is not a binding

- **WHEN** a signature the identity produced under any sync-protocol domain tag is submitted as a response
- **THEN** it does not verify

### Requirement: An identity belongs to at most one account

An identity SHALL be bound to at most one account at a time. Binding an identity already bound to a different account SHALL be refused with a distinct error code, and SHALL NOT disclose which account holds it. An account SHALL be permitted to bind more than one identity, up to a fixed limit, beyond which a further binding is refused. Unlinking SHALL remove the binding and nothing else.

#### Scenario: An identity bound elsewhere is refused

- **WHEN** an identity bound to one account answers a challenge issued by another
- **THEN** the binding is refused with the bound-elsewhere code, and the response does not identify the other account

#### Scenario: An account binds a second identity

- **WHEN** an account with one bound identity binds another, below the limit
- **THEN** both are listed as bound

#### Scenario: An unlinked identity can bind again

- **WHEN** an identity is unlinked from an account
- **THEN** it can subsequently be bound to any account, including the same one

### Requirement: Data deletion removes everything but the sign-in

Deleting data SHALL remove, for the requesting account, every bound and pending identity, every outstanding challenge, and every stored blob scoped to the account, and SHALL leave the account, its sign-in methods, passkeys, second factor, and sessions in place. Once it completes, the account SHALL hold no data other than what signing in requires.

#### Scenario: Data deletion keeps the user signed in

- **WHEN** an account with bound identities and stored blobs deletes its data
- **THEN** its identities, challenges and blobs are gone, and the same session remains valid

### Requirement: Account deletion removes everything the website holds for the account

Deleting an account SHALL remove everything data deletion removes, and additionally the account's passkeys, second-factor secrets and backup codes, verification records that name the account, linked sign-in providers, sessions, and the account itself, and SHALL end every session the account had. Nothing scoped to the account SHALL be retained after deletion completes, and no retention period SHALL apply.

#### Scenario: A deleted account is gone

- **WHEN** an account is deleted
- **THEN** no row in any store refers to it, every session it had is invalid, and its email can be used to create a new, unrelated account

#### Scenario: A deleted account cannot sign in

- **WHEN** a sign-in is attempted with a deleted account's credentials, passkey, or linked provider
- **THEN** it does not reach the deleted account

### Requirement: No account-scoped store escapes the deletion cascade

Every store that holds a value referring to an account SHALL be enrolled in the deletion cascade with a declared scope — removed by data deletion, or removed only by account deletion — and SHALL carry a human-readable description of what it holds. A store referring to an account that is not enrolled SHALL fail the website's test suite. The cascade SHALL remove enrolled stores explicitly rather than relying on the database to propagate deletions. A deletion SHALL be safe to repeat, and SHALL remove an account's data before the account itself, so that a deletion that stops partway leaves an account with less data, never data without an account.

#### Scenario: A new table is caught

- **WHEN** a table with a column referring to the account is added without enrolling in the cascade
- **THEN** the website's test suite fails, naming the table

#### Scenario: An interrupted deletion is retried

- **WHEN** an account deletion fails after removing some data and is requested again
- **THEN** it completes, removing the remainder and the account

#### Scenario: Deletion does not depend on foreign-key enforcement

- **WHEN** the database does not enforce foreign keys
- **THEN** account deletion still removes every enrolled store's rows for the account

### Requirement: Every deletion states that device data is untouched

Before either deletion is confirmed and after it completes, the website SHALL tell the user that Waypoints data on their devices is neither deleted nor reachable by the website, and that removing it means removing it on each device. Data deletion SHALL also state that the account remains and they stay signed in; account deletion SHALL state that it cannot be undone.

#### Scenario: The warning precedes confirmation

- **WHEN** a user opens either deletion action
- **THEN** the statement about device data is shown before the control that confirms it

### Requirement: The account-deletion page is public and self-sufficient

`/account/delete` SHALL be reachable without a session and without the app, SHALL name Waypoints as it appears on its store listing, and SHALL state prominently: the steps to delete an account; what account deletion removes and what data deletion removes, each drawn from the deletion cascade's own descriptions; that nothing is retained after deletion; that data on the user's devices is untouched; and how to request deletion when signing in is not possible. It SHALL link to sign-in, returning a user who signs in to the account's deletion section. Its content SHALL NOT depend on the visitor's session, so the same page is served to everyone.

#### Scenario: The page needs nothing

- **WHEN** `/account/delete` is requested with no session, from a browser that has never visited the site
- **THEN** the page renders in full, including the steps, the deleted and untouched data, and the route for someone who cannot sign in

#### Scenario: The page tracks the cascade

- **WHEN** a store is enrolled in the deletion cascade
- **THEN** its description appears on the page under the deletion scope it declares

#### Scenario: Signing in from the page leads to deletion

- **WHEN** a visitor follows the page's sign-in link and signs in
- **THEN** they arrive at the account's deletion section

### Requirement: The app exposes the account-deletion page from its settings

Every app build distributed through a store whose policy requires an in-app deletion path SHALL include an entry in its settings that opens `/account/delete` on the website in the system browser, reachable without an account and without being signed in to one. The entry SHALL open the page rather than embedding it, so that deletion takes place on the website.

#### Scenario: The entry opens the public page

- **WHEN** a user of the Android app chooses the account-deletion entry in settings
- **THEN** the system browser opens the website's `/account/delete` page
