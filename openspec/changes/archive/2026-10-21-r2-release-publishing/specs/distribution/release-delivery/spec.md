## Purpose

Governs how a published release is reached: the download surface that stands in front of a private artifact store and derives an object from a request's parts, the name a downloaded file is saved under, what a channel's manifest guarantees to an updater, how the published set is enumerated without a third-party proxy, and where release notes are read from.

## ADDED Requirements

### Requirement: The artifact store is never reached directly by a client

The artifact store SHALL remain private. No client SHALL be given a URL that resolves to the store, and the store SHALL NOT be made publicly readable. Every download SHALL pass through the site's download surface, which reads the object under access granted to its own deployment.

#### Scenario: No public store URL is published

- **WHEN** a download link is rendered for any platform or channel
- **THEN** it addresses the site's download surface, and no store hostname appears in it

#### Scenario: The store refuses anonymous reads

- **WHEN** the store is addressed without the deployment's access
- **THEN** the object is not served

### Requirement: A download request names its object formulaically, and every part is checked

The download surface SHALL identify an object from the channel, minor line, platform, and architecture its request names, together with the version carried by the request's final segment. It SHALL validate every part and SHALL reject any request whose parts disagree with one another, including a version that does not belong to the named minor line, a version that does not classify into the named channel, a platform or architecture outside the published set, and a final segment that is not the exact name those parts imply. The object key SHALL be rebuilt from validated parts only, so that no request text reaches the key directly.

#### Scenario: A well-formed request resolves

- **WHEN** a request names a published channel, the minor line its version belongs to, a published platform and architecture, and the exact filename those imply
- **THEN** the corresponding object is served

#### Scenario: A version outside the named line is refused

- **WHEN** a request names a minor line that the requested version does not belong to
- **THEN** the request is refused and no object is read

#### Scenario: A version from another channel is refused

- **WHEN** a request names one channel but a version that classifies into another
- **THEN** the request is refused and no object is read

#### Scenario: A mismatched final segment is refused

- **WHEN** a request's final segment is not the name implied by its other parts
- **THEN** the request is refused, and the segment cannot cause any other object to be read

#### Scenario: An unavailable store is distinguished from a missing release

- **WHEN** the download surface cannot reach the store at all
- **THEN** the response reports an unavailable service rather than a missing artifact

### Requirement: A downloaded file is saved under a readable name

The download surface SHALL set the name a client saves a file under, and that name MAY be friendlier than the version it derives from. For a channel whose versions carry a build counter, the saved name MAY omit the counter in favour of a readable date, since the saved name is never parsed back into a version. The saved name SHALL NOT be required to be a valid version string, and SHALL NOT be relied upon to address an object.

#### Scenario: A nightly saves under a readable name

- **WHEN** a nightly artifact is downloaded
- **THEN** the file is saved under a name carrying its date in readable form rather than its build counter

#### Scenario: The readable name addresses nothing

- **WHEN** a readable saved name is used as a download request
- **THEN** the request is refused, because it is not the name the request's other parts imply

#### Scenario: Short versions pass through unchanged

- **WHEN** an artifact's version carries no build counter
- **THEN** the saved name matches the version as published

### Requirement: Each channel offers a versionless download URL that redirects

Each channel SHALL expose a download URL naming no version, in the same shape as a versioned one, with a reserved alias standing where the minor line goes. A request to it SHALL resolve the channel's current version from that channel's manifest and SHALL redirect to the versioned URL for the same target, rather than serving bytes itself. The alias SHALL NOT be a stored object, and SHALL NOT be cached as though immutable. The reserved alias SHALL NOT be ambiguous with any minor line.

#### Scenario: The alias redirects to the current version

- **WHEN** a versionless download URL is requested for a channel and target
- **THEN** the response redirects to the versioned URL that channel's manifest currently names

#### Scenario: The redirect target is cached as immutable, the alias is not

- **WHEN** a versionless URL and the versioned URL it resolves to are each served
- **THEN** the versioned response is cacheable as immutable and the alias response is not

#### Scenario: The saved file carries the resolved version

- **WHEN** a download is taken through the versionless URL
- **THEN** the file is saved under the resolved version's name, not under the alias

#### Scenario: A channel with nothing published has no alias target

- **WHEN** a versionless URL is requested for a channel whose manifest does not exist
- **THEN** the request reports a missing release and no redirect is issued

#### Scenario: The alias cannot be mistaken for a minor line

- **WHEN** the reserved alias is compared against the form a minor line takes
- **THEN** no minor line can ever equal it, so a request is never ambiguous

### Requirement: The download page's links survive a release without rebuilding

The page offering downloads SHALL link only to versionless URLs, so that its markup does not change when a release is published and may be served from cache in full. Any version or date the page displays SHALL be resolved from the channel's manifest at view time rather than being built into the markup. A stale displayed version SHALL NOT cause a stale link.

#### Scenario: A publish does not invalidate the page

- **WHEN** a release is published after the download page was last built
- **THEN** the page's existing links resolve to the new release without the page being rebuilt

#### Scenario: Displayed version is resolved at view time

- **WHEN** the download page shows a channel's current version
- **THEN** that value comes from the channel's manifest rather than from the markup

#### Scenario: Presentation degrades without breaking downloads

- **WHEN** the displayed version cannot be resolved
- **THEN** the download links still resolve to the channel's current release

### Requirement: A channel's manifest is what an updater reads

Each channel SHALL expose a manifest naming that channel's current version and the targets available for it. A manifest SHALL describe only a release whose every required target was published. A manifest SHALL be served as published rather than derived on demand, and SHALL be the single answer to what version a channel is on — so that the download surface and an updater can never disagree about a channel's current release.

#### Scenario: An updater learns the current version

- **WHEN** a client reads a channel's manifest
- **THEN** it names that channel's current version and the targets published for it

#### Scenario: A channel with nothing published offers no manifest

- **WHEN** a channel has never completed a publishing run
- **THEN** no manifest is available for it, and that is reported as a missing release rather than an error

#### Scenario: One answer for both readers

- **WHEN** the download surface and an updater each resolve a channel's current version
- **THEN** both read the same manifest and obtain the same version

#### Scenario: A manifest is not frozen into a build

- **WHEN** the site is deployed
- **THEN** manifests are read at request time, so a release published after that deployment is reflected without redeploying

### Requirement: The published set is enumerated from the store, not a third party

The set of published releases SHALL be derived from the artifact store and the forge's release records, under access the site itself holds. Enumeration SHALL NOT depend on an unauthenticated third-party proxy of the forge, so that a private repository does not reduce the published set to nothing.

#### Scenario: A private repository still enumerates

- **WHEN** the source repository is private
- **THEN** the published releases for every channel are still enumerated and rendered

#### Scenario: No third-party proxy is contacted

- **WHEN** the site resolves what has been published
- **THEN** it contacts the artifact store and the forge directly, and no third-party mirror of the forge is involved

### Requirement: Release notes are read from the forge

Release notes SHALL be read from the forge's release records using credentials the site holds, and SHALL NOT be expected in the artifact store. A release whose notes cannot be read SHALL still be listed with its version and its available downloads.

#### Scenario: Notes come from the record

- **WHEN** a release's notes are rendered
- **THEN** they are read from that release's record in the forge

#### Scenario: Missing notes do not hide a release

- **WHEN** a published release's notes cannot be read
- **THEN** the release is still listed with its version and downloads
