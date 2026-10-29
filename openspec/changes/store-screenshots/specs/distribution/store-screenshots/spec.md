## ADDED Requirements

### Requirement: A listing screenshot depicts the shipping build

Every phone screenshot in a store listing SHALL embed a capture of the application as built, taken by the capture harness. No screenshot SHALL depict application UI that was drawn rather than captured.

Everything around the capture — headline, supporting line, background, layout, device frame — is composition and SHALL remain the renderer's.

#### Scenario: A screen is ported to a capture

- **WHEN** a screen's body is a drawn mock of the editor
- **THEN** it is replaced by a reference to a capture, and the headline and supporting line are unchanged

#### Scenario: A capture is missing

- **WHEN** a screen references a capture that is not present in the tree
- **THEN** the render fails and names the missing capture

#### Scenario: Composition is not depiction

- **WHEN** a surface draws the device frame, background and headline around a capture
- **THEN** that is conforming, because none of it depicts application UI

### Requirement: A capture is taken against the demo territory

A capture SHALL be taken against the demo territory — a deterministic corpus produced from the repository at the commit being captured — never against an empty application or a real user's data.

The territory SHALL be placed on the device before the release build first launches, as a returning user's data would be. The harness SHALL NOT author content, and the application SHALL NOT be built differently to carry it.

#### Scenario: Capturing an installed build

- **WHEN** the harness installs a release build and places the demo territory in its data directory before first launch
- **THEN** the application opens onto that territory, with no onboarding or editing performed by the harness

#### Scenario: Two runs of the same capture

- **WHEN** the same capture is taken twice on the same build
- **THEN** the content shown is the same both times

#### Scenario: The territory is absent

- **WHEN** the application launches for capture with no territory placed
- **THEN** the capture is refused rather than producing an empty screen

#### Scenario: The territory comes from the commit

- **WHEN** the demo territory is produced for a capture
- **THEN** it is produced by that commit's own store code, so its schema is the one the build opens

#### Scenario: A different Bundle

- **WHEN** the territory is produced from a Bundle other than the default
- **THEN** it is ingested through the same path, and the harness that places it is unchanged

### Requirement: A deep link names an application route

A deep link's path SHALL be a route the application already serves. No parallel vocabulary of capture scenes SHALL be introduced, and a route SHALL require no registration to become linkable.

The capture harness SHALL reach a screen by opening such a link. It SHALL NOT navigate the interface, address webview content, or depend on the structure of the DOM.

#### Scenario: Capturing a screen

- **WHEN** the harness captures a screen
- **THEN** it opens a link naming that screen's route, waits for the application to settle, and captures, naming no element inside the webview

#### Scenario: Linking to a route

- **WHEN** a link names `/library`
- **THEN** the application navigates there, with nothing registered for that route beyond the route itself

#### Scenario: The frontend's markup changes

- **WHEN** the application's markup changes without changing which route a screen is at
- **THEN** no harness code changes

### Requirement: The link scheme identifies the flavor

Each flavor SHALL register a scheme that names it, so a link resolves to one installed application and not to whichever the operating system happens to choose. The base configuration SHALL NOT declare a flavor-independent scheme.

#### Scenario: Flavors installed side by side

- **WHEN** stable and nightly are both installed and a nightly link is opened
- **THEN** nightly receives it, and stable does not

#### Scenario: Capturing a known flavor

- **WHEN** the harness captures against a build it has just installed
- **THEN** the link it opens addresses that flavor's scheme

### Requirement: A deep link navigates and never acts

A deep link SHALL route the application to a destination. It SHALL NOT perform the action that destination offers.

Adding a link kind SHALL NOT change how existing kinds behave.

#### Scenario: A link carrying a setup code

- **WHEN** a pairing link arrives
- **THEN** the code is prefilled for confirmation and is never submitted by the link alone

#### Scenario: A route link in a release build

- **WHEN** a link naming a route is opened against a release build
- **THEN** the application navigates there, because navigation is not a capability

