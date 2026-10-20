# Monster working stats and immunity normalization

## Mapping and evidence

`src/battle_actor_stats.rs` implements target monster stat projection at
C2/2CE4..2D48 (file 022CE4..022D48), FixImmuneStatus C2/2650..2674, and
UpdateImmuneStatus C2/2675..26C8. The actual target bytes match clean in
these regions. Pinned `battle--init_2.asm` and `battle--init.asm` provide
labels and structure. InvertEvade and AddHalf helpers are also executed by
the comparison checker. Confidence is high within these bounded regions.

## Monster projection

The native projection accepts a target 32-byte monster record, existing
maximum HP and existing retaliation flags. Stamina uses maximum HP, not
current HP: floor(max HP / 512) plus 16, capped at 40. The disassembly's
comment says maximum 39, but actual CMP/ADC carry behavior produces 40.
Other projected fields are attack power, hand graphics item, working physical
and magical evasion, hit rate, level, speed, Magic Power (floor 1.5 times
property, capped 255), retaliation bits 82 and monster status. This is the
stat portion of loading, not the entire LoadMonsterProp routine.

## Immunity conventions

The source calls them immunity bytes, but a set bit means the corresponding
status is allowed. FixImmuneStatus clears protection bit 04, then sets it
if Dead was blocked and makes Dead itself allowed. Poison element nullification
blocks Poison status. UpdateImmuneStatus blocks Float when equipment grants
it, and blocks Morph when permanent morph is active. Equipment statuses are
masked with EE78 before restricting status 2/3. Sap/Regen are both blocked
if either is blocked, as are Slow/Haste. All unrelated bits are retained.

## Validation and integration

`python tools/verify_battle_actor_stats.py` compares 3,456 monster cases
(all 384 target records at nine maximum-HP values) and 8,192 cases for each
immunity routine. The checker executes 599,167 actual target instructions
without hooks. It compares all projected fields and all immunity/protection
outputs. Captures and selected traces remain local under
captures/native-battle-actor-stats; the durable report is
battle-actor-stats-validation.json.

A Rust integration test loads actual event group 1/formation 2 and both
present Guards. Guard values include Stamina 16, Speed 30, Level 5, attack
power 16, Magic Power 9, working Evade/MagicEvade 255, HP 40 and MP 15.
It feeds a Guard's derived stats and actual Fire Beam record 131 through
native hit resolution, base damage, modifiers, elemental handling,
accumulation and HP application using a controlled identity RNG table.
The controlled contribution is 182; HP becomes zero and Dead is queued.
This proves component wiring under specified inputs, not an original
runtime battle, command legality, victory or rewards.

## Remaining scope

Full monster loading still needs AI/retaliation script setup, drops/steals,
name aliases, HP restoration and Doom Gaze persistence, Rage properties,
special animation, first-strike queues, randomized Strength, and subsequent
downstream status-flag processing. Character initialization is also incomplete. No live battle
or original PPU/audio runtime comparison is claimed; the story still pauses
at the first battle.

LoadRageProp is now implemented and connected to the Guard integration test; see [battle-rage.md](battle-rage.md). Full monster loading and downstream status-flag processing remain incomplete.

Complete status update is now available in [battle-status-update.md](battle-status-update.md); downstream flag consumers and full battle execution remain unfinished.
