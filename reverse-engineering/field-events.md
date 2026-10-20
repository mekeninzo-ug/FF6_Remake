# Field triggers, NPC data and event continuation

## Mapped tables

Source: pinned FF6 commit `813013276c952fdd27edcf7b8b86f17291542cbf`,
`event_trigger.asm`, `npc_prop.asm/inc`, `map_init_event.asm`, and field
`event.asm`, `obj.asm`, `init_npc_switch.asm`. CPU addresses below use canonical
HiROM mapping; file offsets do not include a copier header.

| Data | CPU / file offset | Layout / evidence |
|---|---|---|
| Field trigger table | `$C4:0000` / `0x040000`, 6654 bytes | 417 u16 relative pointers, 416 map ranges, 1164 five-byte records. Last sentinel excludes fixed-block padding. |
| NPC property table | `$C4:1A10` / `0x041A10`, 20571 bytes | Same pointer count, 2193 nine-byte records; padding is excluded. |
| Initial NPC switches | `$C0:E0A0` / `0x00E0A0`, 128 bytes | 1024 bits, 278 initially set; source bitlist matches both ROMs. |
| Map startup pointers | `$D1:FA00` / `0x11FA00`, 1536 bytes | 512 three-byte script-relative pointers, retained for later startup dispatch. |

Each trigger is X, Y, then a 24-bit event offset relative to `$CA:0000`.
Pointers within trigger/NPC tables are relative to their own table base, not
absolute ROM offsets. Repeated pointers mean empty maps. `CheckEventTriggers`
requires an aligned, user-controlled party and no running event; source field
order checks triggers before event dispatch, then entrances, then object actions.

`tools/extract_native_events.py` reads verified ROMs and source before writing
local bundles under `extracted/twue-3.07/build/native_events/`. All 1164 trigger
coordinates/orders match the source, and 1122 hexadecimal source target labels
resolve exactly. The other 42 named labels are retained as ROM targets; this
check does not independently resolve their names. Raw source tables and target
bytes are authoritative. Trigger pointer/data bytes match the clean ROM.

## NPC properties and differences

Normal NPC records pack:

- Bytes 0-1: event pointer low word. Byte 2: event bank bits 0-1, palette bits
  2-4, BG2 scroll bit 5, low two switch bits 6-7.
- Byte 3: remaining switch bits. Global switch ID is
  `0x300 + (byte3 << 2) + (byte2 >> 6)`.
- Byte 4: X in low seven bits; high bit is rider/special data depending on kind.
- Byte 5: Y in low six bits, speed in high two bits.
- Byte 6: map-sprite graphics ID.
- Byte 7: movement, sprite priority, vehicle/animation fields.
- Byte 8: direction, reaction, layer priority and animation fields.

The loader handles normal NPC positions, direction, speed, palette, graphics,
vehicle flags and initial enabled/visible state. Special/animated formats are
preserved raw but fail closed when creation/display would require unsupported
rendering. Random NPC movement, interaction events, complete collision and
priority are not implemented. Initial NPC bits occupy global event bits
`0x300..0x6FF`; Narshe's five NPC records use bit `0x604`, initially clear.

NPC pointer tables are unchanged, but four records contain six changed bytes:

| Map / NPC (one-based) | CPU / file offset | Change |
|---|---|---|
| 200 / 13 | `$C4:3FE1` / `0x043FE1` | Sprite-priority field changes from normal to low; layer priority changes from top-sprite-only to default. |
| 200 / 14 | `$C4:3FEA` / `0x043FEA` | Same field changes. |
| 317 / 12 | `$C4:574B` / `0x04574B` | Palette 5 (Mog/Umaro) changes to 3 (Strago/Relm/Gau/Gogo). |
| 317 / 25 | `$C4:57C0` / `0x0457C0` | Same palette change. |

See `npc-property-differences.json`. These bytes are preserved from the target;
these maps' rendering and the intended named patch fixes are not validated.