#### Scenario: Pairing after a kind is added

- **WHEN** a pairing link arrives after route links exist
- **THEN** it behaves exactly as it did before

### Requirement: A deep link may pose the page it names

A link MAY carry presentation state — which item is selected, which panel is open, which tab is shown — in its query. A page SHALL read such state from the route it is at through one shared convention, SHALL ignore keys it does not declare, and SHALL NOT create, edit or delete anything in response to a key.

#### Scenario: A posed link

- **WHEN** a link names `/library` and carries presentation state in its query
- **THEN** the library opens in that state, and nothing is created, edited or deleted

#### Scenario: An undeclared key

- **WHEN** a link carries a key the page it names does not declare
- **THEN** the page opens as the bare route would, and the key is ignored

#### Scenario: The demo capture is posed

- **WHEN** the demo screen is captured
- **THEN** its link carries at least one pose, so the capture shows what the bare route could not

### Requirement: A capture is minted from a build of the commit it depicts

A capture SHALL be taken against a build of one commit, in one flavor, by a command a person runs. The capture SHALL record nothing the harness invented: it is the installed build, over the placed territory, at the linked route.

Minting SHALL remain a separate operation from rendering, ordered before it. The renderer SHALL read captures from a path keyed by platform, locale and name, and SHALL NOT take them itself.

#### Scenario: Minting on a developer's machine

- **WHEN** the capture command is run for a platform
- **THEN** it boots the device, installs a build of the current commit, places the territory, opens the route and writes the capture, invoking no renderer

#### Scenario: The store package can be installed

- **WHEN** the package a store would receive installs on the capture device, as the universal Android apk does on an emulator
- **THEN** the capture is taken on that package, unmodified

#### Scenario: The store package cannot be captured

- **WHEN** the package a store would receive cannot run on any device we control, as a distribution-signed ipa cannot
- **THEN** the capture is taken on a build of the same commit and flavor for the simulator, built no differently otherwise

#### Scenario: Rendering before capture

- **WHEN** a listing is rendered and a referenced capture has not been taken
- **THEN** the render fails and names it, rather than emitting a listing with a hole in it

#### Scenario: A second capture is added

- **WHEN** another screen is captured later
- **THEN** it is a new file under the same keyed path, requiring no change to the path's shape

### Requirement: A capture is composed onto the store's canvas

A capture SHALL be embedded in a device frame, and the frame placed on the canvas the destination slot requires. A capture's own pixel dimensions SHALL NOT determine the dimensions of an uploaded asset.

#### Scenario: Device and slot disagree

- **WHEN** a capture is 1206×2622 and the slot requires 1320×2868
- **THEN** the capture is composed onto the slot's canvas rather than scaled to it or uploaded as taken

#### Scenario: Aspect ratios differ

- **WHEN** a capture is 20:9 and the slot's canvas is 9:16
- **THEN** the composition absorbs the difference, and the uploaded asset matches the slot

#### Scenario: Both stores covered

- **WHEN** the demo is rendered
- **THEN** the App Store surface composes an iPhone 17 Pro capture and the Play surface a Pixel 10 Pro capture, each onto its own canvas

### Requirement: A device frame is a sourced render

A device frame SHALL be a render of the device the capture was taken on, held in the tree and never committed, and described by a record naming its image, screen rect, corner radius and capture scale. The capture SHALL be clipped to the screen rect and the frame drawn over it, so the frame's own cutouts occlude the capture.

#### Scenario: Composing into a frame

- **WHEN** a capture is composed into its device's frame
- **THEN** the capture is clipped to the recorded screen rect and the frame is drawn over it, so the camera cutout covers the capture as the hardware does

#### Scenario: Adding a device

- **WHEN** a new device frame is wanted
- **THEN** it is a render exported under the licence, keyed, placed in the frames directory and named by a record, and nothing new enters history

#### Scenario: A frame is absent

- **WHEN** a referenced frame image is not in the tree
- **THEN** the render fails naming it and where to obtain it, rather than with a bare file-not-found
