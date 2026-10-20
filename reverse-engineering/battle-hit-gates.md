# CheckHit gates and special accuracy modes

## Mapping and evidence

Primary structure: pinned `battle--check_hit.asm`. Native
`src/battle_hit_gates.rs` follows actual target C2/220D..22B4 and
C2/22EC..22FA, with stamina helper C2/239C..23BE. File offsets are the CPU
addresses minus C00000. Prefix and stamina helper bytes match clean ROM.
This establishes the local routines, not every caller or patched wrapper.

## Gate order

1. B3 bit 80 plus target Vanish selects physical miss or magical hit before
   protection checks. Flags2 bit 40 instead queues Vanish removal and continues.
2. Reflect status rejects into the reflection path unless flags1 bit 02 bypasses it.
3. Instant-death protection and undead eligibility follow. Eligibility uses
   the XOR of target status1 and monster flags, preserving their high bits.
4. Fight/Capture with no special effect, one target, and a character target
   can invoke Interceptor then Golem. Interceptor consumes one RNG byte and
   tests its low bit. If it fails, existing Golem HP can still force a block.
5. Flags2 bit 20 hits automatically. Bit 40 selects level divisibility. Bit
   10 selects magic-evasion followed by stamina resistance. Otherwise the
   caller proceeds to ordinary accuracy, documented in battle-accuracy.md.

Level mode divides target level by hit-rate byte and uses the hardware
remainder. Zero divisor preserves the dividend low byte as remainder; the
native code handles this explicitly. Stamina mode first compares RandA(100)
against floor(MagicEvade * hit rate / 256). Only if that succeeds does it
consume another RNG byte and compare stamina against its low seven bits.

Returned routes distinguish ordinary accuracy, hit, miss requiring evasion
processing, direct miss, and reflect. Vanish removal and forced block bits
are separate outputs. A Miss is not a final action outcome: C2/22B5 can still
select evasion graphics, Golem or Interceptor effects, or rewrite status-only
attack behavior. These are not yet implemented. No victory or damage is
implied by this component.

## Validation

`python tools/verify_battle_hit_gates.py` compares 32,768 controlled cases
against 944,696 target instructions. It compares route, queued Vanish
removal, forced block and final RNG index. All five routes are exercised.
Actual RNG, prefix and stamina instructions execute; separately byte-checked
hardware multiplication/division helpers are modeled. Traces stop before
ordinary accuracy, miss processing or reflection continuation. Captures are
local under captures/native-battle-hit-gates, with a durable JSON report.
Confidence is high for this bounded behavior. Full original runtime behavior,
miss presentation and the complete battle action remain unverified.

Miss processing and stage integration now exist in [battle-hit.md](battle-hit.md); reflection continuation and rendering remain outside CheckHit.
