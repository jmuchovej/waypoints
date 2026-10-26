## ADDED Requirements

### Requirement: A tool fetched at run time is a justified exception, never a precedent

Every tool a consumer outside the shell uses SHALL be projected, unless that tool meets every condition below, in which case it MAY instead be fetched by the step that needs it, at whatever version its vendor currently serves:

- its vendor publishes only a current build at an unversioned location, so no pinned version could be fetched again once replaced;
- it updates itself when run, so a pinned version would not determine what actually executes;
- the step that fetches it runs before any act that cannot be undone, so a failed fetch leaves nothing to withdraw beyond what any early failure leaves;
- it is fetched over HTTPS, failing on any error response, and every integrity check the artifact itself carries is left enabled;
- what it is used for can be completed by hand from artifacts the run retains, so a failure caused by a new vendor build costs a manual step rather than a release.

Each such tool SHALL be named, with each condition shown to hold, in the design of the change that introduces it. Meeting the conditions for one tool SHALL NOT be cited as grounds for fetching another: a later tool SHALL be justified against the conditions afresh, and a tool that later becomes available at a pinnable version SHALL be projected instead.

#### Scenario: A pinnable tool is projected

- **WHEN** a tool's vendor publishes versioned builds that remain fetchable
- **THEN** the tool is projected like any other, even if fetching it at run time would be simpler

#### Scenario: A failed fetch is retractable

- **WHEN** fetching a run-time tool fails
- **THEN** the run fails before any irreversible act, and is withdrawn like any other early failure

#### Scenario: An exception carries its own justification

- **WHEN** a change introduces a tool fetched at run time
- **THEN** its design names the tool and shows each condition holds for it, and does not rely on an earlier exception having been granted