## Native script execution

The live VM now supports object create/delete, wait-for-object, party-slot
aliases, character party membership, calls/repeated calls, OR/AND switch
conditions, field-control flags and no-ops in addition to its earlier subset.
Call frames retain return PC, subroutine start and repeat count (a zero byte
in repeated calls means 256 iterations). Eight-condition
OR/AND forms use bit 15 as required set/clear polarity; 1020 truth combinations
are covered by Rust tests. Pointers and stack limits are checked.

`$3C` selects field display slots; it does not remove characters from battle
party membership. `$3F` changes membership. Initial Terra/Wedge/Biggs membership
and graphics/palettes come from GameStart at `$CA:5E33`. In particular, Wedge and
Biggs use soldier graphics 14 with palette 1, while Terra uses graphics 0 and
palette 2. These command sequences match both ROMs and native actor states.
This corrects the earlier slice's palette-2 soldier assumption. Complete roster,
battle-slot, equipment and character-property state remain unported.

Active slot scheduling follows selected current-party members, other enabled
current-party characters, camera, other enabled characters and NPCs, capped at
24 objects. The list is cached and refreshed on the pre-dispatch four-frame
sort boundary, so newly created NPCs do not act early. All object commands run
before any actor moves. The selected-slot command also requests a sort.
Full per-character map ownership remains incomplete. Object `$D5`
(position) and `$D7` (copy party position) yield until the next object update,
as their source handlers return to `UpdateNextObjAction`. This corrects the
previous immediate continuation and changes the opening diagnostic from 405 to
417 logical frames. Original loader/refresh timing is still unvalidated.

Validated Narshe scenes:

| Trigger | Script CPU / file offset | Verified native result |
|---|---|---|
| Map 19 `(38,50)` | `$CC:9B1D` / `0x0C9B1D` | Target DLG_12, visible Wedge/Biggs movements, return to `(38,49)`; display slots become Terra-only while all three remain party 1. |
| Map 19 `(38,38)` | `$CC:9B71` / `0x0C9B71` | Bit `$0128` gates the scene. Two guards use graphics 52/palette 1/speed 3, DLG_13, then event battle group 1 with flags `$3F`. |
| Guard scene with `$0128` set | Branch to `$CA:5EB3` / `0x0A5EB3` | Return without spawning guards or blocking movement. |

The battle command yields a typed request and pauses. It does not invoke the
old prototype battle, award rewards, or set victory/progression bits. The live
story currently stops here until the real battle system is implemented. The
121-step mine traversal runs in explicit geometry-only diagnostic mode with
story triggers disabled; it is not a story-completion capture.

## Saves and validation

Versions 8 and 9 local saves carry `F6EV3` snapshots of modeled event state: script PC,
call frames, object cursors, waits/movement, sort state/cached slot order, actors, flags, membership,
field display slots and pending battle request. Event dialogue ID/flags, fault
latch, clocks and NPC-map initialization state are retained in outer metadata.
A payload FNV-1a checksum and structural bounds detect invalid snapshots.
Diagnostic command/effect history is omitted. Older local formats remain
readable through their legacy paths; this is not cartridge SRAM compatibility.

`verify_field_events.py` independently source-encodes 135 bytes across the
backtracking scene and guard prefix through battle, compares both ROMs, and
checks 84 native command spans including the gated return. Actual walking
activates four captured scene states. Native capture code asserts identical
serialized actor/VM state after restoring DLG_12 and the battle pause.

Confidence is high for mapped bytes, the covered VM subset, actor property
selection and tested continuation. Startup pointers are not dispatched yet.
General events, choices/text controls, NPC interactions, full initialization,
collision, audio and battle remain incomplete. Asynchronous actor tasks that
outlive the root event and player control during running events still need
field-loop integration. Dialogue placement/PPU priority,
font fidelity, DMA cadence and original runtime timing are unverified; stored
BOTTOM flags do not yet reproduce the original window positioning.
