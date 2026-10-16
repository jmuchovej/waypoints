## Purpose

Governs how release builds reuse what earlier builds compiled and fetched: one remote store shared by every release build job, reachable only under a credential CI holds and handed only to the processes that contact the store, configured through the repository's declared-secret resolver, serving compilation per unit and fetched directories as immutable snapshots keyed by the manifests that pin them, and never able to turn a store outage into a failed release.

## Requirements

### Requirement: Release builds share one remote compiler cache

Every generated job that builds a release artifact SHALL route its Rust compilation, and the C and C++ compilation cargo's build scripts drive wherever the platform's C compiler can be wrapped, through a compiler cache whose store is remote and shared across runs, channels, and workflows. The cache SHALL be keyed on the compiler, its flags, and its inputs, so that a cache entry is reused only by a compilation that would have produced identical output.

#### Scenario: A second run reuses the first run's compilation

- **WHEN** a release build job compiles a crate that an earlier release build job on the same platform already compiled with the same compiler, flags, and sources
- **THEN** the compilation is served from the shared store rather than repeated

#### Scenario: C sources compiled by cargo are cached

- **WHEN** a release build compiles a crate's bundled C or C++ sources through cargo's build script on a platform whose C compiler can be wrapped
- **THEN** those compilations are served from, or added to, the same store as the Rust compilations

#### Scenario: Channels share the store

- **WHEN** a stable build compiles a crate that a nightly build already compiled identically
- **THEN** the compilation is served from the same store; no channel has a store of its own

#### Scenario: A build that would differ misses

- **WHEN** a release build compiles a crate whose sources, flags, or compiler differ from every cached compilation
- **THEN** the compiler runs and the result is added to the store

#### Scenario: Local builds are unaffected

- **WHEN** a release build is run outside CI with the same recipe
- **THEN** no compiler wrapper is in effect and the remote store is not contacted

### Requirement: Fetched directories are cached as immutable snapshots

A generated release build job SHALL restore, before the step that would otherwise fetch it, each cached directory from the shared store under a key derived from the pinned manifests that determine that directory's contents and the platform. On a miss the consumer SHALL fetch as it would without a cache, and the job SHALL then save the directory under that key. A key SHALL be written at most once: a save SHALL NOT overwrite an existing entry.

#### Scenario: Restore precedes the consumer

- **WHEN** a build job reaches a step that would fetch a cached directory's contents
- **THEN** a restore for that directory's key has already run, and on a hit the consumer finds its contents in place and fetches nothing

#### Scenario: Miss fetches and then saves

- **WHEN** no entry exists for a directory's key
- **THEN** the consumer fetches normally and, after it succeeds, the directory is saved under that key

#### Scenario: Existing entry is never overwritten

- **WHEN** a save runs for a key that already exists in the store
- **THEN** the store's entry is left unchanged

#### Scenario: Manifest change changes the key

- **WHEN** a manifest that pins a cached directory's contents changes
- **THEN** the next build derives a different key, misses, fetches fresh, and saves a new entry; the old entry is untouched

### Requirement: A directory is cached only where its manifest carries content hashes

A directory SHALL be eligible for caching only when the manifest its key is derived from pins the content of what it holds by hash, so that a restored snapshot is exactly what a fresh fetch would have been required to verify. Where a build tool does not record such hashes by default, the repository SHALL enable the tool's verification so that it does, and the build SHALL fail on a mismatch.

#### Scenario: Android dependencies are verified

- **WHEN** the Android build resolves a dependency or plugin artifact
- **THEN** its checksum is compared against the committed verification metadata, and a mismatch fails the build

#### Scenario: Gradle distribution comes from the tool projection

- **WHEN** the Android build runs Gradle in CI
- **THEN** it runs the distribution the tool projection installs and its lockfile pins by checksum; no wrapper download takes place

#### Scenario: Verification metadata is regenerated on demand

- **WHEN** the Android dependency set changes
- **THEN** a documented recipe regenerates the verification metadata, and until it is regenerated the Android build fails naming the unverified artifacts

### Requirement: The store is written and read only under a CI-held credential

The store SHALL accept neither reads nor writes without a credential, and that credential SHALL reach a job only through the environment the job declares for its primary work. No run that lacks such an environment SHALL be able to write to the store, so that restoring from it in a publishing job is restoring only what a trusted job wrote.

#### Scenario: A pull-request run cannot write to the store

- **WHEN** a workflow run outside an environment holding the credential attempts to use the store
- **THEN** it holds no credential for it and the store rejects the request

#### Scenario: The publishing-job cache guard holds

- **WHEN** the generated release workflows are checked for caches in jobs that declare an environment
- **THEN** no cache action is present; every process that contacts the store runs under a scoped resolution of the store credential; a step that wraps the compiler follows the step that started the cache server under that resolution; and all of them sit in a job declaring an environment

### Requirement: The store credential reaches only the processes that contact the store

