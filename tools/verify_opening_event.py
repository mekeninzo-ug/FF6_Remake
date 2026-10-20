#!/usr/bin/env python3
"""Re-encode the bounded opening entry from source macros and check native VM traces.

This is a small independent encoder for this source block, not a general assembler.
Unknown source constructs fail. Event bytecode is not CPU machine code.
"""
from pathlib import Path
import csv
import hashlib
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
ENTRY, END = 0x029A4F, 0x029AD5
OBJECTS = {'TERRA': 0, 'WEDGE': 14, 'VICKS': 15}
DIRECTIONS = {'UP': 0, 'RIGHT': 1, 'DOWN': 2, 'LEFT': 3}
SPEEDS = {'SLOWER': 0, 'SLOW': 1, 'NORMAL': 2, 'FAST': 3, 'FASTER': 4, 'FASTEST': 5}


def word(value: int) -> bytes:
    return value.to_bytes(2, 'little')


def object_command(line: str) -> list[bytes]:
    if line == 'end': return [b'\xff']
    if line == 'scroll_obj': return [b'\xd7']
    if line == 'hide_obj': return [b'\xd1']
    if match := re.fullmatch(r'pos \{(\d+), (\d+)\}', line):
        return [bytes([0xd5, int(match[1]), int(match[2])])]
    if match := re.fullmatch(r'dir (\w+)', line):
        return [bytes([0xcc + DIRECTIONS[match[1]]])]
    if match := re.fullmatch(r'speed (\w+)', line):
        return [bytes([0xc0 + SPEEDS[match[1]]])]
    if match := re.fullmatch(r'wait (\d+)', line):
        return [bytes([0xe0, int(match[1])])]
    if match := re.fullmatch(r'move (\w+), (\d+)', line):
        direction, remaining = DIRECTIONS[match[1]], int(match[2])
        assert 1 <= remaining <= 32
        encoded = []
        while remaining:
            count = min(8, remaining)
            encoded.append(bytes([0x80 | ((count-1) << 2) | direction]))
            remaining -= count
        return encoded
    raise ValueError(f'Unsupported source object command: {line}')


def event_command(line: str, songs: dict[str, int]) -> bytes:
    if line == 'wait_30f': return b'\x92'
    if line == 'sort_obj': return b'\x45'
    if line == 'unlock_camera': return b'\x39'
    if line == 'return': return b'\xfe'
    if match := re.fullmatch(r'load_map (\d+), \{(\d+), (\d+)\}, (\w+), \{ASYNC, NO_FADE_IN\}', line):
        return b'\x6b' + word(int(match[1]) | DIRECTIONS[match[4]] << 12) + bytes([int(match[2]), int(match[3]), 0x40])
    if match := re.fullmatch(r'fade_in_song (\w+), (\d+)', line):
        return bytes([0xf1, songs[match[1]], int(match[2])])
    if match := re.fullmatch(r'switch \$([0-9A-Fa-f]+)=(0|1)', line):
        index, value = int(match[1], 16), int(match[2])
        return bytes([0xd0 + 2*(index >> 8) + (1-value), index & 255])
    if line.startswith('spc_cmd '):
        values = [int(value, 16) for value in re.findall(r'\$([0-9A-Fa-f]+)', line)]
        assert len(values) == 3
        return bytes([0xf6, *values])
    if line.startswith('party_chars '):
        chars = [OBJECTS[name.strip()] for name in line.removeprefix('party_chars ').split(',')]
        return bytes([0x3c, *chars, *([255] * (4-len(chars)))])
    if match := re.fullmatch(r'(show_obj|pass_off|pass_on) (\w+)', line):
        return bytes([{'show_obj': 0x41, 'pass_off': 0x78, 'pass_on': 0x36}[match[1]], OBJECTS[match[2]]])
    if match := re.fullmatch(r'vehicle (\w+), \{MAGITEK, SHOW_RIDER\}', line):
        return bytes([0x44, OBJECTS[match[1]], 0x40 | 0x80])
    if match := re.fullmatch(r'give_gil (\d+)', line):
        return b'\x84' + word(int(match[1]))
    if match := re.fullmatch(r'fade_in (\d+)', line):
        return bytes([0x59, int(match[1])])
    if match := re.fullmatch(r'dlg DLG_(\d+)', line):
        return b'\x4b' + word(int(match[1]))
    raise ValueError(f'Unsupported source event command: {line}')


