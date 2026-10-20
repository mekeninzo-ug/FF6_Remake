#!/usr/bin/env python3
"""Build field sprite lookup data from verified target bytes; ROMs are read-only."""
from pathlib import Path
import hashlib
import json
import re
import zlib
ROOT = Path(__file__).resolve().parents[1]
LABELS = ('MagitekTopLeftTiles','MagitekTopRightTiles','MagitekBtmLeftTiles','MagitekBtmRightTiles')

def source_words():
    text = (ROOT / 'extracted/reference/ff6-disassembly-field/obj.asm').read_text()
    words = []
    for label in LABELS:
        lines = [line for line in text.split(label+':', 1)[1].splitlines() if '.word' in line][:4]
        values = [int(value, 16) for line in lines for value in re.findall(r'\$([0-9a-f]+)', line.split(';')[0])]
        assert len(values) == 16
        words.extend(values)
    return b''.join(value.to_bytes(2, 'little') for value in words)

def frame_words():
    text = (ROOT / 'extracted/reference/ff6-disassembly-field/sprite_data.asm').read_text()
    block = text.split('MapSpriteTileOffsets:', 1)[1].split('MapSpriteGfxPtrsLo:', 1)[0]
    values = [int(value,16) for line in block.splitlines() if '.word' in line
              for value in re.findall(r'\$([0-9a-f]+)', line.split(';')[0])]
    assert len(values) == 58*6
    return b''.join(value.to_bytes(2, 'little') for value in values)

def main():
    target = (ROOT / 'Final Fantasy III (USA) [Patched].sfc').read_bytes()
    assert len(target) == 4194304 and zlib.crc32(target) == 0xfb8a31b0
    clean = (ROOT / 'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert len(clean) == 3145728 and zlib.crc32(clean) == 0xa27f1c7a
    formations, frames = source_words(), frame_words()
    for rom in (clean, target):
        assert rom[0x5eb6:0x5f36] == formations and rom.count(formations) == 1
        assert rom[0xce3a:0xd0f2] == frames
    output = ROOT / 'extracted/twue-3.07/build/native_sprites'
    output.mkdir(parents=True, exist_ok=True)
    records = []
    for filename, start, end in (('magitek_tiles.bin', 0x5eb6, 0x5f36), ('frame_offsets.bin', 0xce3a, 0xd0f2)):
        raw = target[start:end]
        (output / filename).write_bytes(raw)
        records.append({'file': filename, 'file_offset': f'0x{start:06X}', 'cpu': f'0x{start+0xc00000:06X}',
                        'length': len(raw), 'sha256': hashlib.sha256(raw).hexdigest().upper()})
    for asset, start, size in (('vehicle.4bpp', 0x183000, 0x2000), ('map_sprite_pal/vehicle.pal', 0x2680e0, 32)):
        assert (ROOT / 'extracted/twue-3.07/assets/gfx' / asset).read_bytes() == target[start:start+size]
    manifest = {'target_sha256': hashlib.sha256(target).hexdigest().upper(),
                'source_commit': '813013276c952fdd27edcf7b8b86f17291542cbf', 'records': records,
                'vehicle_dma': {'file_offset': '0x183000', 'cpu': '0xD83000', 'vram_word': '0x7200', 'length': 0x1c00},
                'sprite_base_vram_word': '0x6000', 'vehicle_first_obj_tile': '0x120',
                'vehicle_palette': {'file_offset': '0x2680E0', 'cpu': '0xE680E0', 'index': 7},
                'notes': 'Target exports and lookup bytes checked against ROM; source tables also match clean ROM. No vanilla extractor was run on expanded ROM.'}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
    print('Native sprite bundle: 128 Magitek formation bytes and 696 frame-offset bytes verified against source and both ROMs.')
if __name__ == '__main__': main()
