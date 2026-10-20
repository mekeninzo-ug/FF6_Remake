# Post-action Condemned, Quetzalli and near-fatal relic effects

## Sources and address map

The native `src/battle_after_effects.rs` translates these pinned
battle--action_1.asm routines. CPU addresses are HiROM CPU bank C2;
file offsets omit C0/0000 from the CPU address:

| Routine | CPU addresses | File offsets |
| --- | --- | --- |
| StartCondemn | C2/09B4..09CD | 0x209B4..0x209CD |
| StopCondemn | C2/09CE..09D1 | 0x209CE..0x209D1 |
| NearFatalEffect | C2/0A4A..0A90 | 0x20A4A..0x20A90 |
| QuetzEffect | C2/0B4A..0B82 | 0x20B4A..0x20B82 |

These regions are identical in clean and patched ROMs. Labels derive from
source; byte execution is the behavior authority. Confidence is high within
these routines. StopCondemn is a zero-byte store to the selected counter;
it is documented here but has no separate native wrapper yet.

## Condemned

StartCondemn uses attacker level at 7E/11AF, not necessarily the selected
actor's level. It consumes one RNG byte even at level zero, scales the byte
by level with the SNES 8x8 multiply, then adds level with eight-bit wrapping.
It subtracts that value from 60. Underflow produces 20; otherwise the retained
carry makes the final ADC produce 81 minus the wrapped sum. Thus level zero
produces 81, and a sum of 60 produces 21. Counter RAM is 7E/3B05+actor*2.
Do not replace this with a generic clamped 20..80 formula.

## Quetzalli

The handler sets the actor's target-mask bit in 7E/3F2C, changes actor flags
at 7E/3AA0 using `(flags & 0x9B) | 8`, removes matching actor offsets from
execution queue 7E/3820, clears update-high bit 7 (7E/3205), clears only the
high advance-counter byte (7E/3AB5), and sets duration 7E/322C to 224.
It retains command-pool records. Queue scanning is the source's unsigned
CPY/BCC do-while: equal head/tail still scans; wrapped intervals are not
normalized into an ordinary ring iterator. Heads/tails remain unchanged.

## Near-fatal relics

If flags0 bit 0x10 is clear, NearFatal status2 bit 2 is set, and update-high
bit 2 is set, the handler consumes that update bit once. RelicEffect5 bits
0, 1 and 2 append Shell (37), Safe (28), Reflect (36), in that order, to the
shared immediate command list using command 38 and the actor offset as
attack byte. B8 low is overwritten with the spell ID; its previous high
byte is retained. Command storage consumes B1 bit 0x40 and caches zero MP
cost. The consumed near-fatal bit stays cleared even if no relic fires.
The handler creates commands; it does not apply their spell effects.

## Comparison evidence and integration limits

`python tools/verify_battle_after_effects.py` checks every level/RNG-index
pair (65,536 cases), 2,048 Quetzalli cases and 4,096 near-fatal cases against
actual target 65816 code: 3,823,134 instructions. It includes RNG lookup,
command allocation and immediate payload storage with no routine hooks.
Hardware multiply register arithmetic is modeled; CPU cycle timing is not.
The comparison covers queue bytes, flags, timers, all command links and
payload bytes, B8/B1, countdown and RNG index. Traces and CSV fixtures live
locally under captures/native-battle-after-effects. See
[battle-after-effects-validation.json](battle-after-effects-validation.json).

Three focused Rust tests cover carry boundaries, retained stored commands,
and one-shot relic ordering. The full AfterAction2 sequence and a live battle
are not connected yet. Countdown ticking, Jump presentation and relic spell
execution are separate integration requirements.
