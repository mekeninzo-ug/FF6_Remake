# Character equipment projected into battle stats

## Mapping and evidence

Native `src/battle_equipment.rs` translates target C2/2883..2943
(file 022883..022943), the stat-copy portion of UpdateEquipBattle, plus
InvertEvade at C2/2861..286C. These target bytes match clean ROM. Primary
labels and structure are from pinned `battle--init_attacker.asm`.
Confidence is high for this bounded projection, not complete actor setup.

Inputs are the 64-byte equipment scratch record produced by the existing
native equipment aggregator, plus existing party relic bits. The output is
a typed BattleEquipment value; the game does not execute 65816 instructions.
A separate output preserves modified scratch bytes for subsequent consumers.

## Transformations

Strength doubles with byte saturation. Speed, stamina and magic copy through.
Evade and MagicEvade double with saturation then negate as a byte; zero
becomes FF. Examples: raw 0 => 255, 1 => 254, 127 => 2, 128..255 => 1.
These working values belong in hit resolution, unlike menu percentages.

The source records the Genji visual flag before applying its hand-item
mask. The mask can remove relic effects. Without Gauntlet, both hand weapon
flags lose their two-hand bit 40. Relic effects move the Dragon Horn bit into
the packed effects 1/3 record, preserving source bit shifts. Party relic
bits accumulate by OR. Relic Ring supplies the undead flag and all character
projections set HUMAN. Hand powers, hit rates, spellcasts, elements, special
IDs and item IDs, defense, equipment status, run factor, resistance and block
graphic masks are projected. Status immunity begins as the complement of
scratch blocked-status bits; FixImmuneStatus must still normalize it later.

## Validation

`python tools/verify_battle_equipment.py` compares 8,192 controlled scratch
records and all three opening party records (8,195 total), executing 786,672
actual target instructions. The bounded checker uses direct page 1100 for
this region and executes InvertEvade without hooks. All projected fields,
party effects, per-character Genji flags and resulting scratch bytes match.
Native captures and selected traces are local under captures/native-battle-equipment.

A connected Rust test loads Terra's actual target character and equipment,
checks Strength 62, Speed 33, Stamina 28, Magic 39, working Evade 226 and
MagicEvade 242, and passes projected values into the complete native hit
checker. This proves component wiring under controlled inputs, not original
runtime agreement or a full battle.

## Remaining scope

Moogle Suit sprite selection at the beginning of UpdateEquipBattle, final
visual flags and FixImmuneStatus are outside this projection. HP/MP relic
boosts, downstream status-flag processing, complete character battle initialization, monster
initialization and live combat remain unfinished. The story still pauses
at the first battle. No original PPU/runtime capture is claimed.

FixImmuneStatus and UpdateImmuneStatus are now implemented separately in [battle-actor-stats.md](battle-actor-stats.md); full actor setup is still incomplete.

Complete status update is now available in [battle-status-update.md](battle-status-update.md); downstream flag consumers and full battle execution remain unfinished.
