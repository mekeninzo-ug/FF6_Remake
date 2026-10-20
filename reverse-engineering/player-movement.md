# Player movement and patched dash behavior

## Evidence and mapping

The pinned upstream `src/field/player.asm` describes `UpdatePlayerMovement`
(CPU `$C0:485F`), `CheckPlayerMove` (`$C0:4E16`) and step effects. Object action
updates are phased every four field frames; `MoveObjs` advances 8.8 fixed-point
positions every frame. The active party leader occupies slot zero after sorting.
`entrance.asm::CheckEntrances` explicitly rejects objects between tile boundaries.
`reset.asm::FieldLoop` checks entrances before object actions and movement.

The patched ROM replaces vanilla speed selection inside `CheckPlayerMove`:

| CPU address | File offset | Target bytes / meaning |
|---|---|---|
| `$C0:4E21` | `0x004E21` | `20 74 D6 80 0F`: JSR `$D674`, BRA to common speed store |
| `$C0:4E28` | `0x004E28` | `20 74 D6 80 08`: second call/branch in replaced vanilla path |
| `$C0:D674` | `0x00D674`, 25 bytes | New Cancel/Sprint Shoes speed routine; clean ROM contains `$FF` |
| `$C0:5885`, `$C0:5892` | `0x005885`, `0x005892` | JSR `$D68D` replaces walking-coordinate shifts |
| `$C0:D68D` | `0x00D68D`, 18 bytes | Sliding Dash gait helper; clean ROM contains `$FF` |

These are bounded 65816 routines interpreted with 8-bit accumulator, 16-bit
index registers, and mapped WRAM inputs. They are not 6502 disassembly or a
flat-ROM CPU interpretation. `player-movement-validation.json` retains raw
bytes, per-instruction CPU/file locations and branch examples. The validator
models only instructions needed by these two routines, outside the game runtime.

The supplied bug-fix compendium calls these changes Cancel Button Dash (Master
ZED, Version C) and Sliding Dash (SilentEnigma). Byte evidence supports the
speed/gait behavior below; the notes' broader event/config compatibility claim
has not been compared through original runtime captures.

## Ported rules

The routine starts with speed index 2, increments it if normalized control
byte `$07` has bit `$80`, and increments it again if `$11DF` has bit `$20`.
Unlike the original inline selection, this hook does not read `$1EB8` bit 1.
The movement multiplier at `$C0:7ECA` is unchanged. Cardinal rate `$0040`
times multipliers 4, 8 or 16 gives 1, 2 or 4 pixels per logical frame:

| Cancel held | Sprint Shoes effect | Speed index | Frames per 16-pixel tile |
|---|---|---|---|
| No | No | 2 | 16 |
| Yes | No | 3 | 8 |
| No | Yes | 3 | 8 |
| Yes | Yes | 4 | 4 |

Equipment is not ported, so live field control currently supplies no Sprint
Shoes effect. The core supports and validates all four combinations; no
prototype equipment toggle substitutes for actual equipment behavior.

The gait hook shifts the low coordinate byte right twice, adds one when facing
right or down, then shifts once more. Its caller masks with 3. The native
on-foot phase helper follows these target bytes. Magitek has a separate,
unchanged coordinate-based gait path. Full on-foot sprite DMA/idle/flip details
and hardware presentation remain incomplete.

Native movement keeps its completed tile plus a signed fractional displacement.
At phase zero it samples direction (right, left, up, down priority), checks the
existing map-level passability, locks direction/speed and increments the step
counter. Movement advances every logical tick. Releasing or changing controls
mid-step does not turn or change velocity. Arrival commits the new tile;
entrances are checked on the next tick, before starting another movement.
A map change consumes that tick. Menus queue until tile alignment. Poison,
encounters, map events, doors and complete actor/bridge collision are not supplied
by this movement port.

The display camera and Magitek gait use actual intermediate world-pixel
coordinates. Keyboard X maps to held Cancel/dash; Shift remains an additional
dash binding. Esc/I open the menu. Controller remapping is not implemented.

## Saves and validation

Version 7 local saves retain facing, field frame/phase, fractional logical
clock, locked movement direction/speed/progress, pending aligned entrance state,
event bits, vehicle flags and existing prototype state. Invalid direction,
speed, fractional progress and off-map motion are rejected. Menu input requests
are transient. Versions 2 through 6 remain readable at tile-aligned state;
old formats did not retain facing or partial player movement. This is not SNES
SRAM compatibility.

`verify_player_movement.py` checks 512 input combinations against the mapped
speed routine and 1024 coordinate/direction combinations against the gait
routine. Native diagnostics include normal vertical/horizontal walking, dash,
a blocked step and a real entrance transition. All 97 game states match the
expected pixel/tile progression. Twenty-eight full rendered frames (1,605,632
pixels) match independently composed map and raw-ROM Magitek/rider pixels.
Save/restore and mid-step control changes are also covered by Rust tests.

This establishes bounded native behavior and implementation agreement. Original
emulator timing, exact refresh, DMA cadence, camera clipping/parallax and PPU
priority remain unverified. The game uses a nominal 60 Hz clock and clamps long
host deltas; diagonals, input remapping and the remaining game systems stay open.

Version 8 retains the version-7 player movement fields and adds direct snapshots
for modeled event/actor state. The complete initialization, party and SRAM
systems remain open; see [field-events.md](field-events.md).

Version 9 adds initialized character records. Native Sprint Shoes aggregation
now follows the unchanged C0/6CF3 enabled/current-party loop and reads target
item field effects. Tests assign the item to a guest, check membership gates,
combine it with Cancel for speed 4 and restore an in-flight movement snapshot.
See [equipment.md](equipment.md). Opening gear has no Sprint Shoes, so normal
story and geometry-reference movement remain unchanged.
