# Equipment calculations and target item properties

Main evidence: pinned FF6 disassembly commit
`813013276c952fdd27edcf7b8b86f17291542cbf`, `battle/equip.asm`,
`battle/init_attacker.asm`, `field/obj.asm`, `data/item_prop.asm` and
`data/item_prop.mac`. The verified patched ROM supplies all runtime records.

## Address map

| Region | CPU | File offset | Size / purpose |
|---|---|---|---|
| ItemProp | D8/5000 | 185000 | 7680 bytes, 256 × 30-byte records |
| UpdateEquip | C2/0E77 | 020E77 | Character stat initialization and six-item aggregation |
| CalcEquipEffect | C2/0F9A | 020F9A | One item, status flags, numeric bonuses and hand effects |
| EquipEvadeTbl | C2/1105 | 021105 | 11 signed little-endian u16 entries |
| Field UpdateEquip | C0/6CF3 | 006CF3 | 51-byte loop through 16 enabled active-party members |
| UpdateEquipBattle | C2/286D | 02286D | Subsequent battle projection, not fully ported |
| MultAB | C2/4781 | 024781 | Unsigned SNES hardware 8×8 multiplication helper |

The entire equipment region C2/0E77–C2/111A and field loop are byte-identical
between clean and target images. This includes the evade table. ItemProp has
one target difference: D8/562C / file 18562C, item 52 (`MITHRIL_ROD`), byte
14 hex (decimal 20), power 60 → 61. Attribution to a specific patch component
is unknown. No source or input ROM is modified.

## Item record

| Offset | Contents |
|---|---|
| 00 | Item type (low three bits), usage flags |
| 01–02 | Equipment eligibility and flags, including Imp equipment bit 6 of byte 02 |
| 03–04 | Learned spell rate / spell ID |
| 05 | Field effects, including Sprint Shoes bit 20 hex |
| 06–07 | Status protection |
| 08 | Equipped status 3 |
| 09–0D | Five relic-effect bytes |
| 0E | Targeting |
| 0F | Weapon element or nonweapon elemental half damage |
| 10–11 | Four signed magnitude nibbles: vigor, speed, stamina, magic |
| 12–13 | Cast-spell properties / item flags or weapon effects |
| 14 | Attack or defense power; consumable power in consumable records |
| 15 | Weapon hit rate or nonweapon magic defense |
| 16–18 | Elemental absorption, nullification, weakness |
| 19 | Equipped status 2 |
| 1A | Evade / magic block table indices |
| 1B | Block animation/weapon special effect |
| 1C–1D | Little-endian price |

For consumables, offsets 15–18 instead contain two status words; offset 1B
is a consumable special effect. The extractor validates this distinction.
The native equipment calculator accepts arbitrary item/slot combinations for
comparison, including combinations the real menu would forbid. Equipment
eligibility and inventory editing are not implemented by that calculation.

## Native calculation

`equipment.rs` implements UpdateEquip arithmetic and retains the source's
11A0–11DF result layout as a 64-byte calculation output. This is a native
stat calculation, not an emulated CPU or runtime memory bus.

- Start with CharProp base power, defenses, evasion and run factor; current
  modeled vigor/speed/stamina/magic come from the character record.
- Apply six equipment records in reverse slot order. OR field/status/relic
  effects; accumulate signed magnitude stat modifiers and evade-table values.
  Nibble 8 represents negative zero, not -8. Clamp low stat bytes after all
  additions, preserving the source's high-byte behavior in the raw result.
- Item defense/magic defense saturate at 255. Normal items supply power 1
  under Imp; Imp-only equipment supplies its actual power only under Imp.
- Weapon effects apply only in the two hand slots. Weapon/empty hand masks
  choose which hand's attack power becomes zero and when to clear two-handed
  weapon flags. Dual wield also clears both two-handed flags and emits its
  source effect mask.
- Preserve earring stacking, Hyper Wrist vigor arithmetic, block animation
  masks and HP/MP boost flag priorities. These are flags; boosted HP/MP totals,
  final battle status immunity and command behavior are separate work.

`battle_strength()` models the later doubled-vigor byte projection (capped at
255). The rest of UpdateEquipBattle, damage, targeting, relic command effects,
status changes and special weapon actions remain incomplete.

## Opening party results

| Slot | Vigor | Speed | Stamina | Magic | Defense | Magic defense | Evade | Magic block | Right / left power |
|---|---|---|---|---|---|---|---|---|---|
| 0 / Terra | 31 | 33 | 28 | 39 | 97 | 69 | 15 | 7 | 42 / 0 |
| 14 / Wedge | 40 | 35 | 46 | 29 | 132 | 86 | 25 | 0 | 62 / 0 |
| 15 / Biggs | 41 | 36 | 45 | 28 | 134 | 86 | 22 | 0 | 65 / 0 |

The native Equipment page now displays the six actual Terra slots with target
item-name text. Status displays the calculated stats. These are read-only
pages with approximate native layout/font; no original menu reproduction or
working equip/unequip interaction is claimed. Item icon tokens are recognized
but not rendered; internal `{pad}` becomes a space (e.g. Kaiser Claws).

Field effects OR across enabled members of the active party, independently
of field display slots or visibility, following C0/6CF3. Sprint Shoes item
230 now supplies the patched field speed flag. Cancel plus Sprint Shoes selects
speed index 4. Opening characters do not own Sprint Shoes; tests assign them
to exercise membership, movement and snapshot paths. No gear is granted in play.
The character snapshot's equipment bytes preserve assigned gear, including
the test guest's Sprint Shoes. Equipment-derived stats are recomputed.

## Validation and confidence

```powershell
python tools/extract_native_items.py
.\tools\cargo-local.ps1 test --offline
.\tools\cargo-local.ps1 build --release --offline
python tools/verify_equipment.py
```

The extractor verifies both complete ROM SHA-256 identities, 1763 numeric
clean-source item fields, evade-table entries, unchanged routines and the one
item-data difference. Provenance is under `build/native_items/provenance.json`.

The verifier executes the actual target C2/0E77 routine in a bounded 65816
instruction checker with width flags, native stack, bank addressing and only
the verified multiply helper hooked. It compares all 64 output bytes and
HP/MP boost flags with Rust for 3080 cases: three opening properties, every
item in every slot under normal and Imp status, and selected combinations.
The run executes 3,870,629 target instructions. It also checks three actual
Game roster rows. Rust tests exercise Sprint Shoes membership, Cancel/dash
combination, mid-movement saves and target item-name parsing. Both native menu
BMPs were visually inspected.

Confidence is high for these bounded calculations. No complete battle execution,
original menu capture, PPU, SRAM or sound behavior is validated by this checker.
Additional relic combinations, transformed status handling and altered current
stats from esper bonuses need further coverage as those systems are implemented.