The store's location and credential SHALL be declared in the repository's secret manifest under one scope of the CI profile, so that resolving that scope yields exactly those values. Each process that contacts the store — the compiler-cache server and the store client — SHALL be run under a resolution of that scope that lasts only for that process. The credential SHALL NOT be exported into the job environment, placed in any step's configuration, or made visible to the build or to anything that runs inside it.

#### Scenario: The scope resolves only the store's values

- **WHEN** the CI profile is checked or resolved under the store's scope
- **THEN** it resolves the store's location and credential and no automation or signing secret

#### Scenario: Each resolving step is self-describing

- **WHEN** a step that resolves the store's scope is inspected in the generated workflow
- **THEN** it names the provider, supplies the vault credential, states its reason, and the enclosing job and workflow name none of these

#### Scenario: The build sees no credential

- **WHEN** the build step and the processes it spawns are inspected on any platform
- **THEN** no store credential and no store endpoint is present in their environment; compilation reaches the store only through the already-running cache server

#### Scenario: The job environment holds no credential

- **WHEN** any step's environment or configuration in a generated workflow is inspected
- **THEN** it names no store credential and no store endpoint

### Requirement: Clean-room builds carry the compiler wrapper

A build recipe that scrubs its environment SHALL forward the compiler wrapper selection into the build, and nothing of the store's configuration, so that a scrubbed build caches exactly as an unscrubbed one does while the compile remains credential-free. A build recipe that selects the C and C++ compilers SHALL apply the same wrapper to them whenever one is in effect, and SHALL NOT apply it to the linker.

#### Scenario: A scrubbed target compiles through the cache

- **WHEN** a release build runs on a platform whose recipe scrubs the ambient environment
- **THEN** the compiler inside the scrub is wrapped by the cache and reaches the already-running server, no store credential or endpoint crosses the scrub, and the signing material still reaches only the build process

#### Scenario: Recipe-selected C compilers are wrapped

- **WHEN** a release build runs on a platform whose recipe names the C and C++ compilers and hands them to cargo's build scripts, and a compiler wrapper is in effect
- **THEN** those build scripts invoke the compilers through the wrapper, and the linker is invoked directly

#### Scenario: An unscrubbed target compiles through the cache

- **WHEN** a release build runs on a platform whose recipe keeps the ambient environment
- **THEN** the compiler is wrapped by the cache and reaches the store with no recipe change beyond the forwarded set

### Requirement: An unreachable store degrades to a cold build

A release build SHALL NOT fail because the cache store is unreachable or rejects a request. If the compiler-cache server cannot be started against the store, the job SHALL continue and the build SHALL compile without the remote store; a compiler-cache client that loses its server mid-build SHALL compile directly rather than fail; a directory-cache restore failure SHALL be treated as a miss; a directory-cache save failure SHALL be reported and SHALL NOT fail the job. The build SHALL still produce its artifacts, and the job's cache report SHALL show which store, if any, was in use.

#### Scenario: Store outage during a release

- **WHEN** the store cannot be reached when a release build starts, or fails mid-build
- **THEN** every restore reports a miss, the build completes from a cold compile, and the job's cache reports show the failures and the store that was actually used

### Requirement: The cache tools are projected, not fetched

The compiler cache binary and the store client SHALL be provided to CI by the same tool projection as every other tool — one declared version each, pinned by the committed lockfiles, present only in the lanes that build release artifacts. No workflow SHALL install either through a marketplace action.

#### Scenario: Only build lanes carry the tools

- **WHEN** the projected tool configurations are inspected
- **THEN** the compiler cache and the store client appear in the desktop and mobile build lanes and in no other lane

#### Scenario: Version moves with the projection

- **WHEN** a declared version changes
- **THEN** the projected configurations and lockfiles change with it and the generated-file lockstep check covers the move; no workflow names a version of its own

### Requirement: Every caching job reports what its caches did

Every generated job that uses the store SHALL log, for each directory cache, whether its key hit or missed, and SHALL end with a report of the compiler cache's hits, misses, and errors for that job and the store it used, after which the compiler-cache server SHALL be stopped. The compiler-cache report SHALL run whether or not the build succeeded.

#### Scenario: A successful build reports

- **WHEN** a release build job finishes its build
- **THEN** the job log shows each directory cache's hit or miss and the compiler cache's hit, miss, and error counts and store location for that job, and the server holding the credential is no longer running when the upload steps begin

#### Scenario: A failed build still reports

- **WHEN** a release build job's build step fails
- **THEN** the compiler-cache report still runs, so a store problem that caused the failure is visible in the same log

#### Scenario: A warm rebuild of the same commit is a determinism probe

- **WHEN** a release build job runs twice against the same commit, pins, and toolchain
- **THEN** the second run's report shows every directory cache hit and the compiler cache hit on every dependency compilation; a miss on an unchanged dependency is a defect to investigate, not an expected outcome
