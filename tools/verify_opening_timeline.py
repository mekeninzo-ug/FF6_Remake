#!/usr/bin/env python3
"""Check live scheduler movement and rendered-game integration; not emulator timing."""
from pathlib import Path
import csv
import hashlib
import json
import subprocess
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]

def rows(path):
    with path.open(newline='', encoding='utf-8') as stream:
        return list(csv.DictReader(stream))

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()

def main():
    executable = ROOT / 'target/release/ff6-native.exe'
    subprocess.run(['python', str(ROOT / 'tools/verify_opening_event.py')], cwd=ROOT, check=True)
    timeline_dir = ROOT / 'reverse-engineering/captures/native-opening-timeline'
    motion_dir = ROOT / 'reverse-engineering/captures/native-opening-motion'
    subprocess.run([str(executable), '--trace-opening-timeline', str(timeline_dir)], cwd=ROOT, check=True)
    subprocess.run([str(executable), '--capture-opening-motion', str(motion_dir)], cwd=ROOT, check=True)
    timeline = rows(timeline_dir / 'timeline.csv')
    game = rows(motion_dir / 'game-timeline.csv')
    assert timeline == game, 'The actual game update loop differs from scheduler diagnostic'
    frames = {}
    for row in timeline:
        frames.setdefault(int(row['frame']), {})[int(row['object'])] = row
    assert list(frames) == list(range(1, max(frames)+1))
    source_report = json.loads((ROOT / 'reverse-engineering/opening-event-validation.json').read_text())
    expected = {entry['offset']: entry for entry in source_report['source_command_bindings']}
    commands = rows(timeline_dir / 'scheduled-commands.csv')
    seen = set()
    teleports = set()
    for index, command in enumerate(commands):
        pc = int(command['event_offset'], 16)
        entry = expected[pc]
        assert pc not in seen
        seen.add(pc)
        assert int(command['cpu'], 16) == pc + 0xca0000
        assert int(command['file_offset'], 16) == pc + 0x0a0000
        assert (int(command['opcode'], 16), int(command['length'])) == (entry['opcode'], entry['length'])
        assert command['object'] == ('' if entry['object'] is None else str(entry['object']))
        if entry['object'] is not None:
            assert int(command['frame']) % 4 == 0, 'All three active-party slots belong to phase zero'
            if entry['opcode'] in (0xd5, 0xd7): teleports.add((int(command['frame']), entry['object']))
        if entry['object'] is None and entry['opcode'] == 0x92:
            assert int(commands[index+1]['frame']) - int(command['frame']) == 30
        if entry['object'] is not None and entry['opcode'] == 0xe0:
            raw = (ROOT / 'extracted/twue-3.07/event/event_script.bin').read_bytes()
            following = next(item for item in commands[index+1:] if item['object'] == command['object'])
            assert int(following['frame']) - int(command['frame']) == (raw[pc+1]+1)*4
    skipped = sorted(set(expected) - seen)
    # Vicks' asynchronous direction-only block is replaced before its next
    # object update. Its two one-byte body commands are decoded but not run.
    assert skipped == [0x029a88, 0x029a89]
    assert [expected[pc]['opcode'] for pc in skipped] == [0xcc, 0xff]
    requests = (timeline_dir / 'scheduled-requests.txt').read_text()
    assert 'SupersededBlock { object: 15, next: 170632, end: 170634 }' in requests
    distance = {0: 0, 14: 0, 15: 0}
    overlap = 0
    for number, actors in frames.items():
        assert set(actors) == {0, 14, 15}
        moving = sum(int(row['offset_x']) != 0 or int(row['offset_y']) != 0 for row in actors.values())
        overlap += moving >= 2
        if number == 1: continue
        for actor_id, row in actors.items():
            previous = frames[number-1][actor_id]
            position = lambda r: (int(r['x'])*4096+int(r['offset_x']), int(r['y'])*4096+int(r['offset_y']))
            before, after = position(previous), position(row)
            dx, dy = after[0]-before[0], after[1]-before[1]
            if (number, actor_id) in teleports: continue
            assert abs(dx)+abs(dy) in (0, 256), (number, actor_id, dx, dy)
            distance[actor_id] += abs(dx)+abs(dy)
    assert distance == {0: 9*4096, 14: 11*4096, 15: 13*4096}
    assert overlap > 0
    dialogue_frames = [number for number, actors in frames.items() if actors[0]['yield'] == 'dialogue']
    assert len(dialogue_frames) == 1
    assert frames[dialogue_frames[0]][0]['y'] == '50'
    assert frames[max(frames)][0]['yield'] == 'returned'
    assert frames[max(frames)][0]['y'] == '49'
    captures = [motion_dir / name for name in (motion_dir / 'captures.txt').read_text().splitlines()]
    assert len(captures) >= 10
    chosen = [captures[3], captures[5], next(path for path in captures if 'dialogue' in path.name), captures[-1]]
    sheet = Image.new('RGB', (512, 480), '#151b25')
    draw = ImageDraw.Draw(sheet)
    for index, path in enumerate(chosen):
        x, y = (index % 2)*256, (index // 2)*240
        with Image.open(path) as image: sheet.paste(image, (x, y))
        draw.text((x+4, y+226), path.stem, fill='white')
    sheet.save(motion_dir / 'contact-sheet.png')
    report = {'executable_sha256': digest(executable), 'logical_frames': max(frames),
              'dialogue_frame': dialogue_frames[0], 'native_commands_checked': len(commands),
              'superseded_source_offsets': [f'0x{pc:06X}' for pc in skipped],
              'game_and_vm_actor_rows_equal': len(game), 'concurrent_movement_frames': overlap,
              'actor_distance_tiles': {str(k): v//4096 for k, v in distance.items()},
              'captures': {path.name: digest(path) for path in captures},
              'sources': {name: digest(ROOT / 'extracted/reference/ff6-disassembly-field' / name)
                          for name in ('obj.asm', 'event.asm', 'reset.asm', 'field_main.asm')},
              'scope': 'Native logical scheduling and game/render integration; no original emulator comparison.',
              'limitations': ['60 Hz clock approximates SNES refresh', 'Only active-party slot order modeled',
                             'No object collision, fades, audio or original DMA/sprite timing']}
    (ROOT / 'reverse-engineering/opening-timeline-validation.json').write_text(json.dumps(report, indent=2)+'\n')
    print(f"Opening timeline passed: {len(frames)} frames, {len(commands)} executed commands, {overlap} frames with concurrent movement; game and VM rows match.")

if __name__ == '__main__': main()
