# Validation ledger

## Completed

- Extracted/checked 416 trigger/NPC map ranges: 1164 triggers and 2193 NPC
  records. All source coordinates and 1122 hexadecimal targets match. Mapped
  initial NPC flags, six patched NPC bytes and the real opening actor palettes.
  Cached sort ordering, two-phase object commands/movement, and slot-list
  snapshot preservation have direct Rust coverage.
- Added live trigger dispatch, covered control-flow commands and direct VM
  snapshots. The backtracking scene completes and the first guards reach a
  typed battle-group-1 pause. `verify_field_events.py` checks 135 source-encoded
  bytes, 84 native commands, gated return, four actual walking captures and
  snapshot continuation. No battle result is simulated. See [field-events.md](field-events.md).
- Corrected object position/copy commands to yield one update as source requires;
  opening diagnostics now span 417 logical frames. Version 8 saves retain VM
  cursors/tasks/calls directly; old-format opening replay remains a migration path.

- Continuous player movement now uses target fixed-point speed, four-frame
  action phase, locked velocity, aligned entrance checks and smooth scrolling.
  The target Cancel-dash and Sliding Dash hooks are mapped to CPU/file offsets.
  Their bounded 65816 behavior matches 512 speed and 1024 gait inputs. All 97
  native movement states and 28 complete pixel frames match independent checks.
  Version 8 saves retain in-flight player motion and its clock. See
  [player-movement.md](player-movement.md) and `player-movement-validation.json`.
  Sprint Shoes equipment integration, diagonal movement and original timing
  remain open.

- Added Magitek bodies/riders from target lookup tables, 4bpp graphics and
  palette. `verify_magitek.py` compares 32 configurations (four directions,
  four gait phases, rider on/off): 114,688 pixels match an independent raw-ROM
  decoder. Vehicle flags survive return to control and save/load. The relevant
  formation/frame tables and vehicle graphics/palette are unchanged between
  ROMs; rider art comes from the patched ROM. See `magitek-validation.json` and
  [magitek-sprites.md](magitek-sprites.md). Original PPU priority, frame origin,
  DMA latency and hardware sprite limits remain unverified.

- Added live scheduling for the supported entry: events run before object
  commands and fixed-point movement, following pinned `reset.asm::FieldLoop`.
  Source-derived speed/wait rules, concurrent movement, awaited completion,
  script replacement and in-flight save reconstruction pass checks. The game
  renders Terra and both soldiers, with smooth follow-camera movement. See
  `opening-timeline-validation.json`, `captures/native-opening-motion/` and
  [event-vm.md](event-vm.md). Collisions, fades, audio and
  original timing/animation remain open. The live scheduler executes 78
  commands; two structurally decoded commands are superseded before execution.
  Current evidence: 417 logical frames, 165 frames with concurrent actor movement
  and 1,251 matching game/scheduler actor rows. Unsupported events stay halted
  after their diagnostic is dismissed.

- `verify_opening_event.py` independently encodes all 134 bytes of `_cc9a4f`
  from pinned source macros, compares both raw ROMs and exports, and checks all
  80 native event/object command spans. Party, gil, switches, vehicle flags,
  actor endpoints and DLG_11 yield/return states pass. Timing, parallelism,
  audio and presentation are not validated by this endpoint check. See
  `opening-event-validation.json` and [event-vm.md](event-vm.md).
- The target dialogue catalog supplies DLG_11 directly, preserves encoded line
  breaks and rejects unknown controls. Tests cover malformed event commands,
  explicit newlines, old saves and resuming the bytecode pause without extra gil.

- Added raw point/line entrance records to field bundles and direct sub-map
  traversal after successful tile movement. Four source-backed map changes
  (19→39→41→42→41) pass a native 121-input-step diagnostic. The native path
  is checked against patched ROM exit records and five independently rendered
  backgrounds; see `field-traversal-validation.json`. The traversal uses explicit geometry-only mode; live story triggers now stop
  at the first unimplemented battle. Startup scripts remain unimplemented.
