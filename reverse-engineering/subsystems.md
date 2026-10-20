# Subsystem notes

## Event engine and field maps

**Observed:** The patched text export's `DLG_1`–`DLG_9` entries contain the
four lore cards and opening Biggs/Wedge conversation. The native slice uses
those target strings, and the verifier compares them against the patched
export. The lore cards and initial exchange manually follow source-confirmed order.
The map-19 entry now executes serialized target bytecode through a bounded
native VM; see [event-vm.md](event-vm.md).

**Observed:** `event_main.asm::_cc985b` loads maps 23, 20, 244 and 18 for
DLG_6–DLG_9, then runs DLG_1–DLG_5 on map 18. `_cc9a4f` loads map 19 at
`(38, 58)`, grants 3,000 gil and shows DLG_11. The relevant target event bytes
match the clean export. The Rust intro now uses those map and dialogue orders;
actor state and parallel scripts are now executed from bytes with fixed-point
movement. Magitek rendering now uses target tables/graphics. Fades and original
DMA/PPU animation timing remain unimplemented or unverified.

**Derived:** In `_cc9a4f`, Terra starts at `(38, 58)`, moves north eight
tiles before DLG_11, then north one tile after it. Her movement endpoints
are `(38, 50)` and `(38, 49)` respectively. The Rust slice applies these
endpoints from the bytecode, along with Wedge/Vicks endpoints and visibility.
The live scheduler animates all three actors concurrently using source-derived
speed/wait rules. Original timing, collision and Magitek presentation are not
validated.
The earlier provisional control-return Y of 57 omitted the first eight-tile
move and has been corrected. The audit now reads both Terra movement scripts.

**Observed:** MapProp record 21 at `$ED:91B5` selects Narshe North Exterior
BG1/BG2/BG3 layouts 113/114/190, graphics IDs 10/30/31/32 and BG3 graphics 3,
tilesets 19/20, palette 12 and tile-property table 8. The field renderer now
uses the target's extracted tilemaps, map tilesets, graphics, palette and
directional passability data. Its graphics windows follow the upstream
`LoadMapGfx` VRAM destinations, and the map tileset export is interleaved as
`LoadTileset` does. `tools/verify_rom_assets.py` compares the generated bundle
back to patched-ROM bytes and mapped exports.

**Observed:** `InitScrollClip` obtains six dimension masks from MapProp bytes
23 and 24. The 1,024-byte layouts on maps 18/23 are 32×32, correcting the earlier
16×64 inference. Mines 41–43 are 128×64. Both renderers now index with explicit
width and height masks. `LoadMapTiles` clears 0x4000 physical WRAM bytes for a
zero layout ID; the runtime stores the active logical zero array. Source and
formats are detailed in [field-entrances.md](field-entrances.md). Per-map camera
clipping, parallax and original PPU composition still need runtime comparison.

**Implemented:** Player control now advances in source-derived pixel steps,
with target Cancel-dash speed, locked velocity and aligned entrance checks.
Partial movement survives local save/load. See [player-movement.md](player-movement.md).

**Implemented:** Direct sub-map point and line exits are decoded from the
patched records. Native input traverses 19→39→41→42→41 in 121 tile steps.
The full two-ROM table audit checks 512 map ranges per table and confirms no
entrance-table changes. The native route also passes four raw-ROM transition
checks and five independent background comparisons. This route explicitly disables story triggers for geometry validation. Live
play runs the covered Narshe scripts and pauses at the first unimplemented
battle; the traversal is not original story progression.

**Observed:** The upstream event source identifies the event script as a large
block and implements commands through assembler macros. The README groups them
as character/inventory, cutscene, control, dialogue, map, battle/menu and object
commands. It describes parallel object scripts and flow control. The local ROM
exports retain the raw script slice at PC `0x0A0000–0x0CE5FF` and compare 23
changed runs (249 bytes) in that slice.

**Inferred:** The main game is driven by a field loop that updates map layers,
actors, triggers and script state, then cooperates with battle/menu subsystems.
The exact dispatch/state layout must be taken from the 65816 source and hardware
behavior when implementing each subsystem.

**Implemented:** Raw tile triggers now dispatch target scripts. The covered
Narshe backtracking scene completes; the first guards reach a typed battle
request and pause. Normal NPC property decoding, calls/repeated calls and
OR/AND switch conditions are implemented. See [field-events.md](field-events.md).

**Unknown:** Most event bytecode, the full event trigger graph, and complete
map collision/exit semantics remain unmapped. The `(26, 50)` coordinate on map
21 is a diagnostic sample only. General NPCs, object scripts and actor movement,
startup events, world/parent map transitions and bridge z-state remain open.
The modified event runs are not
attributed to named patch fixes yet.

## Battle

**Queued-action initialization:** Native code now loads working payloads,
preserves cached costs, applies the changed-target gate and follows the target's
Imp's Call helper. Command consumption, execution and subsequent restrictions
remain open; see [action-initialization.md](action-initialization.md).

**Advance preparation subset:** Native state now connects scheduler work with
pending-list preparation, delay accumulation, updated targets and deferred
actor flags. AI/target/presentation are explicit hooks; autonomous character
selection and Jump remain unsupported. See [battle-advance.md](battle-advance.md).

