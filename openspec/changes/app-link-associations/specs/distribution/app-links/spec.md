## Purpose

Governs which web links open which installed Waypoints app: the host each shipped channel answers links on, the single table of link paths and the association files derived from it, which builds may be named, how those files are served so that the platforms can verify them, the link set every build declares, the host a build writes into the links it emits, and how a link's landing page behaves when no app takes it.

## ADDED Requirements

### Requirement: Each shipped channel answers links on its own host

Each shipped channel SHALL have exactly one link host, and no two channels SHALL share one: stable on `waypoints.so`, preview on `preview.waypoints.so`, nightly on `nightly.waypoints.so`. The association files served on a link host SHALL name only the app of that host's channel. A host that is not a link host SHALL serve association files naming no app.

#### Scenario: A link opens the channel that owns its host

- **WHEN** a link on `preview.waypoints.so` is opened on a device with stable, preview and nightly installed
- **THEN** the preview app opens it, and neither stable nor nightly is offered

#### Scenario: A channel host names only its own app

- **WHEN** the association files on `nightly.waypoints.so` are read
- **THEN** they name the nightly app and no other

#### Scenario: A missing channel app leaves the link to the browser

- **WHEN** a link on a channel's host is opened on a device where only a different channel is installed
- **THEN** no app opens it, and the host's landing page is shown

#### Scenario: An unknown host associates nothing

- **WHEN** the association files are requested on a host that is not a link host
- **THEN** they are well formed and name no app

### Requirement: One table declares every link path

Every path that is or may become an app link SHALL be declared in one table, and each entry SHALL be either associated or reserved. Both platforms' association files on every link host SHALL be derived from that table, so that the set of associated paths is identical across platforms and across link hosts. A path SHALL match exactly: a path that merely begins with an associated path SHALL NOT be associated.

#### Scenario: An associated path reaches both platforms on every host

- **WHEN** a path is associated in the table
- **THEN** the Apple association on every link host includes it for that host's app, and the same path is the one every build declares to Android

#### Scenario: A longer path is not captured

- **WHEN** a link names a path that begins with an associated path but is not equal to it
- **THEN** no app is associated with that link

#### Scenario: The pairing path is associated

- **WHEN** the table is read
- **THEN** `/pair` is associated

### Requirement: A reserved path is held but not associated

A reserved path SHALL appear in no association file and SHALL NOT be declared by any build. The site SHALL answer a request for a reserved path as not found, rather than serving any page there, so that associating the path later takes over no page a user has seen. A reserved path SHALL NOT collide with a path the site already routes.

#### Scenario: A reserved path opens no app

- **WHEN** a link to a reserved path is opened on a device with every channel installed
- **THEN** no app opens it

#### Scenario: A reserved path serves no page

- **WHEN** a reserved path is requested on any link host
- **THEN** the response is not found, and no content, sign-in, or catch-all page is rendered there

### Requirement: Only shipped release builds are ever named

Association files SHALL name only the Apple app identifiers and Android packages of the shipped channels: `so.waypoints`, `so.waypoints.preview` and `so.waypoints.nightly`. The development flavor's identifier and any debug-build package SHALL NOT be named on any host. No fingerprint of a debug signing key SHALL be published on any host.

#### Scenario: Development is not associated

- **WHEN** the association files on any link host are read
- **THEN** `so.waypoints.dev` appears in neither

#### Scenario: Debug packages are not associated

- **WHEN** the Android association on any link host is read
- **THEN** no package ending in `.debug` appears, and no debug-keystore fingerprint is listed

### Requirement: The Apple association names its channel's app identifier and the associated paths

The Apple association on a link host SHALL name its channel's app identifier as the developer team identifier joined to that channel's bundle identifier. It SHALL list each associated path from the table as an exact path component, and no reserved path. It SHALL conform to Apple's documented `applinks` format.

#### Scenario: The app identifier is team-qualified

- **WHEN** the Apple association on `preview.waypoints.so` is read
- **THEN** its only app identifier is the team identifier followed by `.so.waypoints.preview`

#### Scenario: The fragment does not affect matching

- **WHEN** an associated link carries a fragment
- **THEN** the link is associated regardless of the fragment's content

### Requirement: The Android association lists its channel's signing certificates

The Android association on a link host SHALL hold at most one statement, delegating link handling to that channel's package, with the SHA-256 fingerprints of every certificate an installed release build of that package may be signed with. Fingerprints SHALL be configured per channel at deploy time, so that pinning or rotating a certificate needs no commit. A channel with no fingerprints configured SHALL publish no statement rather than an empty one. A configured value that is not a well-formed SHA-256 fingerprint SHALL NOT be published. The association SHALL conform to Google's documented Digital Asset Links statement format.

