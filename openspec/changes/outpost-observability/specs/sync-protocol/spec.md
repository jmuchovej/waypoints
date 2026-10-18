## ADDED Requirements

### Requirement: A Station keeps an events table

A Station with its own database SHALL keep an events table in it. Each event SHALL have a sequence number that increases by one per event and never repeats across restarts, a timestamp, a kind from the closed set below, and the fields its kind requires. Every field in the table below SHALL be a typed column of the table, one column per field name, nullable where a kind does not use it; the table SHALL hold no free-form document. The Station SHALL append an event only after the decision it records has taken effect, and SHALL append one for every occurrence below. Events SHALL carry only metadata the Station already holds — Territory ids, member ids, device public keys, counts, durations, reasons — and SHALL NOT carry Notice payloads, envelopes, read keys, or any key material. Ids and keys SHALL be rendered as lowercase hex wherever they are shown. Kinds and reasons SHALL be drawn from the closed set in the table; a new kind or reason is an addition to this table, and renaming one is a breaking change to the Station's contract with its commodore. A Station without its own database (a seeded, storage-less Station) SHALL record no events.

`member` names whoever the event is about — the registrant, the authenticated caller, the subscriber, or the record's author — since the kind already says which role they played.

| Kind                   | Level | Fields                                                                                                                                                            |
| ---------------------- | ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `territory registered` | info  | `territory`, `member`                                                                                                                                             |
| `registration refused` | warn  | `territory`, `reason` (`policy`, `invalid log`, `mismatched id`, `no storage`), `member` when authenticated, `count` and `since` when coalesced                   |
| `session refused`      | warn  | `reason` (`bad hello`, `not served`, `not a member`, `unexpected frame`), `territory` when known, `member` when authenticated, `count` and `since` when coalesced |
| `subscription reaped`  | info  | `territory`, `member`, `device`, `reason` (`idle`, `lagged`, `closed`)                                                                                            |
| `order refused`        | info  | `territory`, `member`, `reason` (`not served`, `no fleet key`, `does not chain`)                                                                                  |
| `holds loaded`         | info  | `count`, `duration_ms`                                                                                                                                            |
| `station running`      | info  | `port`, `address`, `data`, `orderer`                                                                                                                              |
| `shutting down`        | info  | `open_sessions`                                                                                                                                                   |
| `station stopped`      | info  | `close_ms`, `timed_out`                                                                                                                                           |

Events SHALL be retained for a period the commodore configures as a duration written the way a person writes one (`3mo`, `6wk`, `90d`), three months by default and never less than one day. The Station SHALL remove events older than the retention period at start-up and on a schedule thereafter, no less often than hourly, and SHALL always keep the most recent event whatever its age, so the sequence is recoverable after any idle stretch.

Calendar units in the period SHALL be calendar-accurate rather than averaged: a period in months retains back to the same day of the month, clamped where that month is shorter, so a three-month window is between 89 and 92 days depending on the day the prune runs. A period written in days is exact, and is how a commodore asks for a window of fixed length.

Refusals SHALL be coalesced per source. The first refusal of a given reason from a connection's remote address SHALL be recorded at once with `count` 1; further refusals of that reason from that address within the following minute SHALL be counted rather than recorded, and once the minute has passed — at the latest at the next event the Station records, or at shutdown — one event SHALL be appended carrying the `count` of refusals it stands for and `since`, the time of the first of them. The address is the coalescing key only and SHALL NOT be written to the event. Coalescing bounds the record under a flood; it does not bound the flood.

#### Scenario: A homed Territory is recorded

- **WHEN** an Owner registers a Territory the Station did not hold
- **THEN** the table gains exactly one `territory registered` event carrying that Territory's id and the registrant's member id, appended after the hold exists on disk

#### Scenario: A refusal says why

- **WHEN** a registrant without Owner permission registers a Territory under the default policy
- **THEN** the table gains a `registration refused` event with the Territory id, the reason `policy`, and the registrant's member id

#### Scenario: The fields are columns

- **WHEN** the events table's schema is inspected
- **THEN** every field named in the table above is a column of its own with a scalar type, and no column holds a document

#### Scenario: The sequence survives a restart

- **WHEN** a Station whose newest event has sequence N is restarted and then records an event
- **THEN** that event has sequence N+1, including when every older event has been pruned

#### Scenario: Payloads never reach the table

- **WHEN** a Vessel syncs a Notice whose plaintext contains a known sentinel
- **THEN** no event's fields contain the sentinel, the ciphertext, or the envelope bytes

