# Monster special and common attack preparation

## Mapping and evidence

`src/battle_attack_prep.rs` translates target C2/32EC..33A2
(file 0232EC..0233A2), the next stage after MP payment. These code bytes
match clean ROM. Pinned battle--attack.asm provides labels and structure;
actual target instructions govern the native arithmetic and flag ordering.
Confidence is high within this bounded stage, not the full attack routine.

## Special properties

When the selected attack setup identifies a monster special (index 6),
preparation sets the source attack/graphics flags and selects the monster's
special animation. Property bit 40 makes it status-only by clearing power
and setting the status-immunity failure flag. Bit 80 makes it undodgeable.
The lower six bits select status 00..1F, damage modifier 20..2F, HP/MP drain
30/31, or Reflect removal 32..3F. Status bits merge into the existing attack
payload. Damage modifiers add one plus (property minus 20) half-damage steps,
with byte wrapping. Guard property 20 therefore adds one half-damage step.

The common stage sets B2 bit 40, requesting the Runic graphics/copy sequence
only if it was previously clear; it does not implement Runic absorption.
Morph adds two modifier steps, physical Berserk adds one, each with byte
wrapping. Multiple targets halve base damage unless the attack's no-split
flag is set. Back row can halve it again unless B3's ignore-row bit is set.
The two halves truncate sequentially in the same source order.

## Animation data and integration

The repeatable battle extractor now exports all 384 special animation bytes
from CF/37C0 (file 0F37C0). This table is unchanged by the patch. BattleData
exposes it by monster ID; the battle-data comparator checks every exported
value against the target. The Guard's animation is 3.

A connected Rust test loads the actual first Guard record and its shared
monster/Rage traits, target initialization, and animation. It supplies
Strength 56 and a successful hit as controlled inputs, because complete
monster Strength initialization/Fight execution remain unfinished. The
native base/modifier pipeline produces base 16, normal damage 10 and special
damage 15 against Terra's projected defense 97. Applying that contribution
reduces her initial HP 63 to 48 without queuing Dead. This is controlled
component integration, not a full enemy turn or original runtime capture.

## Validation

`python tools/verify_battle_attack_prep.py` compares 16,384 cases and 981,581
actual target instructions. All 256 property bytes are covered across
controlled existing flags, status payloads, modifiers and base damage.
GetBitPtr executes actual code; only the Runic graphics/copy interfaces are
hooked. The checker compares all working properties, attack/graphics flags,
animation, modifier byte, base word and requested effects. Captures and
selected traces remain local under captures/native-battle-attack-prep.

## Remaining scope

UpdateFacingDirection, DoAttackerEffect, complete Fight/monster setup,
Runic/Air Anchor behavior, target effects, downstream status-flag processing, rendering, audio
and battle completion remain unfinished. The live story still pauses at
the first battle. Queued graphics IDs do not imply rendered animations.

Complete status update is now available in [battle-status-update.md](battle-status-update.md); downstream flag consumers and full battle execution remain unfinished.