- Audited 512 map ranges per entrance table per ROM: 1,129 point records and
  152 line records each. Pointer-plus-record ranges are unchanged between the
  clean and target ROMs. Repeated pointers represent empty map ranges; their
  provenance index lengths describe the next distinct target, not map contents.
- Corrected compact-layout geometry using pinned `InitScrollClip` source:
  maps 18/23 are 32×32, replacing the previous 16×64 inference. Mine maps
  41–43 are 128×64. Metadata v2 stores both dimensions and both renderers apply
  horizontal/vertical masks. Raw MapProp-to-metadata comparisons pass.

- Installed the official Rust 1.99.1 MSVC toolchain inside project-local
  `.tools/`; the Rustup installer SHA-256 matched its published digest
  `6F4BEF66261261FCB43131BE8720BAB817D403A09EDEC7455C371974B90BDB7E`.
  Existing Visual Studio 2022 C++ tools supply the linker. The wrapper
  `tools/cargo-local.ps1` provides a reproducible local Cargo invocation.
- `tools/cargo-local.ps1 test --offline` passes all 41 Rust tests, including
  opening-state progression, quicksave resume, message completion isolation,
  palette conversion, whole-actor mirroring, map collision, inclusive point/line
  exit matching and actual Narshe/mine transitions.
- The release build succeeds. `--smoke-window` creates the native Win32/GDI
  window, presents three frames, and closes normally; the result and executable
  digest are in `native-window-smoke.json`.
- `cargo package --list --offline` lists source/documentation files and
  excludes ROMs, IPS patches, `Patch/`, `.tools/`, extracted assets, captures
  and Python bytecode. The actual file list and manifest digest are preserved
  in `package-audit.json`.
- `tools/verify_native_opening.py` captures eight frames from the actual Rust
  renderer, checks opening state endpoints, and compares the player-control
  background against the separate Python asset renderer. All 56,128 pixels
  outside Terra's 32x38 Magitek/rider rectangle match; 715 actor pixels differ from the
  background. See `native-render-comparison.json`. This is implementation
  agreement and does not validate original SNES presentation.
- Dialogue completion actions now belong to their messages. Normal messages
  cannot advance the prologue. The version 7 prototype quicksave retains event bits and the
  current opening line, resumes its event action, and clears stale battle state.
  Versions 2 through 6 remain readable. These are local game-state tests, not SRAM
  compatibility tests.

- Corrected Terra's opening state from the complete `_cc9a4f` movement
  sequence: map 19 loads at `(38, 58)`, she moves to `(38, 50)` for DLG_11,
  then `(38, 49)` before control returns. The audit derives these endpoints
  from both source movement commands and checks the native bytecode trace.
- Replaced overflowing 8-bit field palette multiplication with shared BGR555
  conversion using 16-bit arithmetic for field and actor palettes. Fixed
  actor horizontal flips to swap tile columns as well as pixel columns.
  Added Rust regressions for palette endpoints, whole-actor mirroring and
  the opening sequence. These regressions now pass with the local toolchain.

- Directly recomputed both ROM lengths, CRC32 and SHA-256; all match the
  requested identities.
- Added explicit Cargo package exclusions for `*.sfc`, `*.ips` and `Patch/`;
  the read-only audit asserts that these patterns remain in `Cargo.toml`.
- Checked the ROM header bytes and map-mode field in both images.
- Rechecked existing extracted-asset counts and read their extraction reports,
  provenance maps, event/sound reference notes, and the graphics preview script.
- Corrected the extraction index's build-path description after inspection:
  vanilla build-form files are under `vanilla/build/assets/`, while patched
  build-form files are under `twue-3.07/build/en/assets/`. The Rust prototype
  consumes decoded raw assets and does not depend on the asymmetric build tree.
- Read all five text files supplied in `Patch/` (compatibility notes, bug-fix
  compendium, item-name list, monster-name list and v3.07 readme).
- Read the upstream event-script README and source around the mapped event
  entries, the SPC transfer/driver source, and the audio extraction README.
