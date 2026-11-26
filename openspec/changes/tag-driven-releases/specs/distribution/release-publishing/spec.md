## ADDED Requirements

### Requirement: One pipeline publishes every channel, started by its tag

The release pipeline SHALL be one workflow, started by the push of a release tag, by a dispatch naming an existing tag, or by a call. It SHALL derive the channel, the version, the flavor, and the store build numbers from the tag it was started by, SHALL refuse a tag whose tree's manifest names another channel, and SHALL mint no tag of its own. A library group's tag SHALL start no release.

#### Scenario: A pushed tag starts its release

- **WHEN** a release tag of any channel is pushed
- **THEN** one release run starts, reads the channel and version from the tag, and builds and publishes under that channel's flavor

#### Scenario: A tag disagreeing with its tree is refused

- **WHEN** a tag whose version reads as one channel points at a tree whose manifest reads as another
- **THEN** the run fails before any build, naming both channels

#### Scenario: A library tag starts nothing

- **WHEN** a library group's tag is pushed
- **THEN** no release run starts

#### Scenario: A rehearsal without a tag describes the day's nightly

- **WHEN** the pipeline is dispatched as a dry run from a nightly tree with no tag
- **THEN** it builds the manifest's version with the run's UTC date appended, and publishes nothing

### Requirement: The builder knows only the tree it is given

Testing and building a release SHALL be a called workflow that takes a ref, checks it out, runs the release test gates, builds every selected platform stamping the version and build numbers it derives from the ref, renames the outputs as they will be stored, and hands them to its caller as run artifacts. It SHALL hold no store, forge, or App credential, SHALL contact no store, and SHALL report what it built as outputs its caller counts.

#### Scenario: The builder stamps from the ref

- **WHEN** the builder is given a tag
- **THEN** the version and store build numbers it stamps come from that tag, and no caller supplies them

#### Scenario: The builder publishes nothing

- **WHEN** the builder completes every build
- **THEN** no object exists in the store, no release record exists, and no store has been contacted

#### Scenario: The caller counts the builder's targets

- **WHEN** the publishing job decides whether the run is complete
- **THEN** it counts the targets the builder reports against the channel's required set

### Requirement: The publishing run records the release it committed

The release-index entry for a published tag SHALL be written by the run that published it, after the channel's manifest, as a pull request to `main` that merges itself, and SHALL be written on a real run only. A run that withdrew SHALL write no entry.

#### Scenario: The entry follows the manifest

- **WHEN** a real run commits its manifest
- **THEN** the same run then writes the channel's index entry and opens its pull request

#### Scenario: A withdrawn tag is not recorded

- **WHEN** a run falls short and withdraws
- **THEN** no index entry is written for its tag

#### Scenario: A rehearsal renders, not records

- **WHEN** a dry run reaches its publishing steps
- **THEN** the index entry is rendered into the run's artifacts and no pull request opens

### Requirement: A shortfall withdraws what the run created, its tag included

A run that fails before submitting to an app store, or that falls short of its channel's required targets, SHALL withdraw: it SHALL discard the release record it drafted, delete any objects it uploaded to the store, and delete the release's tag when the tag's own push started the run. A tag named by a dispatched re-publication SHALL be left in place, since its release may already be out. Deleting uploaded objects SHALL NOT be load-bearing: no manifest names them.

#### Scenario: A pushed tag is withdrawn

- **WHEN** a run started by a tag push fails or falls short before submitting to an app store
- **THEN** the drafted record is discarded, the uploaded objects are deleted, and the tag is deleted

#### Scenario: A re-published tag survives

- **WHEN** a run dispatched against an existing tag fails or falls short
- **THEN** the drafted record is discarded and the tag is left in place

#### Scenario: Uploaded objects are removed, and their removal is not load-bearing

- **WHEN** a run that uploaded objects to the store fails before submitting to an app store
- **THEN** the run deletes those objects, and if a deletion fails the objects remain unreferenced and unadvertised rather than the withdrawal being treated as incomplete

### Requirement: A withdrawal undoes the bump that minted the tag

When a withdrawal deletes the tag, it SHALL also undo the bump that minted it, so no branch declares a version no tag names: it SHALL revert the version write's merge commit through a pull request the bot opens and merges unattended, and for the first preview of a series it SHALL also delete the series branch and the library tags the cut minted and revert the cut's write on `main`. A run that leaves the tag SHALL leave the write.

#### Scenario: The write is reverted

- **WHEN** a run started by a tag push fails or falls short before submitting to an app store
- **THEN** the tag is deleted and a pull request reverting the write's merge commit is open on the branch it landed on, merging itself once its checks pass

