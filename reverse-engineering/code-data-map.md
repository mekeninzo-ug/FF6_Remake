# Annotated code and data map

Canonical CPU addresses below use full HiROM banks `$C0–$FF`. PC-to-CPU
conversion is defined in [rom-identity.md](rom-identity.md). The upstream
reference is the vanilla disassembly pinned in
`extracted/reference/README.md`; it is not a disassembly of the patched ROM.

## 65816 driver code

| Area | CPU address / PC offset | Evidence | Interpretation / confidence |
|---|---|---|---|
| Sound-driver jump table | `$C5:0000–$C5:000F` / PC `0x050000–0x05000F` | `ff6-disassembly-sound/sound_main.asm`, labels `InitSound_ext`, `ExecSound_ext`, `TfrSong_ext`, `TfrSongScript_ext`; source says pointers use `+$C50000`. | 65816 entry stubs. High. |
| Sound driver | `$C5:0034` and following / PC `0x050034` onward | `sound_main.asm`, `InitSound` at `@0034`, followed by `ExecSound`, `PlaySong`, `TfrSong`, `TfrSamples`, `TfrSongScript`. | 65816 side of the SNES-to-SPC protocol. Routine ends must follow the source/linker map. High for labels. |
| Embedded SPC program | `$C5:070E` / PC `0x05070E` source label `SPCCode` | `sound_main.asm`, `; c5/070e`, transfer table destination `$0200`. | Bytes are loaded into SPC RAM and then execute as SPC700 code; the ROM location is not its run address. High. |
| Song count/data boundary | `$C5:3C5E` / PC `0x053C5E` source label `NumSongs` | `sound_main.asm`, `; c5/3c5e`; `sample_brr.asm` begins at `$C5:3C5F`. | Named data boundary, not code. High. |