- Inspected the pinned upstream `tools/extract_assets.py` at commit
  `813013276c952fdd27edcf7b8b86f17291542cbf`: its entry point recognizes the
  Japanese 1.0 and U.S. 1.0/1.1 CRCs, then constructs a HiROM `AssetExtractor`
  from the `rip_list` definitions. It has no recognized CRC or pointer layout
  for this 4 MiB patched image, so the existing patched exports were reused.
- `python tools/verify_rom_assets.py` passed: both ROM identities, the IPS
  identity and all 2,174 addressed IPS records, five patch-note copies, per-ROM
  asset counts, 12,626 canonical PC/CPU source-map pairs per ROM, and the
  239-file text/graphics/data diff totals. It now also checks that the first
  nine upstream field walk-frame entries occur exactly once at `C0:CE3A` in
  both ROMs and that the patched Terra/Soldier exports differ from clean.
- Reassembled target Terra/Soldier atlas pixels from local 4bpp exports and
  the target BGR555 palette. The atlas is a data-inspection image, not an
  emulator reference capture.
- Added `tools/extract_native_field.py` for a read-only, reproducible assembly
  of MapProp 21's patched field assets. It decodes the property record, loads
  the named exported layers/tilesets/palette/tile properties, and copies the
  four 0x2000-byte map graphics DMA windows from the patched ROM into a local
  VRAM bundle. `tools/render_native_field.py` renders a cropped asset preview;
  it is not an emulator capture.
- Expanded the assembler to ten opening/Narshe/mine map bundles. Explicit
  width/height pairs now come from `InitScrollClip` and MapProp bytes 23/24.
  The earlier assumption that every serialized layout has 64 source rows was
  incorrect and has been replaced. See [field-entrances.md](field-entrances.md).
- Extended `tools/verify_rom_assets.py` to check the MapProp-21 IDs and PC/CPU
  evidence, every bundled map/tileset/palette/tile-property file against its
  mapped export, the tileset interleave, metadata, and all four VRAM windows
  against the patched ROM. `python tools/verify_rom_assets.py` passes the
  complete ROM/IPS/asset/text/sprite/map audit.
- Replaced the fabricated field backdrop with decoded patched maps. The
  opening event source confirms cards on maps 23/20/244/18, the Biggs/Wedge
  exchange on map 18, then map 19 at `(38, 58)` followed by DLG_11 and 3,000
  gil. The Rust sequence now follows that map/dialogue order. The entry now
  schedules actor movement; original camera timing remains unverified; Magitek composition is now implemented.
- Added Rust unit coverage for cardinal steps, map edges, directional masks,
  impassable tiles, counter tiles, and compact-layout wrapping. The ROM audit
  checks a passable diagnostic coordinate on map 21 and compares every
  generated opening-map bundle to its patched source assets.
- Checked the event-script bytes for the opening sequence between `$CC:985B`
  and `$CC:9AD5`; both clean and patched exports match. The source-derived map
  and dialogue order is also asserted by the audit.
- Saved `captures/map-0021-asset-preview.png` from the decoded layers at that
  demo spawn. It is a generated asset preview, not a reference emulator frame;
  exposed backdrop rendering and exact layer composition remain unverified.
- Refreshed decoded map previews at the source event coordinates for maps 23,
  20 and 19. Camera positions use the 64×64 field ring; the images remain
  asset previews, not emulator captures.
- Generated map 18 and map 244 opening previews. Layout ID zero follows the
  loader's 0x4000-byte physical WRAM clear; bundles store logical zero grids
  using MapProp dimensions, rather than the `none.bin` placeholder export.
- Generated a grayscale atlas preview of exported `title_opening.en.4bpp`
  tiles. Title palette, tilemap, timed animation and original splash sequence
  remain unmapped; the Rust title scene is not claimed as a reference capture.
- Replaced the prototype's invented intro summary with the four opening lore
  cards from patched `DLG_6`–`DLG_9`, followed by the seven Biggs/Wedge
  dialogue lines from `DLG_1`–`DLG_5`. A verifier compares the runtime strings
  against those extracted target entries after removing layout/control codes.
  Event timing and presentation are still approximations.

## Not yet completed

