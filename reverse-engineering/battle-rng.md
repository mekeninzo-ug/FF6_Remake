# Battle RNG, initial gauges and Guard attack selection

Source: verified clean/target ROMs and FF6 disassembly commit
`813013276c952fdd27edcf7b8b86f17291542cbf`, specifically `battle/rand.asm`,
`battle/bit_math.asm`, `battle/init.asm`, `battle/ai_cmd.asm`,
`battle/ai_script.asm` and `field/rng_tbl.asm`. Target bytes remain authoritative.

## Data and code addresses

| Region | CPU | File offset | Format / finding |
|---|---|---|---|
| RNGTbl | C0/FD00 | 00FD00 | 256 bytes, a permutation of 0–255; unchanged |
| Battle seed | C2/2440 | 022440 | Game-time frame byte at 021E, shifted left twice |
| Rand | C2/4B5A | 024B5A | Increment byte BE, then read RNGTbl[index] |
| RandA | C2/4B65 | 024B65 | Multiply next value by A; return high product byte |
| GetBitNum | C2/51F0 | 0251F0 | Index of highest set bit; zero returns zero |
| CountBits | C2/520E | 02520E | Popcount used by RandBit |
| RandBit | C2/522A | 02522A | Choose an ascending set-bit rank using RandA(popcount) |
| InitGauge | C2/2575–C2/2601 | 022575–022601 | Initial ten-slot gauge/rank calculation |
| AIScriptPtrs | CF/8400 | 0F8400 | 384 little-endian u16 offsets from CF/8700 |
| AIScript | CF/8700 | 0F8700 | Fixed 3950-hex-byte script region, not 65816 code |
| InitAI pointer load | C2/2D71 | 022D71 | Confirms pointer table base remains valid |
| GetNextAICmd | C2/1A5F | 021A5F | Confirms target still fetches from CF/8700 |
| AIRand3 | C2/1A80 | 021A80 | RandA(3), then select one of three script operands |

The listed RNG, bit, gauge and AI fetch/selection code regions are unchanged
between clean and patched images. Code/data addressing is HiROM; execution
uses 65816 M/X widths, direct page and native stack behavior. AI scripts are
interpreted game data and are not disassembled as CPU code.

AI pointer bytes change at 717 positions; script bytes change at 13,770
positions in the fixed region. These are **raw layout differences**, not counts
of changed AI behaviors. Pointer relocation, script lengths/order and normalized
per-monster differences still require analysis. All 384 target pointers remain
inside the region; observed maximum offsets are 3946 hex clean / 3947 target.
Extraction preserves the target layout without assuming vanilla pointer equality.

## RNG semantics

`battle_rng.rs` implements:

```text
seed = (game_time_frames_byte * 4) & FF
next(): index = (index + 1) & FF; return table[index]
below(bound): return floor(next() * bound / 256)
bit(mask): choose ascending set-bit rank below(popcount(mask))
```

Bound 0 and bound 1 still consume one value. Empty bit masks return zero
without consuming a value. A one-bit mask consumes a value. The index wraps
after 256 draws; frame seeding produces 64 possible initial indices. The table
is not shuffled or replaced by a host PRNG. `BattleRng.index` is explicit so
future battle snapshots can preserve it.

The game-time frame producer and complete battle-initialization call sequence
are not connected to live play yet. Do not seed InitGauge directly from game
time and assume it is equivalent: earlier initialization steps also consume
random values. Native diagnostics supply the routine's entry index explicitly.
RandCarry and weighted RandBitWithRate are not yet exposed as native operations.

The main battle loop also increments BE at C2/0019 / file 020019, without a
table read. `advance_index()` exposes this operation separately from drawing
a value. The scheduler does not invent a mapping between its ATB updates and
main-loop iterations; that live call ordering must be preserved by the future
battle runner.

## Initial gauges

The native function takes ten actor records (present, speed, current 16-bit
gauge) and explicit battle type/flags, enemy-character mask and scrolling state.
These are routine inputs, not defaults asserted for every battle.

1. Count absent targets and form the source's 16-per-absent bonus.
2. Iterate slots 9 down to 0. Consume a unique random rank from all ten bits,
   even for absent targets or a slot whose gauge will not be rewritten.
3. Only a gauge with high byte FF and scrolling state FF is rewritten.
   Its low byte is preserved. No additional presence gate is invented.
4. Normal characters in preemptive or side attacks retain their FF high byte.
   Enemy slots receive high byte 02 in those cases. An enemy-controlled
   character follows the enemy path.
5. Characters in back/pincer attacks use rank*8+1. Other normal cases draw
   RandA(speed), then add speed, rank*8, absent bonus and 1, saturating at FF.
   The original carry from battle-flags bit 7 is preserved for normal characters.

Slots 0–3 are battle characters, slots 4–9 are monsters. This is separate
from the field party display slots. Actual ATB incrementing and wait-mode
arithmetic were ported earlier in `battle_time.rs`; action/menu queues and
complete battle timing remain unimplemented.

## Guard AI selection

The Guard's target script starts at CF/8700:

```text
F0 EE EE EF FF FF
use one of [BATTLE, BATTLE, SPECIAL]; end; empty retaliation end
```

These six bytes match the clean image and disassembly. `battle_ai.rs` loads
target pointers/scripts and implements first-choice selection for direct attacks,
F0 triples and FF end. Selecting FE requests script continuation, matching
the handler's meaning; it is not silently treated as end of turn. Conditional
and other unsupported opcodes fail with the target script address.

For all 256 possible entry indices, Guard chooses EE normal attack 171 times
and EF special 85 times. RandA(3) has discrete group sizes 86/85/85, so the
source's informal one-third comments are not exact probabilities. Subsequent
choices are coupled to RNG values consumed elsewhere; they are not independent
host-random trials. The selector does not yet execute attacks, target choice,
special effects, conditions, retaliation or AI cursor/queue progression.

## Validation and limits

```powershell
python tools/extract_native_battle.py
.\tools\cargo-local.ps1 test --offline
.\tools\cargo-local.ps1 build --release --offline
python tools/verify_battle_rng.py
```

The extractor verifies both full ROM identities, table ranges, every AI pointer,
Guard bytes and unchanged code regions. Full byte differences stay local in
`extracted/twue-3.07/build/native_battle/provenance.json`; a compact durable
table/code map is included in `battle-rng-validation.json`.

The verifier compares native output with bounded execution of actual target
instructions for 65,536 RandA inputs, 2,048 random-bit cases, 1,024 gauge setups
and 256 Guard choices. It also compares 2,560 sequential RNG rows with raw
table reads. RNG/bit checks execute 1,110,495 target instructions; gauge checks
execute 1,570,032. Four gauge instruction/state traces retain CPU/file addresses,
registers, width flags and stack pointer. Only the verified hardware multiply
helper is hooked; RNG and bit-selection instructions execute from the target.

Confidence is high for these bounded operations. Native tests also cover zero
bounds/masks, wrapping, skipped gauges, preemptive behavior and unsupported AI
opcodes. The shared equipment/damage instruction checkers are rerun for regressions.
No original battle runtime, PPU or audio capture is claimed. Live story still
pauses at the first battle request; full RNG call ordering, AI and battle execution
must be connected before claiming a playable battle.