The pinned upstream `src/cutscene/title.asm::TitleMain` (`@500C`) runs
`InitTitle`, the English `SplashLoop`, and then `TitleLoop`; `InitTitle`
(`@507F`) initializes hardware, starts opening-theme track `$02`, and loads
title graphics. These source-local offsets have not been resolved to a canonical
CPU bank and PC in this project. The Rust title screen remains a visual
placeholder; the original timed tasks and title-state table are not ported.
See the upstream [title source](https://github.com/everything8215/ff6/blob/813013276c952fdd27edcf7b8b86f17291542cbf/src/cutscene/title.asm).

## Event bytecode addresses (not 65816 instructions)

The `$CA` labels below identify serialized event-script data, interpreted by
the field engine. They are stored at HiROM CPU addresses and PC offsets, but
the macro lines are not executed as 65816 instructions.

| Event label | CPU address / PC offset | Evidence | Meaning / confidence |
|---|---|---|---|
| Event-script block | `$CA:0000–$CC:E5FF` / `0x0A0000–0x0CE5FF` | Local event exports and `event_main.asm` fixed block `$02E600`. | Mapped script slice; not the complete event-engine code. High for range. |
| `NoEvent` | `$CA:0000` / `0x0A0000` | `event_main.asm`, comment `; ca/0000`. | Event commands set script mode and terminate with no-event. High. |
| `WaitDlg` | `$CA:0001` / `0x0A0001` | `event_main.asm`, comment `; ca/0001`. | Wait for dialogue then return. High. |
| `GameStart_ext` | `$CA:0003` / `0x0A0003` | `event_main.asm`, comment `; ca/0003`. | Calls `GameStart`, then returns; only the wrapper is at this location. High. |
| Treasure event handlers | `$CA:0008–$CA:0017` / `0x0A0008–0x0A0017` | Labels `TreasureItem`, `TreasureMagic`, `TreasureGil`, `TreasureEmpty`. | Dialogue bytecode plus return. High. |
| Random battle handler | `$CA:0018` / `0x0A0018` | Label `RandBattle`. | Object scroll commands, random battle, defeat handling and party refresh. High for this sequence. |
| Opening cinematic event | `$CC:985B` / `0x0C985B` | `event_main.asm::_cc985b`; source loads maps 23/20/244/18, shows DLG_6–DLG_9 then DLG_1–DLG_5. Serialized event bytes match clean and patched exports through `$CC:9A4F`. | High for command order; camera and actor presentation are not ported. |
| Opening field entry | `$CC:9A4F` / `0x0C9A4F` | `event_main.asm::_cc9a4f`; loads map 19 at `(38,58)`, sets the party, grants 3,000 gil and displays DLG_11. Clean/target script bytes match through `$CC:9AD5`. | High for script content and coordinates; structural decoder covers all 134 bytes / 80 spans; live scheduler executes 78 after one superseded object block, see [event-vm.md](event-vm.md). |
| Event triggers / NPC properties | Use linker/source map before assigning PC offsets | `event_trigger.asm`, `npc_prop.asm`, `map_init_event.asm`. | Separate tables; source-file order does not establish ROM location. Medium. |

The 65816 code banks outside the event and sound references have not yet been
fully indexed. Labels and raw ranges must be added only after reading the
linker configuration or matching the bytes to the source. The patched ROM
contains binary edits throughout the image; no global source-level patch map
has been completed.

## Data

| Asset group | Representative target location | Export path | Confidence / limitation |
|---|---|---|---|
| Battle dialogue | `$D0:0000` / PC `0x100000` | `assets/text/battle_dlg.en.txt` | Source map labels and byte spans are recorded. Patched pointers differ from vanilla. |
| General English strings | From `source_asset_map.csv` per label | `assets/text/*.en.txt` | Character/control codes are explicit; escaped control codes are not plain English. |
| Item descriptions | PC `0x310200` pointer table; strings begin at `0x310400` | `expanded/decoded_text_candidates/` and raw expansion banks | Confirmed target structure by extraction report; individual heuristic candidates retain status flags. |
| Graphics | Indexed per file in `source_asset_map.csv` | `assets/gfx/` | 1,430 extracted files per version. Format metadata is in the map. |
| Field map tilemaps | 350 sub-tilemap files | `assets/data/field/sub_tilemap/` | Some names are upstream labels; numeric files remain unknown map IDs. |
| Battle backgrounds | 75 upstream pointer entries | `assets/gfx/battle_bg_tiles/`, `provenance/unclassified_pointer_data/` | 48 spans validate as LZSS, six do not use that decoder, and 21 point into invalid/zero-filled spans in both images. |
| Music sequence data | 85 tracks | `audio/music/track_index.csv`, `.mml`, `.spc` | Sequence bytes and normalized exports compare identical between ROMs; generated SPC playback is not validated. |
| BRR sample/SFX data | 71 static/SFX BRR samples; 481 valid channel sequences | `assets/sound/`, `audio/sfx/` | Byte-identical between ROMs according to comparison report. |

The complete per-asset table is `extracted/{vanilla,twue-3.07}/provenance/source_asset_map.csv`.

## Field sprite frames

| Table | CPU address / PC offset | Evidence | Interpretation / confidence |
|---|---|---|---|
| `MapSpriteTileOffsets` walk-frame prefix | `$C0:CE3A–$C0:CEA5` / `0x00CE3A–0x00CEA5` | Upstream `src/field/sprite_data.asm`, entries `$00–$08`, names `WALKING_DOWN_1..3`, `WALKING_UP_1..3`, and `WALKING_LEFT_1..3`. The encoded 108-byte prefix occurs exactly once at that offset in both supplied ROMs. | Six little-endian byte offsets make one 16 × 24 frame; divide each by 32 to index a decompressed 4bpp sprite. Right-facing walking reuses left-facing frames with horizontal flip. High for the nine entries; later table rows are not yet binary-verified. |
| Terra / Soldier graphics | Per-file PC/CPU address in each `source_asset_map.csv` | Target exports `assets/gfx/map_sprite_gfx/{terra,soldier}.4bpp` and `assets/gfx/map_sprite_pal/terra.pal`; the target and clean Terra/Soldier graphics exports differ. | Raw 4bpp tiles and BGR555 palette are decoded by the runtime. The exports do not include the full object animation/palette/priority state. High for file format and target selection; medium for current runtime use. |

The prototype uses these mapped walk frames for Terra. NPC actor identity, real
map object definitions and turn timing are still provisional. The exact source
table can be inspected in the pinned upstream [field sprite data](https://github.com/everything8215/ff6/blob/813013276c952fdd27edcf7b8b86f17291542cbf/src/field/sprite_data.asm).

## First field map: Narshe North Exterior

The 65816 `LoadMapProp` routine multiplies the current map index by 33 and copies
that record from `MapProp` at `$ED:8F00` (PC `0x2D8F00`). For map index 21, the
record is `$ED:91B5` / PC `0x2D91B5`. Its packed fields decode to BG1/BG2/BG3
sub-tilemap IDs 113/114/190, graphics IDs 10/30/31/32 plus BG3 graphics ID 3,
map tilesets 19/20, palette 12, and tile-property table 8. The values were
recomputed from the patched MapProp record and are checked against the local
runtime bundle by `tools/verify_rom_assets.py`.

| Map asset | CPU address / PC offset | Evidence / use |
|---|---|---|
| MapProp base and record 21 | `$ED:8F00` / `0x2D8F00`; `$ED:91B5` / `0x2D91B5` | `map.asm::LoadMapProp`; 33 bytes per map; patched target record selects the IDs above. High. |
| BG1 sub-tilemap 113 | `$DB:C786` / `0x1BC786` | `narshe_north_ext_bg1.bin`; 4,096-byte 64 × 64 metatile-ID grid after extraction. High. |
| BG2 sub-tilemap 114 | `$DB:D08D` / `0x1BD08D` | `narshe_north_ext_bg2.bin`; same grid dimensions. High. |
| BG3 sub-tilemap 190 | `$DC:B651` / `0x1CB651` | `narshe_north_ext_bg3.bin`; same grid dimensions. High. |
| BG1/BG2 map tilesets 19/20 | `$DE:755F` / `0x1E755F`; `$DE:7C3A` / `0x1E7C3A` | English decompressed exports `narshe_ext_bg1.en.bin` and `narshe_ext_bg2.en.bin`; the runtime copies alternating bytes from decompression work buffers into `$7F:C000` and `$7F:C800`. High for transformation; source IDs are upstream table labels. |
| BG1/BG2 graphics 10/30/31/32 | `$E0:8460` / `0x208460`; `$E2:3000` / `0x223000`; `$E2:4000` / `0x224000`; `$E2:4F00` / `0x224F00` | Four 0x2000-byte DMA source windows. VRAM word destinations are `$0000`, `$1000`, `$1800`, `$2000`; the later windows overlap. High for source IDs and loader destinations. |
| BG3 graphics 3 | `$E6:9895` / `0x269895` | `narshe_steam.2bpp`; output begins with a 0x40-byte formation table, then 0x1000 bytes copied to VRAM word address `$3000`. High. |
| Map palette 12 | `$ED:D080` / `0x2DD080` | `narshe_ext.pal`, 128 little-endian BGR555 colors. High. |
| Tile properties 8 | `$D9:AFCA` / `0x19AFCA` | `narshe_ext.bin`, 512 decompressed bytes. The player code indexes tile flags at `$7600` and directional flags at `$7700`. High for addresses and use; bridge/door interactions remain incomplete. |

The Rust field renderer draws this map bundle with patched Terra walk frames
and uses its tile-property/direction bytes for cardinal movement. Map 21 at
`(26, 50)` remains a diagnostic sample. The source-backed opening entry is
map 19 at `(38, 58)` after the prologue maps 23, 20, 244 and 18. The four
opening map layouts are assembled independently. `InitScrollClip` proves that
maps 18/23 are 32×32 and mines 41–43 are 128×64, correcting the earlier 16×64
compact-layout inference. Both dimensions are explicit in runtime metadata.
Layout ID zero clears 0x4000 physical WRAM bytes; the runtime stores the active
logical zero grid. Entrance table addresses, record formats, selected record
CPU/PC pairs and native traversal evidence are in [field-entrances.md](field-entrances.md).
Map scroll/parallax timing, exact PPU layer composition, animated map tiles,
object/NPC placement and general field triggers remain open.

## Field Magitek sprite data

| Data | CPU address / file offset | Evidence and confidence |
|---|---|---|
| Four 16-word Magitek formation tables | `$C0:5EB6` / `0x005EB6`, 128 bytes | Exact, unique source-table match in both ROMs. Labels `MagitekTopLeftTiles`, `MagitekTopRightTiles`, `MagitekBtmLeftTiles`, `MagitekBtmRightTiles`, 32-byte stride. |
| Map sprite frame offsets | `$C0:CE3A` / `0x00CE3A`, 696 bytes | All 58 six-word source frames match both ROMs. |
| Vehicle planar graphics | `$D8:3000` / `0x183000`, 8192-byte export | Export equals target bytes. `TfrVehicleGfx` transfers only its first 7168 bytes to VRAM word `$7200`. |
| Vehicle OBJ palette 7 | `$E6:80E0` / `0x2680E0`, 32 bytes | Target export/raw-ROM equality; formation attributes select palette 7. |
The game reads target-derived native sprite bundles. No CPU routine address is
inferred from these data matches. See [magitek-sprites.md](magitek-sprites.md).

## Patched player movement code

| Code | CPU / file offset | Evidence |
|---|---|---|
| Cancel/Sprint Shoes speed hook | `$C0:4E21` / `0x004E21`, also `$C0:4E28` / `0x004E28` | Replaces vanilla `CheckPlayerMove` speed selection with JSR/BRA sequences; exact target bytes checked. |
| New dash routine | `$C0:D674` / `0x00D674`, 25 bytes | Base speed 2, normalized Cancel bit `$07.7`, Sprint Shoes `$11DF.5`; 512 bounded A8/Y16 cases checked against native code. |
| Sliding Dash helper | `$C0:D68D` / `0x00D68D`, 18 bytes | Called from `$C0:5885/$5892`; 1024 low-coordinate/direction cases checked. |

New routine spans contain `$FF` in the clean ROM. The surrounding engine is
65816 code, while the input/status bytes are WRAM; no flat-file 6502 decoding
is used. See [player-movement.md](player-movement.md).

## Field trigger/NPC map

| Data/script | CPU / file offset | Evidence |
|---|---|---|
| Trigger table + records | `$C4:0000` / `0x040000`, 6654 bytes | 416 map ranges / 1164 records; both ROMs identical, source coordinate/order verified. |
| NPC table + records | `$C4:1A10` / `0x041A10`, 20571 bytes | 2193 records; six changed bytes in four records, see `npc-property-differences.json`. |
| Initial NPC switches | `$C0:E0A0` / `0x00E0A0`, 128 bytes | All source bitlist bytes match both ROMs; target values initialize NPC flag range. |
| Map startup pointers | `$D1:FA00` / `0x11FA00`, 1536 bytes | Retained 512 pointers; startup execution remains open. |
| Backtracking scene | `$CC:9B1D` / `0x0C9B1D`, 84 bytes | Full native scene through return; source encoding matches both ROMs. |
| First guard prefix | `$CC:9B71` / `0x0C9B71`, 51 bytes | Source/native flow through DLG_13 to battle request; no victory/progression simulated. |
| EventReturn | `$CA:5EB3` / `0x0A5EB3` | Source label/default NPC pointer and raw `$FE`; gated first guard branch verified. |

See [field-events.md](field-events.md) for format, confidence and remaining gaps.

## Battle and character maps

See [battle-data.md](battle-data.md) for CF/0000 monster properties,
CF/5000 event groups, CF/5900 formation properties, CF/6200 monster slots,
and bounded C2/09D2, C2/111B, C2/1193 and C2/11BB timing primitives.
See [characters.md](characters.md) for ED/7CA0 character properties,
ED/8220 experience, E6/F4A0 HP growth, E6/F502 target MP growth, and
C0/9F78–C0/A2F8 initialization evidence. These links distinguish data,
65816 code, implemented paths and unimplemented runtime behavior.

[equipment.md](equipment.md) adds D8/5000 ItemProp, C2/0E77–C2/111A equipment
calculation, C0/6CF3 field aggregation and native routine-comparison evidence.

[damage.md](damage.md) maps the rewritten C2/2B69 shared damage entry and
C2/0D4A helpers, caller C2/322B, C2/0C9E modifiers and C4/6AC0 attack data.
The clean C2/2B9D entry label is invalid in the target and is not used.

[battle-rng.md](battle-rng.md) maps C0/FD00 RNG data, C2/4B5A and C2/4B65,
C2/522A bit selection, C2/2575 initial gauges, CF/8400 AI pointers and
CF/8700 scripts. AI table compatibility is checked against target fetch code.

[battle-scheduler.md](battle-scheduler.md) maps C2/0033 dispatch selection,
C2/111B timing coordination, C2/4E66 queue writes, C2/03E4 command delay and
the three 256-byte WRAM queues. Callback and command-payload gaps are explicit.

[battle-commands.md](battle-commands.md) adds C2/4D1F player input,
C2/4E54 allocation, C2/4EDF payload storage, C2/4F08 MP cost and C2/0301
normal-list removal. Patched Item/Slots bytes are outside the supported paths.

[battle-advance.md](battle-advance.md) maps C2/2188 advance dispatch and
C2/031C pending-command preparation, connecting native input, command storage,
wait counters and action queues without claiming live execution.

[action-initialization.md](action-initialization.md) maps C2/0276 working
payload initialization, patched C2/02A7 call/C2/67E1 helper and CF/FE00
command properties. Original and rewritten command values remain distinct.

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
