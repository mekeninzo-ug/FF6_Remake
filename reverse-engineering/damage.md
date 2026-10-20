# Patched damage arithmetic and attack properties

Target evidence: verified TWUE 3.07 + Bug Fixes ROM, all supplied patch notes,
and everything8215/ff6 commit `813013276c952fdd27edcf7b8b86f17291542cbf`.
The clean disassembly provides labels, callers and data formats. Its vanilla
base-damage implementation does **not** describe the target implementation.

## Code and data map

| Region | CPU address | File offset | Evidence / status |
|---|---|---|---|
| MagicProp | C4/6AC0 | 046AC0 | 256 × 14-byte attack records, two changed bytes |
| Patched base damage | C2/2B69–C2/2C20 | 022B69–022C20 | 184 bytes, 178 differ; shared physical/magic entry |
| Patched wide arithmetic/relic helpers | C2/0D4A–C2/0D86 | 020D4A–020D86 | 61 bytes, 55 differ |
| Main attack caller | C2/322B | 02322B | `20 69 2B EA EA EA`, updated entry and removed old relic call |
| Target modifiers | C2/0C9E–C2/0D49 | 020C9E–020D49 | 172 bytes unchanged, includes MultDmg helper |
| Shift helper | C2/0DD1 | 020DD1 | Variable shift of the wider intermediate |
| Damage multiplier | C2/370B–C2/372E | 02370B–02372E | Unchanged multiplier and saturation |
| Hardware multiplication | C2/4781 | 024781 | Verified SNES unsigned 8×8 helper |
| Wider multiplication | C2/47B7–C2/47D5 | 0247B7–0247D5 | Unchanged Mult24, two hardware multiplications |
| Battle RNG | C2/4B5A | 024B5A | Table/index operation now ported; live call ordering remains open, see battle-rng.md |

Canonical HiROM mapping is used. Code runs on the 65816 with changing M width,
eight-bit indices in these paths, direct-page scratch and a native stack.
The input image is not treated as flat 6502 code. SPC execution is separate.

The clean `CalcDmg` label C2/2B9D is now inside a patched instruction operand,
not a valid target entry. The caller at C2/322B invokes C2/2B69 instead, and
the old separate RelicDmgEffect call is replaced by three NOPs. Preserve those
target changes when porting callers. The patch notes' **Ultimate Damage** entry
describes overflow and Offering/Genji Glove reductions applying to too many
physical commands; target bytes and native comparisons support that behavior.
Exact attribution of every instruction to an individual IPS component is open.

## Annotated target control flow

| CPU / file offset | Meaning established from target instructions |
|---|---|
| C2/2B69 / 022B69 | Load level, save multiplier and level in Y |
| C2/2B6F / 022B6F | Read physical flag, save carry in P on stack |
| C2/2B76 / 022B76 | Load power; select monster physical or nonzero-level magic ×4 term |
| C2/2B89 / 022B89 | Save term; recover physical selector from stacked P |
| C2/2B90 / 022B90 | Check B2 bit 4000, otherwise form rounded-up 1.75 physical power |
| C2/2BA1 / 022BA1 | Add attacker stat with byte-to-word carry |
| C2/2BA8 / 022BA8 | First wider multiply; next multiply uses power for magic or level for physical |
| C2/2BBA / 022BBA | Merge high intermediate through patched helper C2/0D5C |
| C2/2BC1 / 022BC1 | Select /32 magic or /256 physical shift |
| C2/2BCF / 022BCF | Magic skips the physical-character additions |
| C2/2BD2 / 022BD2 | Monsters skip character-only power/relic reductions |
| C2/2BD6 / 022BD6 | Round-up 1.5 character term, then add base power |
| C2/2BDD / 022BDD | Apply Offering/Genji reductions only for command 00 or 06 |
| C2/2BE5 / 022BE5 | Read relic effects, halve for Offering with upward rounding |
| C2/2BF5 / 022BF5 | Test shifted Genji bit; apply rounded-up three-quarter damage |
| C2/2C0B / 022C0B | Restorative attacks skip damage-boost relics |
| C2/2C14 / 022C14 | MP-affecting attacks use the magic damage-boost selection |
| C2/2C1A / 022C1A | Read relic damage word and enter shared boost/cap helper |
| C2/0D78 / 020D78 | Saturate wider result to 65535, then store base damage |

Raw target region exports and digests are local in `build/native_magic/`.
Four full instruction/state traces are retained in
`captures/native-damage/patched-instruction-traces.json`; every row links
CPU address, file offset, opcode, A/X/Y/P and stack pointer. These are bounded
target-instruction traces, not an original battle runtime capture.

## Native base arithmetic

`damage.rs::BaseDamage` receives the source inputs explicitly: level, power,
projected Strength or MagicPower, physical/magic and monster/character flags,
restorative/MP flags, B2 attack flags, command, and relic effects.
Use wide integers until the target's final base-damage cap.