#### Scenario: A failed first preview undoes the cut

- **WHEN** the first preview of a new series fails or falls short before submitting to an app store
- **THEN** the app tag, the library tags the cut minted, and the series branch are deleted, and a pull request reverting the cut's write is open on `main`

#### Scenario: A re-published tag's write stays

- **WHEN** a run dispatched against an existing tag fails or falls short
- **THEN** the write that tag was minted from is untouched

### Requirement: A failure past the point of no return is not withdrawn

A run that fails at or after submitting to an app store SHALL NOT withdraw. The stores already hold the build, so the release record, the store objects, and the tag SHALL be left in place, and the run SHALL fail visibly, leaving any remaining step to be re-run.

#### Scenario: A failure past the point of no return is not withdrawn

- **WHEN** a run fails at or after submitting to an app store
- **THEN** the release record, the uploaded objects, and the tag are all left in place, and the run reports failure without withdrawing

## MODIFIED Requirements

### Requirement: Each channel elects what it publishes

A channel SHALL publish only what its election admits. The election is the bump's: a nightly SHALL be minted only when product work landed since the previous nightly, and a run finding none SHALL mint no tag, so no release run starts. The publishing run SHALL publish the tag that started it, on every channel, or the existing tag a real re-run names. A dry run SHALL elect nothing to publish: it MAY build for verification, and SHALL upload nothing to the store, publish no record, and write no manifest. A real run that names no tag SHALL be refused.

#### Scenario: A quiet repository publishes no nightly

- **WHEN** the nightly bump finds no product work landed since the previous nightly
- **THEN** no tag is minted, no release run starts, and no manifest is written

#### Scenario: A tagged channel publishes its trigger

- **WHEN** a release run is started by a tag push
- **THEN** that tag's version is the one published, and no other version is elected

#### Scenario: A build-only run publishes nothing

- **WHEN** a dry run is asked to build
- **THEN** the build proceeds for verification, and nothing is uploaded, advertised, or recorded

#### Scenario: A real run without a tag is refused

- **WHEN** the pipeline is dispatched as a real run with no tag
- **THEN** the run refuses before building, and nothing is published

### Requirement: A channel advertises a release only once every target has succeeded

Each channel SHALL have a declared set of required targets. A build of a target SHALL NOT publish anything: it SHALL hand its artifacts on, named as they will be stored, and every publishing act SHALL be performed by a single job that runs only after the whole set has built. That job SHALL write the channel's manifest only after every target's artifacts are in the store, and the manifest SHALL be the last write to the store. The release-index entry SHALL follow the manifest and SHALL be the run's only write after it. Completeness SHALL be determined by counting successful target builds against the required set, and SHALL NOT be inferred from the absence of failure — a target that was skipped SHALL block publication exactly as a target that failed.

#### Scenario: A skipped target blocks publication

- **WHEN** a run is invoked for a subset of its channel's required targets, with a tag
- **THEN** the publishing job does not run: nothing is uploaded, submitted, or advertised, and no release record is published, even though no target failed

#### Scenario: A failed target blocks publication

- **WHEN** one target's build fails and the rest succeed
- **THEN** the publishing job does not run: nothing is uploaded, submitted, or advertised, and no release record is published

#### Scenario: A complete run advertises exactly once

- **WHEN** every required target for a channel has built successfully in a tagged run
- **THEN** the publishing job uploads every target's artifacts, publishes the release record, and writes the channel's manifest naming that version and its targets

#### Scenario: The manifest is written last

- **WHEN** a publishing run uploads artifacts and writes a manifest
- **THEN** every artifact the manifest names is already present in the store at the moment the manifest becomes readable

#### Scenario: The record follows the manifest

- **WHEN** a publishing run writes its index entry
- **THEN** the channel's manifest is already readable, and nothing is written to the store afterwards

## REMOVED Requirements

### Requirement: A shortfall withdraws what the run created, and nothing else

**Reason**: The rule that a run deletes only a tag it minted itself, leaving a triggering tag in place, described a pipeline in which preview and stable tags were pushed before the run that built them. Under the tag-driven pipeline every tag that starts a run was minted for that publication minutes before, nothing has recorded it, and the tag ruleset permits the run to delete it, so the triggering tag is what a shortfall withdraws. The requirement continues as _A shortfall withdraws what the run created, its tag included_ and _A failure past the point of no return is not withdrawn_, which carry the two unchanged scenarios and replace _A self-minted tag is withdrawn_ and _A triggering tag survives_ with their reversal.

**Migration**: Regenerate the release workflow; the withdraw job's tag step is conditioned on the run being the tag's own push in place of the planner's `tag_minted` output, which goes.
