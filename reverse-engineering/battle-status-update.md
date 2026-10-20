# Complete native status update and patched handlers

## Mapping and evidence

`src/battle_status_update.rs` implements UpdateStatus C2/4391..4405,
all 32 set and 32 remove dispatch entries, and their helpers through
C2/46AF. Dispatch tables are at C2/46B0 and C2/46F0. File offsets are CPU
addresses minus C00000. Pinned battle--status_effect.asm supplies the vanilla
structure; actual patched code/table bytes govern differences. Confidence
is high within UpdateStatus, not downstream battle flag consumers.

Actors are processed in descending source slot order. Present actors run
preparation, set-status handlers (high bits first), clear-status handlers
with source immunity filtering, CalcStatus, current-status storage and pending
reset. Handler changes to subsequent clear masks remain visible in source
order. Absent actors retain pending requests. Status entries that point to
RTS have no invented effect.

## Side effects and target changes

The native routine includes death/Petrify cleanup, revival masks, Zombie
normalization, Confuse and Vanish masks, Imp equipment updates, Sleep/Stop/
Reflect/Frozen counters, Condemned and command refresh flags, Sap/Regen and
Slow/Haste exclusions, and Morph refresh requests. These are the source's
flags/counters, not substitutes for later battle processing.

The target rewrites Confuse/Vanish handlers and their dispatch pointers, and
adds Dance/Rage removal to character Imp/Rage clearing. The native code
follows those instructions. Equipment change uses a word increment at a
byte-indexed flag; overflow can carry into the next byte, which is retained.
Imp can also cause a Rage-clear handler call and a second increment.
Patched changed-character masks retain character bits while monster-specific
masks use the high target byte.

## Validation and clean comparison

`python tools/verify_battle_status_update.py` compares 2,688 complete ten-actor
states: 2,048 controlled random states plus 640 isolated status/actor cases.
It executes 5,600,730 actual target instructions without hooks and verifies
that all 64 dispatch entries are exercised. Compared fields include current
and pending words, retaliation snapshots, actor flags/update requests, status
counters, equipment flags and all affected global masks. It also checks
preserved X. Source ROM files remain unchanged.

The 640 isolated cases are run against clean ROM as a baseline. Twelve
character cases differ: Imp set, Imp clear and Rage clear for four character
slots. Full before/after results are local in version-handler-differences.json;
changed code/table offsets and summary counts are durable in
battle-status-update-validation.json. Selected traces and native CSV remain
local under captures/native-battle-status-update. This verifies component
behavior under supplied states, not original runtime equivalence.

## Connected native paths

Tests connect lethal HP damage to a committed Dead status, confirm menu and
update flags, then run a controlled revival and check monster masks. Other
tests connect queued Poison to full status update, and actual Fire Beam
properties to Frozen removal. Frozen's counter is retained on removal,
because its remove handler has no reset effect. A focused test covers the
patched Imp/Dance/Rage and equipment-word carry behavior.

## Remaining scope

Downstream processing of update flags, ATB/menu refresh, death/retaliation and
victory handling, complete attack/action execution, rendering, audio and field
return remain unfinished. A committed Dead bit does not establish enemy
removal, rewards or victory. The live story still pauses at the first battle.
Original PPU/runtime comparison remains unavailable.

Living-mask rebuilding and normal-action cleanup are now implemented separately in [battle-cleanup.md](battle-cleanup.md); full post-action sequencing and victory remain unfinished.
