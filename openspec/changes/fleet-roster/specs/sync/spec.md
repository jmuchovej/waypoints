## ADDED Requirements

### Requirement: A Vessel homed to a Fleet trusts only rostered Stations

When the sync configuration names a Fleet — its identity and the roster it trusts — every dial the Vessel makes (registration, sync, subscription, ordering) SHALL first verify that the Station's authenticated identity is listed in that roster, and SHALL refuse the Station otherwise. The Vessel SHALL fetch the roster from its configured Station when it holds none, SHALL replace its roster when the Station serves one with a higher revision that verifies, and SHALL reject a roster with a lower revision than the one it trusts. Without a Fleet in the configuration the Vessel SHALL behave as it does today.

#### Scenario: An unrostered Station is refused on every path

- **WHEN** the configuration names a Fleet and the configured Station's identity is not in the trusted roster
- **THEN** registration, sync, subscription, and ordering each fail with an error naming the Station as untrusted, and no Territory data is sent to it

#### Scenario: The roster is fetched on first use

- **WHEN** the configuration names a Fleet identity and no roster, and the configured Station serves a roster that verifies against it
- **THEN** the Vessel stores that roster and proceeds

#### Scenario: A newer roster replaces the held one

- **WHEN** the Station serves a roster with a higher revision that verifies
- **THEN** the Vessel adopts it and records the new revision as trusted

#### Scenario: Rollback is refused

- **WHEN** the Station serves a roster with a lower revision than the trusted one
- **THEN** the Vessel keeps its roster and reports the attempt

### Requirement: Pairing names the Fleet

The pairing payload a new device receives SHALL carry the Fleet identity beside the Station address when the inviting device is homed to a Fleet. A joining device SHALL fetch the roster from that Station, verify it against the identity, and verify the Station against it before registering or syncing. A payload without a Fleet identity SHALL keep the single-Station behaviour.

#### Scenario: A joining device bootstraps its trust from the payload

- **WHEN** a new device completes pairing from a payload carrying a Fleet identity and a Station address
- **THEN** its configuration names that Fleet with a roster that verifies, and the Station it registered with is listed in it

#### Scenario: A Station not in the Fleet cannot be injected through pairing

- **WHEN** a payload names a Fleet identity but a Station whose identity is not in the roster that Station serves
- **THEN** the joining device refuses to register and reports the Station as untrusted
