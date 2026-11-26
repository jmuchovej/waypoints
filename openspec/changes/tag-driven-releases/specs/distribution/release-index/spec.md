## MODIFIED Requirements

### Requirement: The index is written after its tag exists

A channel's index entry SHALL be written only after the tag it describes has been created, and only by the run that published it, after that run's manifest. An index entry SHALL NOT be written in the same commit that the tag points at, and SHALL NOT be a precondition of tagging. A run that withdrew its tag SHALL write no entry.

This ordering is forced, not chosen: the entry records the tag's rev and source hash, so an entry written before tagging would have to contain a hash of content that includes itself; and an entry written before the publication succeeded would record a release that may never exist.

The writer SHALL refuse a tag whose version classifies as one channel while the tagged tree's manifest version classifies as another.

#### Scenario: Tag precedes its index entry

- **WHEN** a release is published on any channel
- **THEN** the tag is created first, and the index entry naming that tag is committed afterwards, to `main`

#### Scenario: A failed index write leaves a valid release

- **WHEN** a tag is created successfully but the subsequent index write fails
- **THEN** the release itself remains valid and published, and the channel's package continues to resolve its previous entry until the index is written

#### Scenario: A wrongly minted tag is not recorded

- **WHEN** a tag whose version reads as a preview points at a tree whose manifest version reads as nightly
- **THEN** the writer fails without touching any index file, naming both channels

#### Scenario: A withdrawn release is not recorded

- **WHEN** a release run falls short and deletes its tag
- **THEN** no index entry names that tag, and the channel's package continues to resolve its previous entry