#### Scenario: A channel's statement names only its package

- **WHEN** the Android association on `nightly.waypoints.so` is read with fingerprints configured for nightly
- **THEN** it holds one statement, for `so.waypoints.nightly`, listing exactly those fingerprints

#### Scenario: One channel's fingerprints never reach another host

- **WHEN** fingerprints are configured for stable only
- **THEN** the stable host lists them, and the preview and nightly hosts hold no statement

#### Scenario: A malformed fingerprint is not published

- **WHEN** a configured fingerprint value is not a well-formed SHA-256 fingerprint
- **THEN** that value is absent from every served statement

#### Scenario: Rotation is a deploy-time change

- **WHEN** a channel's fingerprint configuration changes and the site is redeployed
- **THEN** the served statement reflects the new set with no change to the repository

### Requirement: Association files are served directly to anyone

On every link host, each association file SHALL be served at its platform's well-known path with a success status, an `application/json` content type, and its body, to an unauthenticated request. It SHALL NOT be served through a redirect, a login, or any other access gate, whatever gating applies to the rest of that host.

#### Scenario: An unauthenticated fetch succeeds

- **WHEN** `/.well-known/apple-app-site-association` or `/.well-known/assetlinks.json` is requested on any link host with no credentials
- **THEN** the response is a success with an `application/json` body and no redirect

#### Scenario: Apple's association CDN holds the file

- **WHEN** Apple's association CDN is queried for a link host
- **THEN** it returns that host's Apple association rather than reporting it missing

#### Scenario: Google's verifier reads the statement

- **WHEN** the Digital Asset Links API lists statements for a link host whose channel has fingerprints configured
- **THEN** it returns that channel's statement with no error code

### Requirement: Every build declares the same link set

Every mobile build, of every flavor, SHALL declare the same set of web links: every link host, each with every associated path from the table, over `https` only. The association files SHALL be the only per-channel part of link handling. The declared set SHALL equal the table's associated paths across the link hosts, with nothing added and nothing missing.

#### Scenario: A preview build declares every host

- **WHEN** the link declaration of a preview build is inspected
- **THEN** it names `waypoints.so`, `preview.waypoints.so` and `nightly.waypoints.so`, each with every associated path

#### Scenario: An undeclared host's association does not reach the app

- **WHEN** a stable build is installed and a link on `preview.waypoints.so` is opened
- **THEN** the stable app is not verified for that host and does not open the link

#### Scenario: The declaration and the table agree

- **WHEN** the app's declared link set and the table are compared
- **THEN** every declared path is associated in the table and every associated path is declared, for every link host

#### Scenario: Plain http is not claimed

- **WHEN** an `http://` link to an associated path is opened
- **THEN** no app is declared for it

### Requirement: A build writes links on its own channel's host

A link a shipped build emits SHALL use its own channel's link host. A development build SHALL emit the `waypoints://` form, which no association opens. Reading a link back SHALL accept the associated path on any link host and the `waypoints://` form, and SHALL NOT accept the same path on any other host.

#### Scenario: A nightly build emits a nightly link

- **WHEN** a nightly build produces a pairing link
- **THEN** the link is on `nightly.waypoints.so`

#### Scenario: A development build emits the scheme form

- **WHEN** a development build produces a pairing link
- **THEN** the link uses the `waypoints://` form and names no link host

#### Scenario: A link from another channel is still recognized

- **WHEN** a stable build is handed a pairing link on `preview.waypoints.so`
- **THEN** it recognizes the link as a pairing link and routes it to setup, where the payload's own validation decides whether it is usable

#### Scenario: A foreign host is not recognized

- **WHEN** a build is handed the pairing path on a host that is not a link host
- **THEN** it does not recognize the link as a pairing link

### Requirement: A channel host serves only links, association files, and their landing pages

On a channel's link host, a request for an associated path SHALL render that path's landing page, which SHALL offer that channel's download and SHALL copy a link on the same host. Association files SHALL be served there as already required, and reserved paths SHALL be answered as not found there as on every link host. Any other request on a non-stable link host SHALL be redirected temporarily to the same path on `waypoints.so`. The redirect SHALL NOT apply to an associated path, a reserved path, or an association file.

#### Scenario: The preview landing page offers preview

- **WHEN** `/pair` is opened in a browser on `preview.waypoints.so`
- **THEN** the page offers the preview download and copies a link on `preview.waypoints.so`

#### Scenario: Other paths go to the apex

- **WHEN** `/download/stable` is requested on `nightly.waypoints.so`
- **THEN** the response redirects to `/download/stable` on `waypoints.so`

#### Scenario: The landing page is never redirected away

- **WHEN** an associated path is requested on a channel host
- **THEN** the response is that path's landing page on the same host, not a redirect
