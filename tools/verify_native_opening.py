#!/usr/bin/env python3
"""Capture the Rust opening and compare field pixels with the Python asset renderer.

This checks implementation consistency. Neither renderer is an emulator reference.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path
import subprocess
import sys

from PIL import Image, ImageChops, ImageDraw

ROOT = Path(__file__).resolve().parents[1]


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path, default=Path('target/release/ff6-native.exe'))
    parser.add_argument('--output', type=Path, default=Path('reverse-engineering/captures/native-opening'))
    args = parser.parse_args()
    executable = (ROOT / args.executable).resolve()
    output = (ROOT / args.output).resolve()
    subprocess.run([str(executable), '--capture-opening', str(output)], cwd=ROOT, check=True)

    trace_path = output / 'state-trace.csv'
    with trace_path.open(newline='', encoding='utf-8') as stream:
        trace = list(csv.DictReader(stream))
    assert len(trace) == 8, 'Expected title, four lore cards, two dialogues and player control'
    assert [int(row['render_map']) for row in trace[1:5]] == [23, 20, 244, 18]
    for row, phase, y in ((trace[6], 2, 50), (trace[7], 3, 49)):
        assert (int(row['render_map']), int(row['x']), int(row['y'])) == (19, 38, y)
        assert int(row['opening_phase']) == phase
        assert int(row['gil']) == 3000

    reference_path = output / 'map-0019-background.png'
    subprocess.run([
        sys.executable, str(ROOT / 'tools/render_native_field.py'),
        '--map', str(ROOT / 'extracted/twue-3.07/build/native_field/map-0019'),
        '--x', '38', '--y', '49', '--scale', '1', '--output', str(reference_path),
    ], cwd=ROOT, check=True)
    native_path = output / '07-player-control.bmp'
    native = Image.open(native_path).convert('RGB')
    native.save(output / '07-player-control.png')
    reference = Image.open(reference_path).convert('RGB')
    assert native.size == reference.size == (256, 224)
    difference = ImageChops.difference(native, reference)

    # The independent asset renderer has no actors. Exclude only Terra's
    # 32x38 Magitek+rider bound, computed from the sprite composition contract.
    camera_x = min(max(38 * 16 + 8 - 128, 0), 64 * 16 - 256)
    camera_y = min(max(49 * 16 + 8 - 112, 0), 64 * 16 - 224)
    actor_x, actor_y = 38 * 16 - camera_x, 49 * 16 - 30 - camera_y
    actor_box = (actor_x, actor_y, actor_x + 32, actor_y + 38)
    actor_changed = sum(pixel != (0, 0, 0) for pixel in difference.crop(actor_box).getdata())
    ImageDraw.Draw(difference).rectangle(
        (actor_x, actor_y, actor_x + 31, actor_y + 37), fill=(0, 0, 0))
    outside_changed = sum(pixel != (0, 0, 0) for pixel in difference.getdata())
    assert outside_changed == 0, f'{outside_changed} background pixels differ'
    assert actor_changed > 0, 'Expected a visible actor in the native capture'

    report = {
        'comparison': 'native Rust renderer versus independent Python asset renderer',
        'executable_sha256': digest(executable),
        'state_trace_sha256': digest(trace_path),
        'native_frame_sha256': digest(native_path),
        'map_manifest_sha256': digest(ROOT / 'extracted/twue-3.07/build/native_field/map-0019/manifest.json'),
        'map_id': 19,
        'position': [38, 49],
        'resolution': [256, 224],
        'excluded_actor_rectangle': actor_box,
        'different_pixels_outside_actor': outside_changed,
        'actor_pixels_different_from_background': actor_changed,
        'opening_states_checked': len(trace),
        'emulator_reference': False,
        'limitations': 'Does not establish original PPU priority, event timing, actor fidelity or game-wide behavior.',
    }
    report_path = ROOT / 'reverse-engineering/native-render-comparison.json'
    report_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(f'Native opening OK: {len(trace)} states; {outside_changed} differing background pixels; {actor_changed} actor pixels')


if __name__ == '__main__':
    main()