- No compatible emulator is installed/available for reference screen, input,
  battle, save or audio captures. The map preview is composed from decoded
  assets, so no emulator/runtime comparison is claimed.
- The REA firmware signature scan is unavailable in this session because its
  provider requires Linux and a configured `REA_BINWALK_COMMAND`. This does not
  affect the ROM identity, source-map, byte-diff, or extracted-asset audits.
- The upstream ROM extraction scripts themselves are not present in the local
  reference subset; only their output, source maps, pinned provenance notes,
  and the local graphics-preview script are present. The checked-in extraction
  report records that `tools/extract_assets.py`, the Makefile, maps and pinned
  `romtools` submodule were inspected before the clean-ROM extraction and that
  the vanilla-only extractor was not run on the expanded ROM.
- No full source-to-target patched disassembly, event opcode coverage map,
  complete damage-formula comparison, SRAM map or SPC playback validation exists yet.

## Battle table and arithmetic addition

The release build and 73 Rust tests pass. `battle-data-validation.json`
records 1024 event-group selections, 576 formation decodes (1364 occupied
monster slots), 16384 CalcSpeed cases against bounded execution of the target
65816 routine, and 512 clock/gauge frames against an independent model.
`battle-extraction-validation.json` records 4992 numeric source-field checks
and every differing table byte. These do not validate complete battles.
Source addresses, format details and unimplemented paths are in [battle-data.md](battle-data.md).

## Character initialization addition

`characters-validation.json` records 6336 property/level cases, three opening
characters and six actual Game/save rows. The target changes 86 MP growth
bytes; the target differs from the Japanese source at level 23 as detailed in
[characters.md](characters.md). Native Terra now starts at level 3, HP 63,
MP 27 and EXP 96. Version 9 saves add checksummed initialized guest records;
older versions remain readable with reconstructed opening guests. The native
stats-menu BMP was visually inspected for the new level/EXP/next values.
These are static-source/table and native save checks, not an original battle
runtime comparison or completion of the party/battle systems.

## Equipment calculation addition

`equipment-validation.json` records 3080 equipment cases against bounded
execution of the target C2/0E77 routine (3,870,629 instructions), comparing
all 64 result bytes and HP/MP boost flags. The target item table changes one
byte: Mythril Rod power 60 to 61. The three native opening-party rows match
the routine. Native Equipment and Status pages now display target gear names
and derived stats; both BMPs were visually inspected. Rust tests cover field
Sprint Shoes membership and its combination with Cancel dash/save restoration.
These checks do not implement combat, equipment editing or all relic behavior.
See [equipment.md](equipment.md).

## Patched damage arithmetic addition

`damage-validation.json` records 8768 base cases, 4096 modifier cases, 12
opening Magitek base cases and 3072 controlled opening modifier contexts.
Base/modifier checks execute the actual target instructions in a bounded
65816 checker. The target rewrites C2/2B69 and C2/0D4A rather than preserving
the vanilla formulas/call graph. Detailed addresses, rounding, changed attack
bytes and unimplemented combat stages are in [damage.md](damage.md).
The equipment checker is rerun after changes to shared checker instructions.

## Battle RNG and initial gauge addition

`battle-rng-validation.json` records 65,536 bounded RNG inputs, 2,048 bit
selections, 2,560 sequence rows, 1,024 initial gauge setups and 256 Guard AI
choices. Actual target RNG instructions execute in the checker; no host PRNG
or injected random return is used for these comparisons. Native battle code
now implements these primitives. Live battle initialization/call ordering and
AI/action execution remain incomplete; see [battle-rng.md](battle-rng.md).

## Native scheduling addition

`battle-scheduler-validation.json` records 1024 ATB states, 512 dispatch
states, 1792 command-delay cases, 1536 ring operations and 216,916 target
instructions. Actor flags/counters, queue pointers/contents, clock and callback
order match bounded target execution. Input/status/escape/graphics callbacks
and battle-end handling are explicitly bounded, not claimed as implemented
subsystems. The live battle remains incomplete; see [battle-scheduler.md](battle-scheduler.md).

## Command storage and input addition

