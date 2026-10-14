## ADDED Requirements

### Requirement: A Station reports its operations as structured events

A Station SHALL emit an event for each of the occurrences below, at the stated level, carrying at least the stated fields, in every log format it offers. Event names and field names SHALL be stable: changing one is a breaking change to the Station's operator contract. Events SHALL carry only metadata the Station already holds — Territory ids, member ids, device public keys, counts, durations, reasons — and SHALL NOT carry Notice payloads, envelopes, read keys, or any key material.

| Event                     | Level | Fields                                                                                                                          |
| ------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------- |
| `territory registered`    | info  | `territory`, `registrant`                                                                                                       |
| `territory re-registered` | debug | `territory`, `registrant`                                                                                                       |
| `registration refused`    | warn  | `territory`, `reason` (`policy`, `invalid log`, `mismatched id`, `no storage`), `registrant` when authenticated                 |
| `session refused`         | warn  | `reason` (`bad hello`, `not served`, `not a member`, `unexpected frame`), `territory` when known, `identity` when authenticated |
| `subscription parked`     | debug | `territory`, `member`, `device`                                                                                                 |
| `subscription reaped`     | info  | `territory`, `member`, `device`, `reason` (`idle`, `lagged`, `closed`)                                                          |
| `order granted`           | debug | `territory`, `author`, `position`                                                                                               |
| `order refused`           | info  | `territory`, `author`, `reason` (`not served`, `no fleet key`, `does not chain`)                                                |
| `holds loaded`            | info  | `count`, `duration_ms`                                                                                                          |
| `station running`         | info  | `port`, `address`, `data`, `orderer`                                                                                            |
| `shutting down`           | info  | `open_sessions`                                                                                                                 |
| `station stopped`         | info  | `close_ms`, `timed_out`                                                                                                         |

Ids and keys SHALL be rendered as lowercase hex. Reasons SHALL be drawn from the closed set above; a new reason is an addition to this table.

#### Scenario: A homed Territory is announced

- **WHEN** an Owner registers a Territory the Station did not hold
- **THEN** the Station emits `territory registered` at info with that Territory's id and the registrant's member id, exactly once

#### Scenario: A repeat registration is quiet

- **WHEN** a Territory the Station already serves is registered again
- **THEN** the Station emits `territory re-registered` at debug and nothing at info or above

#### Scenario: A refusal says why

- **WHEN** a registrant without Owner permission registers a Territory under the default policy
- **THEN** the Station emits `registration refused` at warn with the Territory id, the reason `policy`, and the registrant's member id

#### Scenario: The scan is accounted for

- **WHEN** a Station starts with stored Territories
- **THEN** it emits `holds loaded` with the number of holds opened and the milliseconds the scan took, before `station running`

#### Scenario: Payloads never reach the log

- **WHEN** a Vessel syncs a Notice whose plaintext contains a known sentinel through a Station logging at the most verbose level
- **THEN** no line of the Station's log contains the sentinel, the ciphertext, or the envelope bytes

#### Scenario: Events are machine-readable in the structured format

- **WHEN** the Station runs with the structured log format
- **THEN** each event is one JSON object whose `fields.message` is the event name and whose other `fields` are the fields above, with ids as hex strings and counts as numbers

## MODIFIED Requirements

### Requirement: Station logs can be emitted as structured records

A Station SHALL offer a structured log format in which every log line is a single JSON object carrying at least a timestamp, a level, and a message, selectable at start-up. The default remains the human-readable format.

The Station SHALL emit every event at or above a minimum level and nothing below it. The minimum SHALL be set at start-up through the `RUST_LOG` environment variable, in that variable's usual syntax, and SHALL default to informational for the Station's own crates. The levels in the event table are assigned so that the default floor is the complete operational record: refusals at warn, decisions an operator acts on at info, per-session detail at debug. Narrowing what is shown below the floor is the log consumer's concern; the Station offers no finer control.

#### Scenario: Structured output

- **WHEN** a Station is started with the structured log format selected
- **THEN** every line it writes to its log output parses as a JSON object with timestamp, level, and message fields

#### Scenario: Default is unchanged

- **WHEN** a Station is started without selecting a log format
- **THEN** its log output is the human-readable format

#### Scenario: The default floor is the operational record

- **WHEN** a Station runs at the default floor through a registration, a refused registration, a reaped subscription, and a stop
- **THEN** every info and warn event in the table for those occurrences appears, and no debug event does

#### Scenario: Lowering the floor reveals detail

- **WHEN** a Station is started with `RUST_LOG=outpost=debug`
- **THEN** `subscription parked` and `territory re-registered` appear in the log, and other crates' debug output does not
