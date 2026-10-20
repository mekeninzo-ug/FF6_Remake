# FF6 ROM extraction and comparison report

## Verified inputs

Both ROMs matched the fingerprints supplied for the task:

| ROM | Bytes | CRC32 | SHA-256 |
|---|---:|---|---|
| Vanilla U.S. v1.0 | 3,145,728 | `A27F1C7A` | `0F51B4FCA41B7FD509E4B8F9D543151F68EFA5E97B08493E4B2A0C06F5D8D5E2` |
| TWUE v3.07 + Bug Fixes | 4,194,304 | `FB8A31B0` | `F385C715B3088D7D697171EC76B93B8F32623D63B3672A364121269A9E41DBB9` |

The IPS has 2,174 records. Expanding its normal and RLE records yields 1,805,587 written bytes; every one matches the already-patched ROM (0 mismatches). The patch was not applied during this work.

## Extraction method

The FF6 repository README documents `make rip` for the unmodified U.S. v1.0 ROM. I inspected its Makefile, `tools/extract_assets.py`, maps, and the `romtools` submodule before running anything. The Makefile's `setup` target forces a recursive `--remote --force` submodule update, so I did not run `make rip`. Instead, I ran the underlying extractor directly against a copy of the verified vanilla ROM using the checked-out repository commit `813013276c952fdd27edcf7b8b86f17291542cbf` and its pinned `romtools` commit `f023d6b2ebdafd5a23a79264a9a3775ff5ea537e`.

The upstream extractor recognizes only the Japanese 1.0, U.S. 1.0, and U.S. 1.1 CRCs and uses fixed vanilla ranges; it does not support the patched 4 MiB layout. I did not run it on the patched ROM. I used its U.S. asset definitions and ROMTools decoders against the patched bytes, updated relocated battle-dialog and monster-dialog ranges, handled the moved/duplicated battle-background pointers separately, and preserved all additional `$F0–$FF` banks as raw files. The patched dump therefore comes from the patched ROM itself; it is not a copy of vanilla assets.

## What is in the ROM folders

Each folder has 30 U.S. text assets, 1,430 graphics/palette/tilemap assets, 535 data assets, and 71 BRR samples. `provenance/source_asset_map.csv` gives source PC and CPU addresses, text labels, item numbers, and extraction status.

Graphics remain in the decoded planar formats from the upstream extractor. PNG previews render 1bpp–4bpp tiles, SNES BGR555 palettes, `.scr` tilemaps, and 353 field/world tilemaps; corresponding tile IDs and tile attributes are in CSV. The preview renderer records 3 vanilla and 2 patched files it could not tile-decode; original bytes remain available. A separate unclassified gallery shows raw battle-background pointer spans whose lengths do not validate as LZSS.

Text assets retain their `#text` labels and explicit control-code spellings such as `{n}`, `{0}`, and `{0xNN}`. The character maps used for these spellings are under `reference/char_table/`. The patched expansion contains a 256-entry item-description pointer table at PC `0x310200`, decoded item descriptions beginning at PC `0x310400`, and 202 additional address-tagged text candidates. The raw 1 MiB expansion is in `twue-3.07/expanded/`; F2/FC candidates and any unclassified bytes should be checked against their raw bank files before treating heuristic decodes as final text.

The full event-script slice is preserved at PC `0x0A0000–0x0CE5FF` in both folders. The source label index is from the upstream partial disassembly. The patched slice has 23 changed byte runs totaling 249 bytes. The disassembly itself documents that only a small part of the event script is fully reverse-engineered, so changes without a nearby source label remain raw bytes.

`mfvitools` at commit `f675df1d312ca1729bbc0a95fe830740d3ac737d` extracted all 85 music sequences to MML and SPC, with track addresses and names in `audio/music/track_index.csv`. The ROM sequence bytes, normalized MML, and generated SPC files match for all 85 tracks. The 63 dynamic song samples, 71 static/SFX BRR files, and both SFX pointer/sequence regions also match between ROMs. BRR files were additionally decoded to mono 32 kHz WAV previews. The 512 SFX channel pointers produced 481 valid channel sequence files; empty/out-of-range pointers remain identified in `audio/sfx/sequence_index.csv`. mfvitools labels its SPC builder experimental, so the extracted SPC files are data outputs, not playback-validated recordings.

## Confirmed ROM differences

The mapped asset comparison contains 239 changed files: 27 text assets, 205 graphics assets, and 7 data assets. No mapped output asset is missing from either side after the duplicate battle-background table entry was retained on both sides. Full per-file byte counts and sizes are in `asset_diff.csv`; decoded text is in `text_changes.diff`.

Directly visible examples include changed story, battle, and monster dialogue; item, monster, and action names; the `Hard cider` dialogue edits; the title-opening graphic; many monster and map-sprite graphics; battle-background graphics; and the World of Ruin world tilemap. The separate expanded-bank item descriptions confirm the extended descriptions are stored in the patched ROM.

## Pointer exceptions and boundaries

The upstream battle-background table has 75 entries. Forty-eight resolve and decode as LZSS with valid pointer-span lengths. Six more point at non-LZSS spans; their bytes and provisional 4bpp tile views are preserved under `provenance/unclassified_pointer_data/` and `previews/unclassified/` without presenting them as confirmed decoded assets. Twenty-one entries point into zero-filled or otherwise invalid LZSS regions in both ROMs; their addresses, span lengths, and hashes are recorded in the source map. No source definition establishes them as actual graphics, so they are not silently replaced with vanilla data.

The high-bank dump retains every byte from `$F0:0000` through `$FF:FFFF`. Only confirmed table structures and readable strings are decoded; other content remains raw for future mapping.