`battle-commands-validation.json` records 100 creations, 64 removals,
140 allocations, 20,480 MP-cost cases and 256 supported input cases. Actual
target instructions verify storage, links, targets, cached costs and scheduler
queue/flag integration. Patched Item/Slots normalization remains unsupported.
No live battle execution is claimed; see [battle-commands.md](battle-commands.md).

## Advance preparation addition

`battle-advance-validation.json` records 512 cases and 27,023 target
instructions, with explicit AI, targeting and presentation callbacks. Native
input, pending commands, wait duration and normal-action enqueue are connected
in an integration test. Live combat remains incomplete; see
[battle-advance.md](battle-advance.md).

## Queued-action initialization addition

`action-init-validation.json` records 8192 cases and 209,373 target
instructions, including the actual patched Imp helper and RNG instructions.
Working/original command distinction, cached costs, targets, summon/changed
masks and unchanged queue payloads match. Full action execution is not
claimed; see [action-initialization.md](action-initialization.md).

See [elemental-damage.md](elemental-damage.md) for C2/0B83..0C9D elemental resolution, counter arithmetic, comparison evidence and remaining limits.

See [damage-application.md](damage-application.md) for native C2/1323 HP, C2/1350 MP, net calculation and pending-death behavior.

See [battle-accuracy.md](battle-accuracy.md) for target Evade fix and ordinary accuracy at C2/22FB..239B, including RNG and Image handling.

See [battle-hit-gates.md](battle-hit-gates.md) for C2/220D hit gates, reflection, forced blocks and level/stamina modes.

See [battle-hit.md](battle-hit.md) for the joined C2/220D..23EC native CheckHit, miss effects and complete bounded instruction comparison.

See [battle-equipment.md](battle-equipment.md) for C2/2883 equipment-to-battle projection and C2/2861 working evasion conversion.

See [battle-actor-stats.md](battle-actor-stats.md) for C2/2CE4 monster working stats and C2/2650..26C8 immunity normalization, with target comparisons.

See [battle-rage.md](battle-rage.md) for complete native LoadRageProp at C2/2DC1..2E39 and target comparisons of monster/Rage trait merging.

See [battle-ai-init.md](battle-ai-init.md) for native AI initialization, decoded script boundaries and content changes separated from relocation.

See [battle-ai-turn.md](battle-ai-turn.md) for native normal AI attack emission, command mapping, wait/resume handling and comparison scope.

See [battle-targets.md](battle-targets.md) for native C2/587E target selection, actor eligibility, side/pincer restrictions and combined Guard AI comparisons.

See [battle-target-init.md](battle-target-init.md) for native InitTarget, item/Throw/Tools paths, working attack-property loading and exhaustive byte-pair comparisons.

See [battle-resources.md](battle-resources.md) for C2/3201 casting restrictions and C2/32B1 cached MP payment, failure effects and comparison evidence.

See [battle-attack-preparation.md](battle-attack-preparation.md) for C2/32EC monster special properties, common damage setup, special animation data and target comparisons.

See [battle-status-requests.md](battle-status-requests.md) for C2/4406 status requests, pending/current separation, immunity failure and target instruction comparisons.

See [battle-status-transition.md](battle-status-transition.md) for C2/450D status preparation, Overcast, retaliation snapshots and bounded post-handler storage/reset helpers.

See [battle-status-update.md](battle-status-update.md) for complete native UpdateStatus, all patched status handlers, full target instruction comparisons and clean baseline differences.

See [battle-cleanup.md](battle-cleanup.md) for native UpdateDead living masks/counts, RemoveAllActions and connected death/cleanup comparisons.

See [battle-readiness.md](battle-readiness.md) for pending/player readiness, ATB disabling, cancellation, source queue mutation and menu gates.

See [battle-menu-state.md](battle-menu-state.md) for native command/spell eligibility refresh, memory layout and target comparisons.

See [battle-after-effects.md](battle-after-effects.md) for Condemned, Quetzalli and near-fatal relic handlers and [battle-morph.md](battle-morph.md) for Morph/Revert transitions and target comparisons.
