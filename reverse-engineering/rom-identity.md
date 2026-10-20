# ROM identity and mapping

## Supplied images

| Role | Bytes | CRC32 | SHA-256 |
|---|---:|---|---|
| Clean Final Fantasy III (USA) v1.0 | 3,145,728 (`0x300000`) | `A27F1C7A` | `0F51B4FCA41B7FD509E4B8F9D543151F68EFA5E97B08493E4B2A0C06F5D8D5E2` |
| Target TWUE v3.07 + Bug Fixes | 4,194,304 (`0x400000`) | `FB8A31B0` | `F385C715B3088D7D697171EC76B93B8F32623D63B3672A364121269A9E41DBB9` |

The hashes above were computed from the local files and match the user-supplied
fingerprints. The IPS has 2,174 records; expanding its normal and RLE records
in memory produces 1,805,587 addressed bytes, all of which equal the target
ROM bytes. No patch was applied or either ROM modified. See
`extracted/comparison/rom_verification.json`.

## Header and cartridge map

At PC offset `0x00FFC0`, both files contain the cartridge header. The title
field still says `FINAL FANTASY 3` in the patched image; the title-screen
uncensoring is a graphics/content change and does not rename the header. The
map-mode byte at `0x00FFD5` is `0x31` (HiROM with FastROM enabled). The target
header otherwise retains the U.S. cartridge metadata; its ROM-size byte is
`0x0C`.

For canonical full-bank HiROM addresses in `$C0:0000–$FF:FFFF`:

```text
PC = ((bank - 0xC0) * 0x10000) + address
CPU = (bank << 16) | address
```

This mapping yields `$C0:FFC0` at PC `0x00FFC0`, `$CA:0000` at PC `0x0A0000`,
and `$D1:F000` at PC `0x11F000`. It applies to the canonical high-bank window;
HiROM also exposes mirrors in lower banks and the `$40–$7D` area. A mirrored
CPU address must be normalized before converting it to a file offset.

| Image range | Canonical CPU banks | Notes |
|---|---|---|
| Clean `0x000000–0x2FFFFF` | `$C0–$EF` | 48 × 64 KiB banks. |
| Patched `0x000000–0x2FFFFF` | `$C0–$EF` | Original address region; bytes may be relocated or changed by the patch. |
| Patched `0x300000–0x3FFFFF` | `$F0–$FF` | Added 1 MiB expansion banks. Preserve as target data; do not decode with vanilla-only pointers. |

The clean ROM is 3 MiB, while the cartridge header's size code represents a
4 MiB capacity class. The patch fills the additional 1 MiB address space; the
existing export retains these banks raw at `extracted/twue-3.07/expanded/`.

## SNES execution context

The game is a 65C816 program, not a flat 6502 binary. The CPU has 24-bit
bank:address addressing, emulation/native modes, accumulator/index width flags,
direct page and data bank registers, long calls/returns, and memory-mapped PPU,
DMA, controller and audio ports. `$21xx` PPU registers and `$2140–$2143` SPC
ports are hardware interfaces, not ordinary RAM. Bank transitions and register
width state are needed when translating routines. The upstream disassembly is
the authority for named routines; raw ROM bytes without labels remain
unclassified until a valid structure is demonstrated.

## Provenance

- Local ROM hashes and sizes: `extracted/comparison/rom_verification.json`.
- Per-asset source PC/CPU locations and status:
  `extracted/vanilla/provenance/source_asset_map.csv` and
  `extracted/twue-3.07/provenance/source_asset_map.csv`.
- The source map follows the upstream FF6 U.S. asset definitions. The patch
  moves/duplicates some data and extends banks, so it is not a universal
  address translator for arbitrary target bytes.