For magic, with power P, level L and magic M:

```text
term = P when L=0, otherwise 4*P
base = term + floor(P*L*M / 32)
```

For physical:

```text
term = 4*P for a monster, otherwise P
adjusted = term if B2 bit 4000, otherwise ceil(7*term/4)
base = floor((adjusted + projected_strength) * L * L / 256)
for a character: base = ceil(3*base/2) + term
```

For characters using Fight (00) or Capture (06), Offering takes ceil(base/2),
then Genji Glove takes ceil(3*base/4). The reductions do not apply to other
commands, matching the patch's intended correction. These are arithmetic
inputs, not claims that those commands or relic actions execute in game yet.

Restorative attacks skip Atlas Armlet/earring damage boosts. Otherwise, physical
HP damage with Atlas Armlet adds floor(base/4). Magic and MP-affecting attacks
with one earring add floor(base/4), or with two add floor(base/2). Saturate to
65535 only at the end of this base calculation. Vanilla comments calling these
boosts +50%/+100% do not accurately express the shifts in the routine.

## Target modifiers

`damage.rs::Modifiers` ports C2/0C9E. It accepts an explicit RNG byte so the
caller must supply the real battle RNG later. The native host PRNG is not
substituted for the cartridge sequence.

- Variance uses `rng | E0`, a factor from 224 to 255. The source comment saying
  240–255 is inaccurate. MultDmg computes floor(value*factor/256)+1, not /255.
- HP physical attacks use defense/Safe; magic or MP-affecting attacks use
  magic defense/Shell. The ignore-defense flag bypasses this defense/status
  block. The Golem/Interceptor defense override is retained as an explicit
  `normal_defense` input, not implemented as a live summon system.
- The normal defense multiplier is 255-defense. Defense 255 still produces
  one at this stage because of MultDmg's +1. Safe/Shell multiply by AA hex.
- Physical row/defend flags halve damage; target Morph halves magic damage.
  Non-restorative character-to-character damage halves again.
- The attack multiplier adds floor(damage/2) per count, with source saturation
  and its B2/attack-flag ignore predicate. Disabled modification returns the
  unchanged base value.

This ends before elemental absorption/null/half/weakness, final 9999 cap,
drain/fractional/special effects, hit checks and application to current HP/MP.
Those remain unimplemented. Routines for complete combat execution are not
replaced with a synthetic turn-based victory path.

## Attack properties and opening diagnostics

MagicProp records contain targeting, element, three flag bytes, MP cost, power,
misc flags, hit rate, special effect and two status words (14 bytes total).
The clean serializer and 768 numeric fields are checked before target export.
Target changes are:

- C4/6FF4 / file 046FF4, attack 95 (`SUPLEX`), flag byte 2: 21 → 61 hex,
  adding NO_DMG_SPLIT. This agrees with the supplied Suplex damage-split note.
- C4/736B / file 04736B, attack 158 (`RIPPLER`), misc byte 7: 03 → 02,
  clearing STATUS_BLOCK. Broader Rippler fix behavior requires its changed
  routines; this single byte does not prove the entire fix.

Opening property/stat diagnostics evaluate attacks 131/132/133/135 (Fire,
Bolt, Ice beams and Heal Force) for Terra, Wedge and Biggs. This is not a
validated availability/menu-targeting list. For Terra, Fire Beam base damage
is 459. Against a normal Guard's magic defense 140, its modifier-stage value
is 181–206 across all supplied RNG bytes. Fire does not overlap the Guard's
poison weakness. HP subtraction and victory are not executed.

The 3072 opening modifier rows are controlled arithmetic contexts; Heal Force
is evaluated as a restorative value with ignore-defense, without asserting
that a Guard is a legal player-selected healing target.

## Validation and remaining work

```powershell
python tools/extract_native_magic.py
.\tools\cargo-local.ps1 test --offline
.\tools\cargo-local.ps1 build --release --offline
python tools/verify_damage.py
```

The verifier compares 8768 base-damage cases with actual patched instructions
(1,373,194 instructions), 4096 modifier cases (412,722 instructions), 12
opening base cases and 3072 opening modifier cases. Edge cases include level
zero, large products, command gates, MP/restoration flags and relic combinations.
The bounded checker models M/X widths, stack, direct page and HiROM code/data
addresses. Only the independently verified hardware multiply helper and an
explicitly supplied RNG return are hooked. It is not a generic SNES emulator.
The equipment verifier is rerun after shared checker changes.

Confidence is high for the sampled arithmetic and serialization paths. The
live story still pauses at battle group 1. Action queues, initial RNG/gauges,
AI, hit/element/status effects, complete command selection, battle presentation,
audio, rewards and event return remain incomplete. These checks do not prove
a complete battle or original runtime equivalence.

The bounded elemental stage and counter accumulation are now implemented separately; see [elemental-damage.md](elemental-damage.md). Current HP application and complete battle execution remain open.
