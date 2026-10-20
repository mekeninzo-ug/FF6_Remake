# Magitek sprite composition

## Source and byte evidence

Pinned FF6 source commit `813013276c952fdd27edcf7b8b86f17291542cbf`:
`obj.asm::DrawMagitek`, `TfrVehicleGfx`, `ObjVehicleTileTbl`, and
`sprite_data.asm::MapSpriteTileOffsets`. Four 16-word formation tables match
uniquely in both ROMs at file `0x005EB6`, CPU `$C0:5EB6`. All 58 six-word
frame entries match at file `0x00CE3A`, CPU `$C0:CE3A`.

`tools/extract_native_sprites.py` verifies these source/ROM matches before
copying 128 formation bytes and 696 frame-offset bytes from the target into
`extracted/twue-3.07/build/native_sprites/`. Its manifest links every output
back to CPU/file locations. It also verifies the reused vehicle graphics and
palette exports against target bytes. No vanilla extractor is used here.

Vehicle data is at file `0x183000` / CPU `$D8:3000`; the export is 8192 bytes.
`TfrVehicleGfx` transfers only 7168 bytes to VRAM word `$7200`. With OBJ base
word `$6000`, this corresponds to OBJ tile `$120`. Formation low 9-bit tile
numbers minus `$120` select the local 4bpp tiles. Each 16x16 piece uses tile
indices `base, base+1, base+16, base+17`; horizontal/vertical flags flip the
entire piece. Palette bits select OBJ palette 7, whose BGR555 data is at file
`0x2680E0` / CPU `$E6:80E0`. These four mapped ranges are byte-identical in
clean and target ROMs. Patched Terra rider graphics are read from the target,
not substituted with clean art.

## Formation and animation

Four pieces form a 32x32 body. The chosen entry is `direction*4 + phase`, with
`phase = (coordinate >> 3) & 3`. DrawMagitek uses absolute object X when moving
horizontally and absolute Y otherwise, including while stationary. The top
pieces are at object Y-24 and bottom pieces at Y-8 in the native composition
contract. The rider sits at X+8, Y-30 plus the coordinate's one-pixel bob.

Rider DMA selects six sprite tiles, but Magitek displays only its upper 16x16
sprite (first four frame-offset words). The four gait slots use upward frames
4,5,4,3; downward 1,2,1,0; sideward $2E,$2F,$2E,$2F, flipped when facing right.
At rest the rider uses phase zero. Vehicle bit `$80` controls rider visibility;
vehicle kind `$40` selects Magitek. Bodies are composed before riders so the
rider wins overlaps within one object.

These positions derive from the source's OAM construction. SNES OAM Y and BG
scanline origins have subtle offsets; the native camera/frame origin is still
an approximation, pending original reference capture. See
[SNESdev OAM](https://snes.nesdev.org/wiki/OAM) for hardware context. This
project does not claim to emulate the PPU.

## Validation and limits

`verify_magitek.py` independently decodes target ROM planar graphics and
palette, source-verified tables and rider offsets, then compares two native
atlases. Four directions times four phases, with/without riders, give 32
configurations and 114,688 checked pixels. All match. The native game uses the
same compositor during the opening and after player control returns. Version
6 saves retain actor vehicle bytes; legacy completed-opening saves reconstruct
Terra's still-active Magitek flag. See `magitek-validation.json` and the native
opening motion captures.

Confidence is high for asset identity, formation words, color decoding and
piece/frame selection. Exact camera/PPU origin, background/sprite priority,
OAM overflow/dropout, DMA update latency and original runtime captures remain
unverified. General vehicle behavior, chocobo/raft rendering, the world-map
vehicle system are not complete. Player-controlled pixel movement is now
implemented; see [player-movement.md](player-movement.md).
