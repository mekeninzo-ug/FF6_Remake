# Morph and Revert state transitions

## Evidence and mapping

`src/battle_morph.rs` translates UpdateMorph C2/0AA8..0B35 (file offsets
0x20AA8..0x20B35) and SaveMorphCounter C2/0B36..0B49
(offsets 0x20B36..0x20B49), from pinned battle--action_1.asm.
Both code regions match clean ROM. It composes the verified native
UpdateCmdList translation rather than duplicating that eligibility logic.
Confidence is high within the transition routines; live Morph rendering,
duration ticking, full dispatcher invocation and story progression remain
unimplemented.

## State and branch behavior

Outside retaliation (B1 bit 0), a zero high ATB byte enables flags0 bits
0x88. Four command IDs are scanned: Morph (3) becomes Revert (4) when
morphed, or Revert becomes Morph when reverted. Only matching IDs change.

Revert calls SaveMorphCounter, invalidates the global morphed actor, clears
the actor's Morph gauge byte, refreshes character commands, then clears the
battle Morph counter. SaveMorphCounter does nothing when no actor is
morphed; otherwise it multiplies the saved resource by the battle counter's
high byte and stores the product's high byte. The resource calculation is
truncated eight-bit arithmetic, not a floating-point percentage.

Morph initializes the global actor and counter only when the previous
actor byte has bit 7 set and story flags bit 2 is clear. It sets counter to
FFFF and computes rate from unsigned FFFF divided by saved resource.
Hardware division by zero yields FFFF. If story flag bit 1 is set, divide
the quotient by two; then divide by eight, cap at 2047 and add one. The
source comment identifies the bit-2 bypass as permanent Morph. Narrative
meaning and event transitions of these story flags need further integration.
An existing morphed actor or permanent Morph leaves global timer/rate
unchanged while retaining the earlier command swap and readiness changes.

## RAM layout

| State | Address under DBR 7E |
| --- | --- |
| Morphed actor offset / FF invalid | 3EE2 |
| Battle counter | 3F30 word |
| Rate | 3F32 word |
| Saved resource | 1CF6 byte |
| Story flags read by this handler | 3EBB byte |
| Actor Morph gauge | 3B04 + actor*2 |
| Actor status4 / Morph bit 08 | 3EF9 + actor*2 |
| Actor menu base | 202E + actor*12 |

The native test interface preserves all ten actor-indexed command regions,
including source behavior for monster slots. Only character slots invoke
UpdateCmdList during Revert.

## Validation

`python tools/verify_battle_morph.py` compares 8,192 cases and 1,039,213
actual target instructions. It executes UpdateMorph, SaveMorphCounter,
UpdateCmdList and multiply/divide helper code without routine hooks. The
bounded harness models multiply/divide registers, including divide-by-zero,
but not hardware cycle timing. Inputs span all ten slots, Morph/Revert,
retaliation, ATB boundaries, permanent/existing Morph, resource 0/1/15/16/255,
status/weapon/relic eligibility and timer values. It compares every menu
record byte and all modeled output globals/flags/gauge state. A reversed
rate branch in the first native implementation was corrected after target
comparison failed; the final fixture set passes. See
[battle-morph-validation.json](battle-morph-validation.json).

Three Rust tests cover Morph-to-Revert resource retention, existing/permanent
Morph bypass and capped zero-resource/story-flag rate behavior. These
transitions are not yet wired into live battles or cartridge save data.
