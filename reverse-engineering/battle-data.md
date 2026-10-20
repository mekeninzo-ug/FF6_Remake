# Battle tables and initial arithmetic

Target: TWUE 3.07 + Bug Fixes. Main source is everything8215/ff6 commit
`813013276c952fdd27edcf7b8b86f17291542cbf`. Local source copies are in
`extracted/reference/ff6-disassembly-battle/`; filenames replace `/` with `--`.
This is static source/ROM evidence and native arithmetic comparison. No original
battle runtime or emulator capture has been obtained.

## Address map

Canonical HiROM bank mapping applies. These are data, not executable code.

| Table | CPU address | File offset | Length | Format | Target changed bytes |
|---|---|---|---|---|---|
| MonsterProp | CF/0000 | 0F0000 | 12288 | 384 records × 32 bytes | 0 |
| EventBattleGroup | CF/5000 | 0F5000 | 1024 | 256 groups × 2 little-endian u16 formations | 4 |
| BattleProp | CF/5900 | 0F5900 | 2304 | 576 records × 4 bytes | 4 |
| BattleMonsters | CF/6200 | 0F6200 | 8640 | 576 records × 15 bytes | 353 |

The upstream event-group comment says 512 bytes, but the extracted table is
1024 bytes. C0/A5A7 uses a four-byte stride. Its second word is selected when
the supplied RNG byte is >= C0; the native decoder accepts an explicit RNG
byte and does not implement or approximate that RNG yet. High-bit formation
selection and conditional battle replacements are not modeled.

`tools/extract_native_battle.py` verifies both full ROM SHA-256 identities,
checks 13 numeric fields/defaults for each of 384 clean source records (4992
checks), and exports target tables. It records every differing byte with both
CPU address and file offset in `build/native_battle/provenance.json`. Inputs
are read only. Native runtime loading never reads the ROM.

## Formation decoding

C2/30E8 loads BattleProp and XORs the low property byte with F0. It copies
15-byte BattleMonsters records to WRAM 7E/3F44. The source copies 16 bytes,
including the following record's first byte; the modeled fields use only 15.

| Record byte | Meaning / evidence |
|---|---|
| 0 | Formation graphics flags; retained raw, not interpreted here |
| 1 | Six present bits, slot 0 = bit 0; C2/2EE1 |
| 2–7 | Six low monster-ID bytes; C2/2F04 |
| 8–13 | Six packed graphics positions; retained raw, screen mapping unported |
| 14 | Six high monster-ID bits, slot 0 = bit 0 |

ID 1FF is empty. ID 0FF with its high bit clear is a real monster. Slots whose
present flag is clear may still have loaded properties; do not delete them.
`battle_data.rs` retains the raw records, loaded property flags, slot numbers,
presence, IDs, positions and starting HP/MP. It does not perform AI startup,
status/equipment initialization or special-case Doom Gaze HP restoration.

## MonsterProp record

`data/monster_prop.mac::end_monster_prop` defines:

| Offset | Contents |
|---|---|
| 00–07 | speed, attack, hit rate, evade, magic block, defense, magic defense, magic power |
| 08/0A/0C/0E | u16 HP / MP / experience / gil |
| 10–13 | level, metamorph, monster flags, monster status |
| 14–16 | three status-immunity bytes |
| 17–19 | elemental absorption, nullification, weakness |
| 1A | attack animation |
| 1B–1D | three initial status bytes |
| 1E–1F | retaliation flags, special attack |

Confidence: high for serialization and numeric base properties. Additional
derived combat fields and initial statuses require the remaining C2/2C30 path.
Names in the clean disassembly are labels, not necessarily target display text.

## First guard encounter

Event CC/9B71 requests group 1, flags 3F. CF/5004 contains `02 00 02 00`:
both selections resolve to formation 2. Its properties are `E3 00 00 00`,
loaded as `13 00 00 00`. Monster record:

```text
80 0C FF FF 00 00 FF FF 00 00 4B 87 00 00 33
```

Two Guards, monster 0, occupy slots 2 and 3. Each has HP 40, MP 15,
speed 30, level 5, base EXP 48 and base gil 48. These are base rewards,
not a claim about victory distribution. The event still pauses at its battle
request; no victory, rewards, event continuation or battle presentation is faked.

## Battle time arithmetic

`battle_time.rs` implements bounded primitives from C2/111B, C2/1193,
C2/11BB and C2/09D2. They are exercised by native diagnostic captures, not
connected to the live story yet.

- The eight-bit frame counter advances one battle tick after at least two
  frames. A long delay advances only one tick. Wait-mode program stops reset
  the previous frame and do not accumulate ticks. Turn count wraps at 16 bits.
- ATB uses a 16-bit counter. High byte zero is the ready sentinel. Overflow
  clears only the high counter byte and high advance-wait byte, preserving
  their low bytes, then sets advance duration FF. Action/menu queues are unported.
- Advance wait adds half the ATB increment. Overflow or reaching its high-byte
  duration requests enqueue. Caller eligibility/queued flags are unported.
- Speed calculation uses status constant 32 for slow, 84 for haste, otherwise
  64. Slow wins when both bits are present. Add 20 to speed with byte wrapping.
  For monsters multiply by `255 - 24*(battle_speed & 7)` and take the product's
  high byte. Multiply by 1.5 × status constant, then shift right four.

The source comment saying monster division by 255 is inaccurate: C2/4781
uses SNES unsigned 8×8 hardware multiplication and keeps the high byte (/256).
Another comment says haste 82; the immediate byte is 54 hex = 84.

## Reproduction and limits

```powershell
python tools/extract_native_battle.py
.\tools\cargo-local.ps1 build --release --offline
.\target\release\ff6-native.exe --inspect-first-battle
python tools/verify_battle_data.py
```

The verifier checks 1024 group selections, all 576 formations/1364 occupied
monster slots, 16384 speed cases and 512 diagnostic clock/gauge frames.
Speed cases execute the actual target C2/09D2 routine in a bounded A8/I8
instruction checker, hooking only the verified C2/4781 multiply helper.
This is not a general CPU emulator. Gauge/clock comparisons use an independent
arithmetic model plus Rust tests, not instruction-level or original runtime
execution. Random gauge setup now has target-instruction comparison in
[battle-rng.md](battle-rng.md). Character/equipment and damage arithmetic are
covered in their separate notes. Live commands, targeting, AI progression,
rewards, battle graphics, sound and event return remain incomplete.
