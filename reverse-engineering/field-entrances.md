# Field entrances and layer geometry

Authority: supplied clean and patched ROMs, preserved exports, and vanilla FF6
disassembly commit `813013276c952fdd27edcf7b8b86f17291542cbf`. The local source
copies are under `extracted/reference/ff6-disassembly-field/`. Interpretation is
source-backed; original runtime behavior has not been captured in an emulator.

## Entrance tables

| Table | CPU pointer table / PC | CPU data / PC | Records |
|---|---|---|---|
| Point (`short_entrance`) | `$DF:BB00` / `0x1FBB00` | `$DF:BF02–$DF:D977` / `0x1FBF02–0x1FD977` | 1,129 × 6 bytes |
| Line (`long_entrance`) | `$ED:F480` / `0x2DF480` | `$ED:F882–$ED:FCA9` / `0x2DF882–0x2DFCA9` | 152 × 7 bytes |

Each table has 512 little-endian offsets plus a final end offset. Offsets are
relative to the pointer table, not the data block. Map `m` uses the half-open
range between pointers `m` and `m+1`. Equal adjacent pointers mean zero exits.
The preserved `.ptr` files normalize these offsets to the data-block start.
The provenance CSV instead records a shared target's span to the next distinct
target; that length must not be mistaken for an empty map's runtime record range.

`verify_field_entrances.py` checks every pointer, record boundary, export byte
and provenance address in both ROMs. Both pointer-plus-data ranges compare
byte-identical between clean and patched images. Confidence: high for stored
bytes, addresses and format; source-based for execution semantics.

| Record | Byte offsets |
|---|---|
| Point, 6 bytes | source X 0; source Y 1; destination map/flags word 2–3; destination X 4; destination Y 5 |
| Line, 7 bytes | source X 0; source Y 1; extent/orientation 2; map/flags word 3–4; destination X 5; destination Y 6 |

Line byte 2 bit 7 selects vertical; bits 0–6 are an inclusive coordinate extent.
An extent of five covers six tiles. The map/flags word stores map ID in bits
0–8, set-parent in bit 9, upper-layer selection in bit 10, show-title in bit 11,
and facing in bits 12–13 (up/right/down/left = 0/1/2/3). Remaining bits are
preserved. Map IDs 0–2 select world maps; `$1FF` restores the parent map.

The source checks line triggers before point triggers, and suppresses exits
while events, menus, map loads or battles are active, between tile boundaries,
or under specific bridge-state conditions. The native slice invokes direct
exits after successful field movement. General event dispatch, bridge state,
fades, title display and startup events are still absent. World/parent exits
and records requesting parent-map saves are retained but not executed.

## Source-backed native route

| Transition | Trigger | Destination | Record CPU / PC |
|---|---|---|---|
| 19 → 39 | X 35–40, Y 1 | `(26,52)`, up | `$ED:F8A5` / `0x2DF8A5` |
| 39 → 41 | X 30–32, Y 20 | `(38,33)`, up | `$ED:F923` / `0x2DF923` |
| 41 → 42 | X 41–43, Y 4 | `(86,28)`, up | `$ED:F931` / `0x2DF931` |
| 42 → 41 | `(86,29)` | `(42,6)`, down | `$DF:C328` / `0x1FC328` |

The compiled diagnostic plans a route through current static collision, then
feeds ordinary field inputs through the runtime. It records 121 successful
steps from the opening control-return state and checks every map change
against the patched ROM records. Five captured backgrounds match the Python
asset renderer outside the actor rectangle. These results do not validate
missing encounters, event gating or original presentation.

## Layer dimensions: correction to the earlier compact-layout inference

`scroll.asm::InitScrollClip` uses the table `$0F,$1F,$3F,$7F` to derive dimensions
16, 32, 64 and 128. MapProp byte 23 holds BG2 height in bits 0–1, BG2 width in
bits 2–3, BG1 height in bits 4–5 and BG1 width in bits 6–7. Byte 24 bits 4–5
hold BG3 height and bits 6–7 hold BG3 width. Width/height are therefore
`16 << encoded_pair`. Source label `@17E7` is not yet binary-mapped here.

Maps 18 and 23 use 32×32 layers. Their 1,024-byte files were previously decoded
as 16×64 by assuming 64 rows; that inference was wrong. Mines 41–43 use
128×64 layers. Runtime format v2 stores all six dimensions explicitly in
metadata bytes 9–20 as three little-endian `(width,height)` pairs.

The loader clears 0x4000 physical WRAM bytes for layout ID zero. The bundle
stores only the active logical zero grid, e.g. 1,024 bytes for map 18 BG3 and
4,096 bytes for map 244 BG3. It does not consume the placeholder `none.bin`.
Horizontal and vertical mask wrapping are implemented. Per-map camera limits
in MapProp bytes 30/31 and original parallax remain unported; logical coordinate
bounds for compact layers are still an implementation approximation.

Sources: pinned [entrance.asm](https://github.com/everything8215/ff6/blob/813013276c952fdd27edcf7b8b86f17291542cbf/src/field/entrance.asm),
[record definitions](https://github.com/everything8215/ff6/blob/813013276c952fdd27edcf7b8b86f17291542cbf/src/field/long_entrance.inc),
[scroll.asm](https://github.com/everything8215/ff6/blob/813013276c952fdd27edcf7b8b86f17291542cbf/src/field/scroll.asm),
and [map.asm](https://github.com/everything8215/ff6/blob/813013276c952fdd27edcf7b8b86f17291542cbf/src/field/map.asm).
