# Character properties and base progression

Primary source: everything8215/ff6 commit
`813013276c952fdd27edcf7b8b86f17291542cbf`, `field/char_prop.asm`,
`field/event.asm`, `event/event_main.asm::GameStart`, `data/level_up.asm`
and `cfg/ff6-en.cfg`. Local reference files are under `extracted/reference/`.
The patched ROM remains authoritative where its tables differ.

## Address and data map

| Table | CPU address | File offset | Size / format | Target byte changes |
|---|---|---|---|---|
| CharProp | ED/7CA0 | 2D7CA0 | 64 × 22 bytes | 0 |
| LevelUpExp | ED/8220 | 2D8220 | 98 × u16, little endian | 0 |
| LevelUpHP | E6/F4A0 | 26F4A0 | 98 × u8 | 0 |
| LevelUpMP | E6/F502 | 26F502 | 98 × u8 | 86 |

CharProp byte layout follows `end_char_prop`:

| Byte offset | Meaning |
|---|---|
| 00–01 | Base HP, MP |
| 02–05 | Four battle commands |
| 06–09 | Vigor, speed, stamina, magic power |
| 0A–0E | Base attack, defense, magic defense, evade, magic block |
| 0F–14 | Weapon, shield, helmet, armor, two relics |
| 15 | Run factor bits 0–1, level modifier bits 2–3, fixed equipment bit 4 |

These are character-property IDs, not party slots or sprite IDs. For example,
Wedge occupies party slot 14 but uses property 32 and sprite 14.

## Initialization and opening party

Event command 40 assigns properties, resets the esper to FF, computes the
average level from available character slots 0 through 13, applies a property
modifier (0, +2, +5, -3), and clamps to levels 1–99. With no available
characters the average is 3. Guest slots 14–15 do not enter that average.
Missing modeled data for a flagged available character fails explicitly.

For level L, `InitMaxHP` and `InitMaxMP` sum the first L-1 growth entries
and the base property byte. `UpdateExp` sums the first L-1 experience entries
and multiplies by eight. This is base initialization, without equipment,
esper bonuses or later stat modifications. The initialization routine may
produce base MP above 999 at high levels; later combat/menu clamping is not
modeled by this base calculation.

GameStart assigns Terra first, then sets switch 02F0 (bit 0 of the 1EDE
availability word), then assigns Wedge and Biggs. Exact target command-40
bindings are verified at CA/5E33–CA/5E8D.

| Party slot | Property ID / source label | Level | HP | MP | EXP |
|---|---|---|---|---|---|
| 0 | 0 / TERRA | 3 | 63 | 27 | 96 |
| 14 | 32 / WEDGE | 1 | 68 | 0 | 0 |
| 15 | 33 / VICKS (Biggs in target text) | 1 | 70 | 0 | 0 |

Terra's next level threshold is 208 total EXP, leaving 112 EXP. The native
menu now uses the target progression instead of its earlier fixed 100 value.

Confidence: high from source serialization, exact ROM tables, unchanged
initialization code regions and native checks; no original emulator runtime
capture is claimed. Source-aligned comparison regions include C0/9F78
(average level), C0/A07C (property assignment), C0/A23E (experience tail),
C0/A27E (HP initialization), and C0/A2BC (MP initialization). Exact region
lengths and digests are in extraction provenance; these are bounded regions,
not claims that every region boundary equals a function boundary.

## Patched MP growth

The target changes 86 of 98 bytes at E6/F502. It matches the Japanese source
table except the level-23 increment: source 15, target 14. All target bytes
are retained without replacement. The initial two increments are 5 and 6,
so Terra's MP is 16+5+6=27. The clean U.S. increments 4 and 4 give 24;
using them would reproduce the wrong version. Attribution to a specific patch
component and the reason for the level-23 byte remain unknown.

## Native state, saves and limitations

`characters.rs` implements base property assignment, initialization, available
party averaging and an initialized 16-slot roster. `Game` uses the target
opening roster. Existing field UI health/EXP scalar fields still project
Terra; save generation synchronizes that projection into her roster record.
The separate guest records survive save/load. Later field property-assignment
events are not connected to this module yet.

Local save version 9 adds a `characters` field containing a `F6CH1` payload:
16 fixed records with presence, property ID/raw properties, level, HP/maxHP,
MP/maxMP and 24-bit experience, plus FNV-1a checksum. Decoding checks lengths,
signature, checksum, IDs, levels and current HP/MP bounds. Version 9 requires
this payload. Earlier saves retain their scalar Terra state and reconstruct
opening guests; they cannot supply guest history they never recorded.
This local payload is not SNES SRAM. The existing general restore path is
not transactional if a later field fails validation.

Character names, statuses (including Magitek), natural magic learning,
full battle projection, stat bonuses, EXP distribution, level-up
effects and full party-menu presentation remain incomplete. Base command and
equipment IDs are retained. Equipment aggregation and read-only gear/status
pages are now covered separately in [equipment.md](equipment.md); commands
and equip/unequip interaction remain unimplemented.

## Validation

```powershell
python tools/extract_native_characters.py
.\tools\cargo-local.ps1 test --offline
.\tools\cargo-local.ps1 build --release --offline
python tools/verify_characters.py
```

The extractor verifies both complete ROM identities, 440 clean source numeric
character fields, 294 clean growth entries and all three opening bindings.
It records all table differences and unchanged initialization code regions in
`extracted/twue-3.07/build/native_characters/provenance.json`.

The verifier compares all 64 properties at levels 1–99 (6336 cases) with raw
target data, including base HP/MP/EXP, speed, commands and equipment. It also
checks the actual Game opening roster and a local save round trip with changed
Terra and guest HP. Captures include CSVs, save data and a native menu BMP.
These checks cover static-source base initialization, not full battle behavior.
