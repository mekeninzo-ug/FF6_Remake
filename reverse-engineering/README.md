# FF6 reverse-engineering record

This directory is the working evidence ledger for a native Rust reconstruction
of Final Fantasy III (USA), targeting Ted Woolsey Uncensored Edition v3.07 +
Bug Fixes. The patched ROM is the authority for target behavior; the clean ROM
is a baseline only.

## Findings

| Question | Finding | Confidence / evidence |
|---|---|---|
| Input identity | Both supplied ROMs match their expected size, CRC32 and SHA-256. | Confirmed directly; see [rom-identity.md](rom-identity.md). |
| Cartridge map | Header mode byte `$31` identifies FastROM HiROM. The 3 MiB clean image occupies canonical banks `$C0–$EF`; the 4 MiB target adds `$F0–$FF`. | High, derived from each ROM header and the extracted source map. Check mirrors before interpreting low-bank addresses. |
| Asset coverage | Existing exports include 30 text assets, 1,430 graphics assets, 535 data assets and 71 BRR samples per ROM. | Confirmed by the extraction report and filesystem counts. |
| Vanilla extraction | Existing vanilla exports were made from the verified clean image using the upstream U.S. maps. The upstream `make rip` target was avoided because its setup prerequisite forces a remote submodule update. | Existing extraction record; this turn rechecked the input identities and exports. |
| Patched extraction | Existing patched exports read directly from the 4 MiB target using adjusted pointers; the upstream vanilla extractor does not recognize that image. | Confirmed by recorded extraction procedure; do not substitute clean assets for unmapped target bytes. |
| Narshe North Exterior map | Patched MapProp record 21 selects tilemaps 113/114/190, map graphics 10/30/31/32 and BG3 graphics 3, tilesets 19/20, palette 12, tile properties 8. | High for IDs/format from the target record and upstream field loader; map runtime bundle is compared byte-for-byte by the audit. Map 21 is used only as a diagnostic movement sample. |
| Opening field sequence | The event source loads maps 23, 20, 244 and 18 for DLG_6–DLG_9, runs DLG_1–DLG_5 on map 18, then loads map 19 at `(38, 58)` for DLG_11 and gives 3,000 gil. | High for command order and map coordinates; the relevant serialized event bytes match clean and patched exports. Native bytecode endpoints are independently checked; original presentation timing is not runtime-captured. |
| Event engine | The engine is a bytecode interpreter with parallel object scripts, control flow, and game-specific commands. Only a small portion of the upstream event block is disassembled. | High for command model; low for undocumented scripts. See [subsystems.md](subsystems.md). |
| SPC audio | The 65816 transfers an SPC program and sequence/sample data to the SPC700; music and SFX exports are identical between the two ROMs. | High for transfer path and byte identity; audible timing remains unvalidated in this project. |
| Full-game behavior | Not yet reconstructed. ROM differences identify changed bytes/assets, but do not independently prove each named bug fix. | Open; see [version-comparison.md](version-comparison.md). |

## Evidence files

- [action-initialization.md](action-initialization.md): working command payloads, status retargeting and the target's Imp's Call helper.
- [battle-advance.md](battle-advance.md): advance preparation, deferred actors, command waits and explicit target/AI boundaries.
- [battle-commands.md](battle-commands.md): shared command pool, cached MP cost and supported battle-input normalization.
- [battle-scheduler.md](battle-scheduler.md): native queue ordering, ATB coordination, command delays and explicit callback boundaries.
- [battle-rng.md](battle-rng.md): table RNG, initial gauges, target AI tables and Guard attack selection.
- [damage.md](damage.md): patched base damage, target modifiers, attack records and bounded instruction comparisons.
- [equipment.md](equipment.md): target items, native equipment arithmetic, status/gear pages and Sprint Shoes integration.
- [characters.md](characters.md): property IDs, target MP growth, opening stats and character snapshots.
- [battle-data.md](battle-data.md): target formations, enemy properties, byte differences and bounded ATB arithmetic.
- [rom-identity.md](rom-identity.md): fingerprints, header, map formula and bank ranges.
- [code-data-map.md](code-data-map.md): annotated code/data locations in PC and CPU coordinates.
- [formats.md](formats.md): graphics, palettes, tilemaps, text and sound formats.
- [field-entrances.md](field-entrances.md): raw exit formats, table addresses,
  width/height masks and the validated native field route.
