#!/usr/bin/env python3
"""Audit entrance exports independently against both ROMs and their source maps."""
from pathlib import Path
import csv
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[1]
TABLES = (
    ('short_entrance', 0x1FBB00, 0x1FBF02, 0x1FD978, 6),
    ('long_entrance', 0x2DF480, 0x2DF882, 0x2DFCAA, 7),
)


def main() -> None:
    reports = []
    images = {}
    for flavor, filename in (
        ('vanilla', 'Final Fantasy III (USA) [Unpatched].sfc'),
        ('twue-3.07', 'Final Fantasy III (USA) [Patched].sfc'),
    ):
        rom = images[flavor] = (ROOT / filename).read_bytes()
        root = ROOT / 'extracted' / flavor
        with (root / 'provenance/source_asset_map.csv').open(newline='', encoding='utf-8-sig') as stream:
            provenance = list(csv.DictReader(stream))
        for name, table, first, end, size in TABLES:
            asset = f'assets/data/field/{name}.bin'
            data = (root / asset).read_bytes()
            assert data == rom[first:end], f'{flavor}: {name} bytes differ from ROM'
            offsets = {int(a): int(b) for a, b in re.findall(
                r'^_(\d+) = (\d+)', (root / asset).with_suffix('.ptr').read_text(), re.M)}
            records = {int(row['item_index']): row for row in provenance
                       if row['asset'] == asset and row['kind'] == 'array'}
            assert len(offsets) == len(records) == 512
            addresses = [table + int.from_bytes(rom[table + 2*i:table + 2*i + 2], 'little')
                         for i in range(513)]
            distinct = sorted(set(addresses))
            provenance_spans = {value: distinct[i+1] - value if i+1 < len(distinct) else 0
                                for i, value in enumerate(distinct)}
            for map_id in range(512):
                pointer = lambda i: table + int.from_bytes(rom[table + 2*i:table + 2*i + 2], 'little')
                start, stop = pointer(map_id), pointer(map_id + 1)
                assert first <= start <= stop <= end
                assert (stop - start) % size == 0
                assert offsets[map_id] == start - first
                row = records[map_id]
                assert int(row['source_pc'], 16) == start
                # The preserved export index describes each shared pointer's
                # span to the next distinct target. The field engine instead
                # uses adjacent pointers, so repeated pointers mean zero exits.
                assert int(row['byte_length']) == provenance_spans[start]
                assert data[offsets[map_id]:offsets[map_id] + stop-start] == rom[start:stop]
            assert pointer(512) == end
            if flavor == 'twue-3.07' and name == 'long_entrance':
                assert data[offsets[19]:offsets[20]] == bytes.fromhex('23 01 05 27 00 1a 34')
                assert data[offsets[39]:offsets[40]] == bytes.fromhex('1e 14 02 29 00 26 21 17 35 06 13 20 26 02')
            reports.append({'flavor': flavor, 'table': name, 'map_ranges_checked': 512,
                            'record_bytes': size, 'record_count': len(data)//size,
                            'pointer_table_pc': f'0x{table:06X}', 'data_pc': f'0x{first:06X}',
                            'provenance_span_semantics': 'next distinct target; runtime ranges use adjacent pointers',
                            'data_sha256': hashlib.sha256(data).hexdigest().upper()})
    changes = {}
    for name, table, first, end, size in TABLES:
        changes[name] = sum(a != b for a, b in zip(images['vanilla'][table:end], images['twue-3.07'][table:end]))
    report = {'tables': reports, 'changed_bytes_in_pointer_and_record_ranges': changes,
              'source_commit': '813013276c952fdd27edcf7b8b86f17291542cbf',
              'scope': 'Stored table identities, pointer ranges, record boundaries and provenance; not runtime event gating.'}
    (ROOT / 'reverse-engineering/entrances-audit.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')
    print('Entrance audit passed: 512 map ranges per table per ROM; 1,129 point and 152 line records per ROM.')
    print('Changed bytes:', changes)


if __name__ == '__main__':
    main()
