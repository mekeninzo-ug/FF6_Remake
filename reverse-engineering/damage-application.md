# HP and MP damage application

## Mapping and confidence

Pinned `battle--apply_dmg.asm` supplies labels; actual supplied ROM bytes
confirm C2/1323..13D2 (file 021323..0213D2) is unchanged by the patch.
Native code is `src/damage_application.rs`. High confidence within the
bounded helpers; full battle behavior is incomplete.

C2/13A7 computes healed minus taken. FFFF counters mean absent. Invincibility
rewrites a valid damage-taken counter to zero. If either global Golem/dog
block-target byte lacks its high bit, taken damage is suppressed for the
net calculation. Healing remains eligible. Both contributions can coexist.

C2/1323 applies HP and C2/1350 applies MP. Net zero leaves resources and
update flags untouched. Healing clamps to maximum, including 16-bit addition
overflow; damage clamps underflow to zero. MP changes request spell/esper
refresh (update flag 0080). Reaching zero MP kills only actors whose monster
flags contain DIE_AT_0_MP. C2/1390 clears HP and weapon spellcast, and queues
Dead (0080) in pending status at 3DD4 unless current status is Zombie.
This does not commit status or run removal, rewards, or victory processing.

The source comment describing helper carry as death is too broad: nonlethal
subtraction also returns carry set. Native Outcome preserves raw helper carry
and callers must not interpret it as a death predicate. The outer ApplyDmg
routine, including attacker attribution and saved processor flags, is outside
this component.

## Comparison

`python tools/verify_damage_application.py` executes 16,384 controlled HP/MP
cases and 543,912 actual target instructions. It executes net calculation,
death helper, SetStatus1 C2/0E32 and SetCharFlag C2/464C without hooks. Inputs
include full-width arithmetic edges and absent counters. Results compare HP,
MP, pending status, update flags, weapon spellcast, damage counter and carry.
Traces and native CSV are local under captures/native-damage-application.
No original SNES runtime comparison is claimed.

A Rust integration test connects base damage, modifiers, elemental resolution,
accumulation and HP application for a controlled level-3 power-60 magic attack
against a 40-HP, 140-magic-defense target: 459 base, 181 contribution, zero HP,
Dead pending. Hit resolution and legal target selection are assumed inputs;
this is not a complete Magitek action or a live battle.