- [opening-timeline-validation.json](opening-timeline-validation.json): live scheduled commands, per-frame movement and actual game-loop comparisons.
- [player-movement.md](player-movement.md): continuous field motion, mapped Cancel-dash/Sliding Dash code and version-7 saves.
- [player-movement-validation.json](player-movement-validation.json): raw 65816 hook checks, native movement states and full-frame pixel comparisons.
- [magitek-sprites.md](magitek-sprites.md): vehicle/rider formation, DMA mapping and palette evidence.
- [magitek-validation.json](magitek-validation.json): 32 native configurations compared to independent raw-ROM pixels.
- [field-events.md](field-events.md): trigger/NPC tables, covered Narshe scenes, calls/conditions and direct VM saves.
- [field-events-validation.json](field-events-validation.json): 135 source-encoded bytes, 84 native command spans and actual walk-trigger captures.
- [npc-property-differences.json](npc-property-differences.json): six patched bytes in four NPC records.
- [event-vm.md](event-vm.md): bounded native event/object VM, supported commands and limits.
- [opening-event-validation.json](opening-event-validation.json): source encoding, ROM identity and 80-command structural decoder comparison.
- [subsystems.md](subsystems.md): event, field, battle, menus, save and sound notes.
- [version-comparison.md](version-comparison.md): clean-to-target changes and patch-note reconciliation.
- [validation.md](validation.md): checks performed, limitations and open questions.
- [native-render-comparison.json](native-render-comparison.json): executable and
  frame digests, opening-state checks and independent asset-renderer comparison.
- [native-window-smoke.json](native-window-smoke.json): actual Win32/GDI
  initialization, presentation and message-loop shutdown result.
- [package-audit.json](package-audit.json): Cargo's actual package file list,
  manifest digest and local-input exclusion result.
- [entrances-audit.json](entrances-audit.json): full point/line table comparison
  and pointer/provenance checks in both ROMs.
- [field-traversal-validation.json](field-traversal-validation.json): 121 input
  steps, four transitions checked against ROM records, and five native frame comparisons.
- `captures/native-opening/`: eight frames and CSV state trace emitted by the
  compiled Rust game; these are native implementation captures.
- `../tools/extract_native_field.py`: read-only target map assembler for the
  maps 18, 19, 20, 21, 23, 39, 41, 42, 43 and 244.
- `../tools/render_native_field.py`: diagnostic renderer for the generated field bundle; not an emulator reference capture.
- [captures/map-0021-asset-preview.png](captures/map-0021-asset-preview.png): generated asset preview at the selected demo spawn; the exposed black backdrop and layer composition have not been compared to an emulator.
- [captures/map-0023-opening-preview.png](captures/map-0023-opening-preview.png), [captures/map-0020-opening-preview.png](captures/map-0020-opening-preview.png), [captures/map-0018-opening-preview.png](captures/map-0018-opening-preview.png), [captures/map-0244-opening-preview.png](captures/map-0244-opening-preview.png), [captures/map-0019-opening-preview.png](captures/map-0019-opening-preview.png): decoded asset previews, not emulator reference frames.
- [captures/title-opening-tiles.png](captures/title-opening-tiles.png): grayscale tile atlas from the title/opening graphics export; no title screen palette or tilemap composition is claimed.

Layer geometry now follows `scroll.asm::InitScrollClip`: maps 18 and 23 use
32×32 layouts, and mine maps 41–43 use 128×64 layouts. The earlier 16×64
interpretation of the 1,024-byte compact assets was incorrect. Both renderers
now use the explicit MapProp width and height masks. A zero layout ID clears
0x4000 physical WRAM bytes; runtime bundles store a zero array of the active
logical dimensions. See [field-entrances.md](field-entrances.md) for source evidence.

## Evidence classes

- **Observed**: bytes, file identities, decoded assets, labels or source text directly present in the local inputs/references.
- **Derived**: a deterministic transformation, such as a HiROM file-offset mapping or a decoded BGR555 value.
- **Inferred**: an interpretation of those observations, including behavior not confirmed by a runtime capture.
- **Unknown**: not established by available sources. Unknown findings are not treated as confirmed features.

The local disassembly and `mfvitools` source are preserved with their upstream
license notices under `extracted/reference/`. The two sibling remake directories
were empty when inspected, so there were no existing Rust conventions to reuse.

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