**Command storage subset:** A native shared pool stores normal, retaliation and
immediate payloads, target masks and cached MP costs. Supported player-input
normalization feeds scheduling and storage. Live menu/action execution and
remaining normalizers are open; see [battle-commands.md](battle-commands.md).

**Scheduling subset:** Native state now combines the ATB clock, actor counters,
advance/action/retaliation queues, dispatch precedence and command-delay
arithmetic. Status, escape, input and graphics callbacks remain explicit, and
command payload/action execution is unimplemented. The live story remains
paused at the first battle request. See [battle-scheduler.md](battle-scheduler.md).

**Implemented arithmetic:** The patched C2/2B69 base-damage routine and
C2/0C9E modifiers now have native translations, checked against target
instruction execution. The patch rewrites the vanilla base routine and folds
relic damage effects into it. Attack properties retain the two target changes.
Hit/element resolution, action execution and live combat remain incomplete.
See [damage.md](damage.md).

**Observed:** The target changes formation records, battle graphics and an
event battle-group table. The 384 base MonsterProp records at CF/0000 are
unchanged. Target decoding and bounded ATB arithmetic are implemented and
audited separately; see [battle-data.md](battle-data.md). The patch
notes list fixes for damage overflow, evade calculation, Rippler status handling,
counter behavior, row/pincer state, jump/launcher targets, Magitek commands,
Rage restoration and other cases. These are documentation claims until each
changed routine/data entry is mapped and/or compared at runtime.

**Inferred:** The native battle implementation needs a command queue, ATB timer,
actor/enemy states, target selection, status/element/resistance rules, damage
calculation, animation/message sequencing, formation data, and escape/victory
resolution. A simple turn-based substitute will not match FF6 and is not a
completion criterion.

**Unknown:** A complete patched battle code/data map and emulator reference
captures have not been produced.

## Menus, party, progression and shops

**Implemented subset:** Character property/growth tables now initialize the
opening roster, with Terra level 3/HP 63/MP 27/EXP 96 and separate guest
properties 32/33 in slots 14/15. The target changes 86 MP-growth bytes.
Native menu EXP-to-next uses the target table; local v9 saves retain the
initialized roster. This does not implement equipment-derived battle stats,
natural magic, later property events or full progression. See [characters.md](characters.md).

**Equipment subset:** Target ItemProp records now drive native UpdateEquip
aggregation and read-only equipment/status pages. Enabled members of the active
party supply field effects, including Sprint Shoes. Equipment editing, final
battle projection and relic command behavior remain open; see [equipment.md](equipment.md).

**Observed:** The patch changes names, descriptions and some menu-facing data.
The base patch expands item descriptions and ability names; the Bug Fixes
variant includes the elemental display and MP menu fixes documented in the
readme/compendium. Optional Add-On menu changes (Blitz command labels, Y-based
relic equip, Rage checklist and Colosseum reward display) are not assumed
present in this target unless the ROM diff proves them.

**Unknown:** Menu state machines, every equipment/stat formula, shop inventory,
character growth and progression flags are not yet ported or runtime-validated.

## Save data

**Implemented prototype behavior:** Version 8 stores modeled event state
through direct VM/actor snapshots, including call/object cursors, membership,
waits, flags and battle pauses. Field motion and both logical clocks are also
retained. Diagnostic histories are not saved. Versions 2 through 7 use legacy
migration paths. Snapshot payloads carry a checksum and bounds checks. These
local saves are not cartridge SRAM; full battle/party/audio state is unported.

**Observed:** The cartridge header indicates SRAM is present. The current local
reference set does not yet map the complete SRAM layout, checksum scheme,
slot format or migration behavior.

**Unknown:** Exact fields, character encoding, event-bit packing, play timer,
save/load edge cases and reset semantics.

## Sound

**Observed:** `sound_main.asm` has 65816 entry points for initialization,
execution, song transfers, script transfers and sound effects. `InitSound`
copies blocks to SPC RAM; its destination table starts at `$0200`. `SPCCode` is
embedded in the ROM driver source and is for SPC700 execution, not the 65816.
The 85 exported songs, their normalized MML and SPC data compare identical
between the clean and target images.

**Implementation choice:** Keep sound inside the native game and port the
FF6-specific sound driver behavior, SPC700 instructions and DSP sample path
into Rust. Stream generated 32 kHz stereo PCM through WinMM `waveOut` on the
current Windows backend. Use the local `mfvitools` MML/SPC/BRR exports as
reference inputs and validation material. This is an architecture decision,
not a claim of implemented or validated playback.

**Unknown:** Accurate native playback, the full sequence grammar, DSP timing,
channel effects and SFX scheduling have not been implemented/compared.

## Other special systems

The world map has separate tilemaps, 3D/rotation-related assets and zone-based
encounter tables. The patch documents the Border Crossing fix, which updates
overworld encounter-zone state per step instead of only at redraw. Vehicle
movement, the two-world transition, scenario party selection, the Opera,
multi-party dungeon, colosseum, Veldt, ending and cutscene effects all require
dedicated subsystem work. Each is currently open.

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