def main() -> None:
    source_path = ROOT / 'extracted/reference/ff6-disassembly-event/event_main.asm'
    source = source_path.read_text(encoding='utf-8').splitlines()
    first = source.index('_cc9a4f:')
    stop = source.index('_cc9ad5:', first)
    with (ROOT / 'extracted/twue-3.07/audio/music/track_index.csv').open(newline='', encoding='utf-8') as stream:
        songs = {row['source_label']: int(row['track_id'], 16) for row in csv.DictReader(stream)}
    assert songs['WIND'] == 0x39
    encoded = bytearray()
    expected_trace = []

    def emit(data: bytes, line_number: int, object_id=None):
        expected_trace.append({'offset': ENTRY + len(encoded), 'opcode': data[0],
                               'length': len(data), 'object': object_id, 'source_line': line_number})
        encoded.extend(data)

    pending = None
    object_lines = []
    for index in range(first+1, stop):
        line = source[index].split(';', 1)[0].strip()
        if not line: continue
        if pending:
            object_lines.extend((index+1, data) for data in object_command(line))
            if line == 'end':
                object_id, asynchronous, header_line = pending
                size = sum(len(data) for _, data in object_lines)
                assert 0 < size <= 127
                emit(bytes([object_id, size | (0 if asynchronous else 128)]), header_line)
                for line_number, data in object_lines: emit(data, line_number, object_id)
                pending, object_lines = None, []
        elif match := re.fullmatch(r'obj_script (\w+)(, ASYNC)?', line):
            pending = (OBJECTS[match[1]], bool(match[2]), index+1)
        else:
            emit(event_command(line, songs), index+1)
    assert pending is None
    assert len(encoded) == END-ENTRY == 134
    for flavor, filename in (('vanilla', 'Final Fantasy III (USA) [Unpatched].sfc'),
                             ('twue-3.07', 'Final Fantasy III (USA) [Patched].sfc')):
        exported = (ROOT / f'extracted/{flavor}/event/event_script.bin').read_bytes()
        assert exported[ENTRY:END] == encoded, f'{flavor}: source macro bytes differ'
        rom = (ROOT / filename).read_bytes()
        assert rom[0x0a0000 + ENTRY:0x0a0000 + END] == encoded, f'{flavor}: ROM bytes differ'

    output = ROOT / 'reverse-engineering/captures/native-opening-event'
    executable = ROOT / 'target/release/ff6-native.exe'
    subprocess.run([str(executable), '--trace-opening-event', str(output)], cwd=ROOT, check=True)
    with (output / 'commands.csv').open(newline='', encoding='utf-8') as stream:
        actual = list(csv.DictReader(stream))
    assert len(actual) == len(expected_trace)
    for observed, expected in zip(actual, expected_trace):
        pc = expected['offset']
        assert int(observed['event_offset'], 16) == pc
        assert int(observed['cpu'], 16) == 0xca0000 + pc
        assert int(observed['file_offset'], 16) == 0x0a0000 + pc
        assert int(observed['opcode'], 16) == expected['opcode']
        assert int(observed['length']) == expected['length']
        assert observed['object'] == ('' if expected['object'] is None else str(expected['object']))
        assert observed['scope'] == ('event' if expected['object'] is None else 'object')
    with (output / 'snapshots.csv').open(newline='', encoding='utf-8') as stream:
        snapshots = list(csv.DictReader(stream))
    assert [row['stage'] for row in snapshots] == ['dialogue_11', 'returned']
    for row, y in zip(snapshots, (50, 49)):
        assert (int(row['map']), int(row['gil']), row['party']) == (19, 3000, '0:14:15:255')
        assert (int(row['terra_x']), int(row['terra_y']), int(row['terra_vehicle'])) == (38, y, 192)
        assert row['switch_01cc'] == '0'
    assert snapshots[0]['switch_01c1'] == '0' and snapshots[1]['switch_01c1'] == '1'
    assert int(snapshots[1]['event_offset'], 16) == END
    assert (snapshots[1]['wedge_x'], snapshots[1]['wedge_y'], snapshots[1]['wedge_visible']) == ('38', '50', '0')
    assert (snapshots[1]['vicks_x'], snapshots[1]['vicks_y'], snapshots[1]['vicks_visible']) == ('38', '50', '0')
    requests = (output / 'requests.txt').read_text().splitlines()
    assert requests.count('WaitFrames(30)') == 2
    assert 'SongFadeIn { song: 57, duration: 160 }' in requests
    assert 'SpcCommand([130, 0, 255])' in requests
    report = {'source_commit': '813013276c952fdd27edcf7b8b86f17291542cbf',
              'source_sha256': hashlib.sha256(source_path.read_bytes()).hexdigest().upper(),
              'macro_sha256': hashlib.sha256((ROOT / 'extracted/reference/event_cmd.mac').read_bytes()).hexdigest().upper(),
              'object_interpreter_source_sha256': hashlib.sha256((ROOT / 'extracted/reference/ff6-disassembly-field/obj.asm').read_bytes()).hexdigest().upper(),
              'executable_sha256': hashlib.sha256(executable.read_bytes()).hexdigest().upper(),
              'entry_cpu': '0xCC9A4F', 'entry_file_offset': '0x0C9A4F',
              'encoded_bytes_compared_to_both_exports': len(encoded),
              'encoded_bytes_compared_to_both_roms': len(encoded),
              'native_instruction_records_checked': len(actual), 'return_event_offset': f'0x{END:06X}',
              'source_command_bindings': expected_trace, 'snapshots': snapshots,
              'retained_presentation_requests': requests,
              'validation_mode': 'Structural endpoint decoder; live scheduling is audited separately by verify_opening_timeline.py',
              'limitations': 'This structural pass executes object blocks sequentially to endpoints. It does not validate parallel timing, fades, audio or vehicle rendering.'}
    (ROOT / 'reverse-engineering/opening-event-validation.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')
    print(f'Opening event passed: 134 source-encoded bytes match both exports; {len(actual)} native event/object instructions match source boundaries.')


if __name__ == '__main__':
    main()
