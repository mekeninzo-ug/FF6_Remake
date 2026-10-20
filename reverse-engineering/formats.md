# Asset and data formats

## Graphics and palettes

- Graphics exports use planar SNES tile encodings: `.1bpp`, `.2bpp`, `.3bpp`
  and `.4bpp`. For 2/4bpp tiles, each row's low/high bitplanes are interleaved
  in 16-byte plane pairs; 3bpp data adds a third plane at offset 16 within each
  24-byte tile. A tile is 8 × 8 pixels.
- Palette files are little-endian SNES BGR555 words. Bits 0–4 are red, 5–9
  green, and 10–14 blue; bit 15 is unused for the color value.
- `.scr` screen maps contain little-endian 16-bit tilemap words: 10-bit tile
  number, 3-bit palette, priority, horizontal flip and vertical flip.
- Field/world byte maps are extracted separately. Their IDs are not complete
  on-screen images without the matching graphics, palette, tile properties,
  scroll state, sprite priorities and animated layers.
- For MapProp record 21 (Narshe North Exterior), the three exported sub-tilemaps
  are 4,096-byte, 64 × 64 arrays of 8-bit 16 × 16 metatile IDs. Two 2,048-byte
  map tilesets provide four 256-entry little-endian SNES tile words per
  metatile. `LoadTileset` interleaves the two decompressed 0x400-byte planes
  before use; `LoadMapGfx` places four overlapping 0x2000-byte DMA windows in
  4bpp VRAM. BG3 has a 64-byte formation/palette table followed by 0x1000 bytes
  of 2bpp tiles. These details are implemented in
  `tools/extract_native_field.py`; the result is a local generated bundle under
  `extracted/twue-3.07/build/native_field/map-0021/`. It contains map assets,
  not a ROM copy.
- The MapProp table stores packed graphics, tileset and layout indices rather
  than direct IDs. The field renderer uses the patched MapProp record and the
  corresponding exported map/tileset/palette/tile-property assets. Map21's
  tile-property export is 512 bytes: `$7600`-style tile flags followed by
  `$7700`-style directional passability flags. Bridge z-level selection,
  actor collision and door changes are not fully implemented.
- `extracted/tools/render_graphics.py` was read before reuse. It generates
  grayscale planar tile sheets, BGR555 palette previews, and CSV maps with
  tile attributes. Its generic layout heuristic is for inspection, not a
  runtime renderer. Existing previews are kept alongside source bytes.
- Field actor frames are 16 × 24 pixels assembled from six 8 × 8 tiles. The
  upstream `MapSpriteTileOffsets` table in `src/field/sprite_data.asm` selects
  six byte offsets per frame; frames do not use a simple sequential tile range.
  The first nine walk-frame entries match uniquely at PC `0x00CE3A` in both
  supplied ROMs. The native renderer converts those byte offsets to 32-byte
  tile indices and loads the target's exported 4bpp tiles and BGR555 palette.
  `tools/verify_rom_assets.py` checks this ROM-to-source mapping. Full object
  palette selection, animation timing, sprite priority and all non-walk frames
  remain unimplemented.

The first map diagnostic can be reproduced with
`python tools/render_native_field.py --output narshe-north-entry.png`. It is a
render of decoded target assets, not an emulator capture. Quadrant placement
and parallax/priority details still need runtime comparison.

## Compression and expansion

Field bundle format v2 adds raw point/line entrance files and explicit logical
layer dimensions. Its 21-byte `metadata.bin` contains `F6MP`, version 2, a
little-endian map ID, BG3 foreground flag, tile-property ID, then three
little-endian `(u16 width,u16 height)` pairs. Dimensions come from MapProp
bytes 23/24; byte length alone is ambiguous. Maps 18/23 are 32×32; mines 41–43
are 128×64. See [field-entrances.md](field-entrances.md) for the dimension masks,
6-byte point and 7-byte line trigger formats, and zero-layout semantics.

- Upstream mapped compressed assets use FF6 LZSS. The source reference's
  `lz_list.txt` documents the known compressed spans; build exports retain
  `.lz` files.
- The patched image contains an additional 1 MiB (`$F0–$FF`). It is preserved
  raw. Only validated table layouts and high-confidence text candidates are
  decoded from it.
- The six non-LZSS battle-background spans and 21 invalid spans are retained
  as unclassified/invalid evidence; no guessed decoder output is presented as
  confirmed game graphics.

## Text

- English strings are decoded through the upstream U.S. character table files
  under `extracted/reference/char_table/`.
- `#text` labels preserve source names. `{n}`, `{0}`, `{0xNN}` and similar
  forms represent control bytes and must be interpreted by the UI/event layer.
- Expansion-bank item descriptions use a 256-entry pointer table at PC
  `0x310200`; decoded strings begin at PC `0x310400`. Additional address-tagged
  candidate strings are separate from confirmed table entries.
- Text exports are useful for names/dialogue, but event control flow is not
  fully represented by those text files.

## Audio

- `mfvitools` was pinned at commit `f675df1d312ca1729bbc0a95fe830740d3ac737d`.
  The existing extracts include 85 sequence-to-MML/SPC exports, 63 dynamic
  song samples, 71 static/SFX BRR samples, and 481 valid SFX channel files.
- BRR is the SNES APU's compressed sample format. The local extraction also
  has mono 32 kHz WAV previews; these are analysis outputs, not evidence of
  original sample-rate playback.
- The patched and clean ROM sequence/sample bytes compare identical. Timing,
  DSP interpolation, echo, pitch modulation, envelopes, and SPC scheduling
  remain required for accurate playback.

## Event scripts

The event script is interpreted bytecode, not 65816 instructions. The upstream
`event_cmd.mac` defines named command encodings and `ff6-disassembly-event/README.md`
documents the known subset. Dialogue, choices, map loads, actor movement,
inventory changes, battle starts, branches, and parallel object scripts are
commands in that interpreter. The source explicitly says the public event
disassembly is incomplete; unlabelled byte changes remain raw evidence.

## Native literal dialogue subset

`DialogueCatalog` reads numbered `#text:DLG_N` export sections. Physical CR/LF
in the export are formatting; `{n}` produces a displayed newline and `{0}`
terminates the string. Unknown or unterminated controls return an error. This
subset supports DLG_11; dynamic names, choices, waits and the full control
language remain open. It does not silently strip unknown commands.

## Field event tables and modeled VM saves

[Field event notes](field-events.md) map five-byte triggers, nine-byte NPC
records, initial NPC switches and startup pointers. Version 8 local saves use
checksummed `F6EV3` payloads for modeled VM/actor state and separate metadata
for dialogue, clocks and field motion. This is unrelated to cartridge SRAM.
Field display slots and actual character membership are distinct. The initial
soldier actors use palette 1, correcting the earlier provisional palette 2.