#### Scenario: Old events are pruned on schedule

- **WHEN** the table holds events older than the retention period and the Station starts, or an hour passes while it runs
- **THEN** those events are removed, and the most recent event remains whatever its age

#### Scenario: The period is written as a duration

- **WHEN** a Station is started with a retention period of `6wk`
- **THEN** it starts, and an event dated seven weeks ago is pruned while one dated five weeks ago is kept

#### Scenario: A months period counts calendar months

- **WHEN** a Station with the default retention prunes on 31 May 2027
- **THEN** events dated before 28 February 2027 are removed and one dated 1 March 2027 is kept — the window is the calendar three months, clamped into the shorter month, rather than a fixed count of days

#### Scenario: Retention below a day is refused

- **WHEN** a Station is started with a retention period of `12h`
- **THEN** it exits with a non-zero status and an error naming the option

#### Scenario: A flood of refusals is a handful of rows

- **WHEN** a caller at one address is refused the same way five hundred times within one minute
- **THEN** the table gains a `session refused` event with `count` 1 at the first refusal, one more with `count` 499 and `since` the first refusal's time once the minute has passed, and no others

#### Scenario: A storage-less Station records nothing

- **WHEN** a Station seeded with a fixed replica and no storage directory serves a session
- **THEN** no events table exists and no event is reported

### Requirement: Events are reported through the log as a read-only stream

Each event the Station appends SHALL be reported as one log record, at the kind's level, whose message is the kind and whose fields are the event's fields plus `seq`, the event's sequence number, produced from the row after it is written. An event that could not be written SHALL NOT be reported as an event; the failed write SHALL be reported as a diagnostic. Diagnostics — failures, and per-session detail such as a repeat registration, a subscription parked, or an order granted — are not events: they SHALL NOT be written to the table, and their names and fields are not part of this contract. In the structured format each event record SHALL be one JSON object whose `fields.message` is the kind, with ids as hex strings and counts as numbers.

#### Scenario: A line traces to a row

- **WHEN** a `territory registered` record with sequence S appears in the log
- **THEN** the table holds event S with the same kind and fields

#### Scenario: A failed write is not announced

- **WHEN** the Station's database refuses writes and an Owner registers a Territory
- **THEN** the registration succeeds, the log carries a diagnostic warning about the events table, and no `territory registered` record

#### Scenario: Diagnostics stay out of the table

- **WHEN** a Territory the Station already serves is registered again
- **THEN** the table gains no event, and a diagnostic is reported at a level below the default floor

#### Scenario: Events are machine-readable in the structured format

- **WHEN** the Station runs with the structured log format
- **THEN** each event record parses as one JSON object with `fields.message` equal to the kind, `fields.seq` a number, and the kind's fields present with ids as hex strings

## MODIFIED Requirements

### Requirement: Station logs can be emitted as structured records

A Station SHALL offer a structured log format in which every log line is a single JSON object carrying at least a timestamp, a level, and a message, selectable at start-up. The default remains the human-readable format.

The log stream SHALL carry every record at or above a minimum level and nothing below it. The minimum SHALL be set at start-up through the `RUST_LOG` environment variable, in that variable's usual syntax, and SHALL default to informational for the Station's own crates. Every event kind is at or above that default, so the default stream is the complete operational record; diagnostics sit below it. The floor SHALL NOT affect what the events table records. Narrowing what is shown below the floor is the log consumer's concern; the Station offers no finer control.

#### Scenario: Structured output

- **WHEN** a Station is started with the structured log format selected
- **THEN** every line it writes to its log output parses as a JSON object with timestamp, level, and message fields

#### Scenario: Default is unchanged

- **WHEN** a Station is started without selecting a log format
- **THEN** its log output is the human-readable format

#### Scenario: The default floor is the operational record

- **WHEN** a Station runs at the default floor through a registration, a refused registration, a reaped subscription, and a stop
- **THEN** each event in the table for those occurrences appears in the log exactly once, and no diagnostic does

#### Scenario: Lowering the floor reveals diagnostics

- **WHEN** a Station is started with `RUST_LOG=outpost=debug` and a Territory is registered twice
- **THEN** the repeat registration's diagnostic appears in the log, and other crates' debug output does not

#### Scenario: The floor does not reach the table

- **WHEN** a Station is started with `RUST_LOG=outpost=warn` and an Owner registers a Territory
- **THEN** no `territory registered` record appears in the log, and the table gains the event
