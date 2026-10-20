#!/usr/bin/env python3
"""Compare native Magitek composition with independent raw-ROM pixel decoding."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys
from PIL import Image, ImageChops
from extract_native_sprites import source_words, frame_words
ROOT = Path(__file__).resolve().parents[1]

def tiles(data):
    output = []
    for start in range(0,len(data),32):
        tile = data[start:start+32]
        output.append([sum(((tile[(plane//2)*16+y*2+plane%2] >> (7-x))&1)<<plane
                           for plane in range(4)) for y in range(8) for x in range(8)])
    return output

def palette(data):
    return [tuple(((int.from_bytes(data[i:i+2],'little') >> shift)&31)*255//31
                  for shift in (0,5,10)) for i in range(0,32,2)]

def blit(image, sheet, colors, selected, x, y, horizontal=False, vertical=False):
    for oy in range(16):
        for ox in range(16):
            sx, sy = (15-ox if horizontal else ox), (15-oy if vertical else oy)
            value = sheet[selected[(sy//8)*2+sx//8]][(sy%8)*8+sx%8]
            if value and 0 <= x+ox < 256 and 0 <= y+oy < 224:
                image.putpixel((x+ox,y+oy), colors[value])

def main():
    subprocess.run([sys.executable,str(ROOT/'tools/extract_native_sprites.py')],cwd=ROOT,check=True)
    executable = ROOT/'target/release/ff6-native.exe'
    output = ROOT/'reverse-engineering/captures/native-magitek'
    subprocess.run([str(executable),'--capture-magitek',str(output)],cwd=ROOT,check=True)
    target = (ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes()
    raw_words, raw_frames = source_words(),frame_words()
    assert target[0x5eb6:0x5f36] == raw_words
    assert target[0xce3a:0xd0f2] == raw_frames
    words = [int.from_bytes(raw_words[i:i+2],'little') for i in range(0,128,2)]
    frames = [int.from_bytes(raw_frames[i:i+2],'little')//32 for i in range(0,696,2)]
    vehicle = tiles(target[0x183000:0x184c00])  # only the actual $1c00-byte DMA
    vehicle_colors = palette(target[0x2680e0:0x268100])
    terra_raw = target[0x150000:0x1516a0]
    assert (ROOT/'extracted/twue-3.07/assets/gfx/map_sprite_gfx/terra.4bpp').read_bytes() == terra_raw
    terra = tiles(terra_raw)
    terra_colors = palette(target[0x268040:0x268060])
    comparisons = []
    for show, name in ((False,'vehicles'),(True,'riders')):
        expected = Image.new('RGB',(256,224),'black')
        for direction in range(4):
            for phase in range(4):
                x,y = phase*64+16,direction*56+43
                for piece in range(4):
                    word = words[piece*16+direction*4+phase]
                    base = (word&0x1ff)-0x120
                    blit(expected,vehicle,vehicle_colors,[base,base+1,base+16,base+17],
                         x+(piece%2)*16,y-24+(piece//2)*16,bool(word&0x4000),bool(word&0x8000))
                if show:
                    frame_id = ([4,5,4,3] if direction==0 else [1,2,1,0] if direction==2 else [0x2e,0x2f,0x2e,0x2f])[phase]
                    selected = frames[frame_id*6:frame_id*6+4]
                    blit(expected,terra,terra_colors,selected,x+8,y-30+(phase&1),direction==1)
        native = Image.open(output/f'{name}.bmp').convert('RGB')
        changed = sum(pixel != (0,0,0) for pixel in ImageChops.difference(native,expected).getdata())
        assert changed == 0, (name,changed)
        native.save(output/f'{name}.png')
        expected.save(output/f'{name}-reference.png')
        comparisons.append({'atlas':name,'configurations':16,'pixels_compared':256*224,'different_pixels':changed,
                            'native_sha256':hashlib.sha256((output/f'{name}.bmp').read_bytes()).hexdigest().upper()})
    clean = (ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    ranges = {'magitek_formations':(0x5eb6,128),'sprite_frame_offsets':(0xce3a,696),
              'vehicle_export':(0x183000,0x2000),'vehicle_palette':(0x2680e0,32)}
    differences = {name:sum(a!=b for a,b in zip(clean[start:start+size],target[start:start+size]))
                   for name,(start,size) in ranges.items()}
    report = {'executable_sha256':hashlib.sha256(executable.read_bytes()).hexdigest().upper(),
              'comparison':'Native Rust composition versus independent raw-ROM planar decoder',
              'atlases':comparisons,'clean_target_changed_bytes':differences,
              'sprite_origin':'Object X/Y, vehicle top at X,Y-24; rider at X+8,Y-30+coordinate bob',
              'source_labels':['DrawMagitek','TfrVehicleGfx','ObjVehicleTileTbl','MapSpriteTileOffsets'],
              'emulator_reference':False,
              'limitations':'Pixel composition checked; original camera/PPU origin, hardware priorities, OAM overflow, DMA animation latency and runtime timing remain unverified.'}
    (ROOT/'reverse-engineering/magitek-validation.json').write_text(json.dumps(report,indent=2)+'\n')
    print('Magitek passed: 32 configurations / 114,688 pixels match independent raw-ROM decoding.')
if __name__ == '__main__':main()
