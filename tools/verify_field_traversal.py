#!/usr/bin/env python3
"""Validate native field input/exit captures against raw ROM records and asset renders."""
from __future__ import annotations

import csv
import hashlib
import json
from pathlib import Path
import subprocess
import sys

from PIL import Image, ImageChops, ImageDraw
from verify_field_entrances import TABLES

ROOT = Path(__file__).resolve().parents[1]


def read_csv(path: Path) -> list[dict]:
    with path.open(newline='', encoding='utf-8') as stream:
        return list(csv.DictReader(stream))


def exits_from_rom(rom: bytes, map_id: int):
    for name, table, _, _, size in reversed(TABLES):
        start = table + int.from_bytes(rom[table + 2*map_id:table + 2*map_id + 2], 'little')
        stop = table + int.from_bytes(rom[table + 2*map_id + 2:table + 2*map_id + 4], 'little')
        for pc in range(start, stop, size):
            record = rom[pc:pc+size]
            index = 3 if size == 7 else 2
            word = int.from_bytes(record[index:index+2], 'little')
            yield {'x': record[0], 'y': record[1], 'extent': record[2] & 127 if size == 7 else 0,
                   'vertical': size == 7 and bool(record[2] & 128), 'map': word & 511,
                   'dest_x': record[index+2], 'dest_y': record[index+3], 'facing': (word >> 12) & 3,
                   'pc': f'0x{pc:06X}'}


def main() -> None:
    executable = ROOT / 'target/release/ff6-native.exe'
    output = ROOT / 'reverse-engineering/captures/native-field-route'
    subprocess.run([str(executable), '--capture-field-route', str(output)], cwd=ROOT, check=True)
    rom = (ROOT / 'Final Fantasy III (USA) [Patched].sfc').read_bytes()
    frames = read_csv(output / 'state-trace.csv')
    assert [int(row['map']) for row in frames] == [19, 39, 41, 42, 41]
    previous = {'map': 19, 'x': 38, 'y': 49, 'total_steps': 0}
    transitions = []
    route = read_csv(output / 'route-steps.csv')
    for raw in route:
        current = {key: int(value) for key, value in raw.items()}
        assert current['total_steps'] == previous['total_steps'] + 1
        if current['map'] == previous['map']:
            assert abs(current['x'] - previous['x']) + abs(current['y'] - previous['y']) == 1
        else:
            matches = []
            for exit in exits_from_rom(rom, previous['map']):
                if (exit['map'], exit['dest_x'], exit['dest_y'], exit['facing']) != (
                        current['map'], current['x'], current['y'], current['facing']):
                    continue
                for dx, dy in ((0, -1), (1, 0), (0, 1), (-1, 0)):
                    x, y = previous['x'] + dx, previous['y'] + dy
                    in_trigger = (x == exit['x'] and exit['y'] <= y <= exit['y'] + exit['extent']) if exit['vertical'] else (
                        y == exit['y'] and exit['x'] <= x <= exit['x'] + exit['extent'])
                    if in_trigger:
                        matches.append(exit)
            assert matches, f'Transition has no matching patched ROM entrance: {previous} -> {current}'
            transitions.append({'from_map': previous['map'], 'to_map': current['map'],
                                'destination': [current['x'], current['y']], 'record_pc': matches[0]['pc']})
        previous = current
    assert len(transitions) == 4

    comparisons = []
    for frame in frames:
        name, map_id, x, y = frame['frame'], int(frame['map']), int(frame['x']), int(frame['y'])
        bundle = ROOT / f'extracted/twue-3.07/build/native_field/map-{map_id:04}'
        reference_path = output / f'{name}-background.png'
        subprocess.run([sys.executable, str(ROOT / 'tools/render_native_field.py'),
                        '--map', str(bundle), '--x', str(x), '--y', str(y), '--scale', '1',
                        '--output', str(reference_path)], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
        native = Image.open(output / f'{name}.bmp').convert('RGB')
        native.save(output / f'{name}.png')
        reference = Image.open(reference_path).convert('RGB')
        assert native.size == reference.size == (256, 224)
        metadata = (bundle / 'metadata.bin').read_bytes()
        width = max(64, int.from_bytes(metadata[9:11], 'little'))
        camera_x = min(max(x*16 + 8 - 128, 0), width*16 - 256)
        camera_y = min(max(y*16 + 8 - 112, 0), 64*16 - 224)
        actor_x, actor_y = x*16 - camera_x, y*16 - 30 - camera_y
        difference = ImageChops.difference(native, reference)
        ImageDraw.Draw(difference).rectangle((actor_x, actor_y, actor_x+31, actor_y+37), fill=(0,0,0))
        changed = sum(pixel != (0,0,0) for pixel in difference.getdata())
        assert changed == 0, f'{name}: {changed} background pixels differ'
        comparisons.append({'frame': name, 'map': map_id, 'position': [x,y], 'width': width,
                            'different_background_pixels': changed})
    report = {'executable_sha256': hashlib.sha256(executable.read_bytes()).hexdigest().upper(),
              'patched_rom_sha256': hashlib.sha256(rom).hexdigest().upper(),
              'successful_tile_steps': len(route), 'transitions': transitions, 'frame_comparisons': comparisons,
              'emulator_reference': False, 'story_events_enabled': False,
              'limitations': 'Explicit geometry-only mode disables story triggers. Static collision and direct sub-map exits only; story play stops at the unimplemented first guard battle. Startup scripts, fades, parent/world maps and bridge state are not validated.'}
    (ROOT / 'reverse-engineering/field-traversal-validation.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')
    print(f'Field traversal passed: {len(route)} actual input steps, four ROM-backed transitions, five exact background comparisons.')


if __name__ == '__main__':
    main()
