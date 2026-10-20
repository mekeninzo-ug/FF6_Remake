# Elemental damage and counter accumulation

## Evidence and mapping

Primary source: pinned FF6 disassembly `battle--calc_dmg.asm`, checked against
both supplied ROMs. C2/0B83..0C9D maps to file offsets 020B83..020C9D;
these bytes are identical in clean and target ROMs. Native implementation:
`src/elemental_damage.rs`. Confidence is high for this bounded arithmetic stage.

After prior modifiers or ratio calculation, power zero skips contribution.
Petrify forces zero and clears contribution flags before undead handling.
The invert-undead flag, undead monster flag, attack Dead/Zombie status bits,
and drain flag control early return and healing inversion. Element handling
uses this priority: globally blocked, absorb, nullify, half, weakness. Any
matching element can select a resistance. The globally blocked check requires
all attack elements to be blocked; it does not remove individual bits before
later resistance checks. Weakness doubles only values below 8000 hex.

C2/0C76..0C9D routes the contribution to damage or healing counters. The
single-counter arithmetic treats FFFF as empty, adds the amount, and caps at
9999. A zero contribution replaces the empty sentinel with zero.

## Validation

`python tools/verify_elemental_damage.py` compares 16,384 resolution cases
against 468,700 actual target instructions, and 4,152 counter cases against
89,199 instructions. The checker supplies the prior calculated amount and
intercepts the later CalcMaxDmg routine. Separate accumulator checks execute
actual counter routing and confirm the other counter remains untouched.
Captures and selected instruction traces are generated locally under
`captures/native-elemental-damage/`; the report is
`elemental-damage-validation.json`.

## Limits

This is a native battle component, not live combat. It does not apply current
HP/MP, resolve hits, implement paired drain or physical status effects, or
finish an action. Ultima Weapon special effect 4 returns an explicit error.
Original hardware runtime comparison remains unavailable. These checks do
not establish a complete battle or victory.
