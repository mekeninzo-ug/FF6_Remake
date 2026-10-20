# Native battle target initialization

## Mapping and extraction

`src/battle_target_init.rs` translates InitTarget C2/26D3..2777
(file 0226D3..022777), including item and Throw/Tools paths, plus
LoadMagicProp C2/2966..2989 and GetItemPropPtr C2/2B63..2B68.
Actual target code matches clean in these regions. Pinned
battle--init_target.asm and battle--init_attacker.asm supply structure.
The repeatable extractor now exports the 30 command target modes at C2/278A,
five Throw/Tools item IDs at C2/2778 and their offsets at C2/277D. All three
tables are checked against clean/target bytes and source constants.

The native initializer reads local extracted command, attack and item tables.
It does not load or execute a ROM. Confidence is high within these routines.

## State and behavior

Initialization resets restriction BA and cursor BB, then uses the command's
target mode and command properties. Commands 30 and above use default Fight
attack properties with enemy targeting. Fight ignores the selected EE/EF
special ID for this initialization stage. Magic modes load the supplied
attack and record the last spell. A menu cursor FF takes the attack's target
byte. Resurrection, random-target and character-exclusion flags augment BA.

Item mode records the item, uses item targeting, and adjusts dead-target
eligibility from its status bits. IDs at or above Sprint Shoes (230) return
without loading a spell. Lower IDs use item property 18 as a spell reference,
mask it to six bits and set B1 bit 10 when the reference's high bit is clear.
Throw/Tools map five special items to attack IDs via the extracted offsets;
other items retain their item properties and do not load a spell.

LoadMagicProp copies all fourteen bytes into working attack properties,
doubles the special-effect byte, and clears it if the original high bit is
set. Unused state, including prior working properties on item-only paths,
is retained exactly. Source carry-dependent branches are preserved.

## Validation and integration

`python tools/verify_battle_target_init.py` compares every command/attack
byte combination: 65,536 cases and 3,495,034 actual target instructions.
The checker executes item pointer calculation, the native 65816 block move,
working-property transformations and target flags. It compares BA, BB, B1,
last item/spell and all fourteen working bytes, and checks X/Y preservation.
Only the separately verified hardware multiply helper is modeled. Captures
and traces remain local under captures/native-battle-target-init.

The Guard AI-to-command test now uses this initializer before target
selection, replacing supplied Fight target flags. The AI-turn comparison's
1,024 targeted cases also execute actual CalcCmdDelay, InitTarget and
ChooseTarget, with final storage intercepted. Across its mapping, Guard,
controlled and targeted cases it executes 1,491,662 instructions. This
checks shared RNG ordering through multiple emitted attacks and Confusion.
Separate command-pool tests cover native storage. Neither proves a complete
battle, legal command UI, victory, rewards or original runtime equivalence.

## Remaining scope

AI targeting commands and conditions, complete monster/character loading,
downstream status-flag processing, attack execution, rendering, audio and battle completion
remain unfinished. Item consumption and effects are not implied by item
target initialization. The live story still pauses at the first battle.

Complete status update is now available in [battle-status-update.md](battle-status-update.md); downstream flag consumers and full battle execution remain unfinished.
