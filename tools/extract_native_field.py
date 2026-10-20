#!/usr/bin/env python3
"""Assemble FF6 field maps' SNES-style runtime assets from local exports.

ROM input is read-only. The output contains only one map's graphics and data,
not a copy of the ROM. `--map-id` addresses the field MapProp table.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
from pathlib import Path


MAP_GFX_DEST_WORDS = (0x0000, 0x1000, 0x1800, 0x2000)
GFX_DMA_BYTES = 0x2000
VRAM_BYTES = 0x8000
DEFAULT_MAP_IDS = (18, 19, 20, 21, 23, 39, 41, 42, 43, 244)
ENTRANCE_TABLES = {
    'short_entrance': (0x1FBB00, 0x1FBF02, 0x1FD978, 6),
    'long_entrance': (0x2DF480, 0x2DF882, 0x2DFCAA, 7),
}


def u16(data: bytes, offset: int) -> int:
    return data[offset] | (data[offset + 1] << 8)


def read_entrances(root: Path, rom: bytes, name: str, map_id: int) -> tuple[bytes, dict]:
    table_pc, data_pc, end_pc, record_size = ENTRANCE_TABLES[name]
    source = root / 'assets/data/field' / f'{name}.bin'
    data = source.read_bytes()
    if data != rom[data_pc:end_pc]:
        raise ValueError(f'{name}: exported records do not match the selected ROM')
    exported_offsets = {int(a): int(b) for a, b in re.findall(
        r'^_(\d+) = (\d+)', source.with_suffix('.ptr').read_text(), re.MULTILINE)}
    offsets = [u16(rom, table_pc + index * 2) - (data_pc - table_pc) for index in range(513)]
    if len(exported_offsets) != 512 or offsets[-1] != len(data):
        raise ValueError(f'{name}: pointer count or end pointer differs from the source format')
    for index in range(512):
        if exported_offsets[index] != offsets[index]:
            raise ValueError(f'{name}: normalized pointer {index} differs from ROM')
        if not 0 <= offsets[index] <= offsets[index + 1] <= len(data):
            raise ValueError(f'{name}: invalid pointer range {index}')
        if (offsets[index + 1] - offsets[index]) % record_size:
            raise ValueError(f'{name}: partial record for map {index}')
    start, stop = offsets[map_id:map_id + 2]
    pc = data_pc + start
    return data[start:stop], {
        'asset': source.relative_to(root).as_posix(),
        'pc': f'0x{pc:06X}',
        'cpu': f'${0xC00000 + pc:06X}',
        'source_length': stop - start,
        'record_bytes': record_size,
        'pointer_table_pc': f'0x{table_pc:06X}',
    }


def read_map(root: Path, rom_path: Path, map_id: int) -> tuple[dict, dict[str, bytes]]:
    if not 0 <= map_id < 512:
        raise ValueError('Map index must fit the 512-entry entrance pointer tables')
    rom = rom_path.read_bytes()
    csv_path = root / "provenance" / "source_asset_map.csv"
    with csv_path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    def entry(asset_prefix: str, item_index: int) -> dict:
        matches = [
            row
            for row in rows
            if row["asset"].startswith(asset_prefix)
            and row["kind"] == "data"
            and int(row["item_index"]) == item_index
            and row["status"] == "ok"
        ]
        if len(matches) != 1:
            raise ValueError(f"Expected one mapped asset for {asset_prefix} index {item_index}; found {len(matches)}")
        return matches[0]

    map_prop_row = entry("assets/data/field/map_prop.bin", 0)
    map_prop = (root / map_prop_row["asset"]).read_bytes()
    offset = map_id * 33
    if offset + 33 > len(map_prop):
        raise ValueError(f"Map property index {map_id} is outside the exported table")
    prop = map_prop[offset : offset + 33]
    word = lambda pos: u16(prop, pos)

    graphics_ids = [
        prop[7] & 0x7F,
        ((word(7) << 1) & 0x7F00) >> 8,
        ((word(8) << 2) & 0x7F00) >> 8,
        ((word(9) << 3) & 0x7F00) >> 8,
    ]
    bg3_gfx_id = (word(10) >> 4) & 0x3F
    tileset_ids = [(word(11) >> 2) & 0x7F, (word(12) >> 1) & 0x7F]
    layout_ids = [word(13) & 0x03FF, (word(14) >> 2) & 0x03FF, (word(15) >> 4) & 0x03FF]
    palette_id = prop[25]
    # scroll.asm::InitScrollClip uses MapProp bytes 23/24, not the
    # decompressed byte count, to choose each layer's horizontal/vertical mask.
    layer_dimensions = [
        [16 << ((prop[23] >> 6) & 3), 16 << ((prop[23] >> 4) & 3)],
        [16 << ((prop[23] >> 2) & 3), 16 << (prop[23] & 3)],
        [16 << ((prop[24] >> 6) & 3), 16 << ((prop[24] >> 4) & 3)],
    ]

    files: dict[str, bytes] = {}
    addresses: dict[str, dict[str, str | int]] = {}

    def read_export(key: str, prefix: str, index: int) -> bytes:
        row = entry(prefix, index)
        path = root / row["asset"]
        files[key] = path.read_bytes()
        addresses[key] = {
            "asset": row["asset"],
            "pc": row["source_pc"],
            "cpu": row["source_cpu"],
            "source_length": int(row["byte_length"]),
        }
        return files[key]

    def read_layout(key: str, layout_id: int, dimensions: list[int]) -> bytes:
        logical_bytes = dimensions[0] * dimensions[1]
        if layout_id == 0:
            # LoadMapTiles treats a zero layout ID as a clear operation and
            # clears 0x4000 physical WRAM bytes. Store the active logical
            # width/height here, rather than loading the `none.bin` placeholder.
            files[key] = bytes(logical_bytes)
            addresses[key] = {
                "layout_id": 0,
                "runtime_bytes": logical_bytes,
                "cleared_wram_bytes": 0x4000,
                "source": "zero-filled by LoadMapTiles for layout ID 0",
            }
            return files[key]
        data = read_export(key, "assets/data/field/sub_tilemap/", layout_id)
        if len(data) != logical_bytes:
            raise ValueError(f'{key}: exported length {len(data)} differs from MapProp dimensions {dimensions}')
        return data

    bg1 = read_layout("bg1_map", layout_ids[0], layer_dimensions[0])
    bg2 = read_layout("bg2_map", layout_ids[1], layer_dimensions[1])
    bg3 = read_layout("bg3_map", layout_ids[2], layer_dimensions[2])
    tileset1 = read_export("bg1_tileset", "assets/data/field/map_tileset/", tileset_ids[0])
    tileset2 = read_export("bg2_tileset", "assets/data/field/map_tileset/", tileset_ids[1])
    tile_properties = read_export("tile_properties", "assets/data/field/map_tile_prop/", prop[4])
    palette = read_export("palette", "assets/gfx/map_pal/", palette_id)
    bg3_graphics = read_export("bg3_graphics", "assets/gfx/map_gfx_bg3/", bg3_gfx_id)

    for name, data in (("bg1_map", bg1), ("bg2_map", bg2), ("bg3_map", bg3)):
        if not data or len(data) % 64:
            raise ValueError(f"{name} has unexpected byte length {len(data)}")
    for name, data in (("bg1_tileset", tileset1), ("bg2_tileset", tileset2)):
        if len(data) != 0x800:
            raise ValueError(f"{name} must decompress to 0x800 bytes, got {len(data):#x}")
    if len(palette) != 0x100:
        raise ValueError(f"Map palette must be 0x100 bytes, got {len(palette):#x}")
    if len(tile_properties) != 0x200:
        raise ValueError(f"Tile-property table must be 0x200 bytes, got {len(tile_properties):#x}")
    if len(bg3_graphics) != 0x1040:
        raise ValueError(f"BG3 graphics must contain a 0x40-byte table and 0x1000 graphics bytes; got {len(bg3_graphics):#x}")

    def interleave_tileset(data: bytes) -> bytes:
        # LoadTileset copies the two 0x400-byte decompression planes alternately
        # through $2180 into WRAM: $C000, $C001, $C002, ... .
        result = bytearray(0x800)
        for i in range(0x400):
            result[2 * i] = data[i]
            result[2 * i + 1] = data[0x400 + i]
        return bytes(result)

    files["bg1_tileset_runtime"] = interleave_tileset(tileset1)
    files["bg2_tileset_runtime"] = interleave_tileset(tileset2)

    for key, name in (('short_entrances', 'short_entrance'), ('long_entrances', 'long_entrance')):
        files[key], addresses[key] = read_entrances(root, rom, name, map_id)
    vram = bytearray(VRAM_BYTES)
    gfx_rows = [entry("assets/gfx/map_gfx/", index) for index in graphics_ids]
    for slot, (graphics_id, word_address, row) in enumerate(zip(graphics_ids, MAP_GFX_DEST_WORDS, gfx_rows)):
        pc = int(row["source_pc"], 16)
        span = rom[pc : pc + GFX_DMA_BYTES]
        if len(span) != GFX_DMA_BYTES:
            raise ValueError(f"Graphics ID {graphics_id} at {row['source_pc']} crosses ROM end")
        byte_address = word_address * 2
        vram[byte_address : byte_address + GFX_DMA_BYTES] = span
        addresses[f"map_gfx_{slot}"] = {
            "item_index": graphics_id,
            "asset": row["asset"],
            "pc": row["source_pc"],
            "cpu": row["source_cpu"],
            "dma_bytes": GFX_DMA_BYTES,
            "vram_word_address": word_address,
        }
    files["vram_bg12"] = bytes(vram)

    map_prop_pc = int(map_prop_row["source_pc"], 16) + offset
    map_prop_cpu = f"${0xC0 + (map_prop_pc >> 16):02X}:{map_prop_pc & 0xFFFF:04X}"
    manifest = {
        "format": "ff6-native-field-map-v2",
        "map_id": map_id,
        "map_prop": {
            "pc": f"0x{map_prop_pc:06X}",
            "cpu": map_prop_cpu,
            "record_bytes": 33,
            "graphics_ids": graphics_ids,
            "bg3_graphics_id": bg3_gfx_id,
            "tileset_ids": tileset_ids,
            "layout_ids": layout_ids,
            "layer_dimensions": layer_dimensions,
            "camera_size_bytes": list(prop[30:32]),
            "palette_id": palette_id,
            "bg3_foreground": bool(prop[2] & 0x80),
            "tile_property_id": prop[4],
        },
        "map_property_source": {
            "asset": map_prop_row["asset"],
            "table_pc": map_prop_row["source_pc"],
            "table_cpu": map_prop_row["source_cpu"],
        },
        "assets": addresses,
        "rom_identity": "Final Fantasy III (USA) [Patched].sfc; read-only inputs",
        "notes": [
            "Four BG1/BG2 graphics DMA windows follow LoadMapGfx and write to 16-bit VRAM word addresses.",
            "The map tileset runtime bytes use the interleaving performed by LoadTileset.",
            "The three exported sub-tilemaps are serialized 8-bit macro-tile IDs, not complete screen maps.",
            "Layer dimensions follow InitScrollClip's MapProp width/height masks.",
            "Layout ID 0 clears 0x4000 physical WRAM bytes; the bundle stores its active logical dimensions.",
            "Entrance records preserve the patched ROM's raw 6-byte point and 7-byte line triggers.",
        ],
    }
    files["metadata"] = (
        b"F6MP"
        + bytes((2,))
        + map_id.to_bytes(2, "little")
        + bytes((1 if prop[2] & 0x80 else 0, prop[4]))
        + b''.join(value.to_bytes(2, 'little') for pair in layer_dimensions for value in pair)
    )
    return manifest, files


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("extracted/twue-3.07"))
    parser.add_argument("--rom", type=Path, default=Path("Final Fantasy III (USA) [Patched].sfc"))
    parser.add_argument(
        "--map-id",
        type=int,
        default=None,
        help="assemble only this MapProp record (default: opening, Narshe and first mine maps)",
    )
    parser.add_argument("--output", type=Path, default=None)
    args = parser.parse_args()
    root = args.root.resolve()
    rom_path = args.rom.resolve()
    map_ids = (args.map_id,) if args.map_id is not None else DEFAULT_MAP_IDS
    for map_id in map_ids:
        if args.output is None:
            output = root / "build" / "native_field" / f"map-{map_id:04d}"
        elif len(map_ids) == 1:
            output = args.output
        else:
            output = args.output / f"map-{map_id:04d}"
        manifest, files = read_map(root, rom_path, map_id)
        output.mkdir(parents=True, exist_ok=True)
        for name, data in files.items():
            (output / f"{name}.bin").write_bytes(data)
        (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        print(f"Map {map_id}: wrote {len(files)} runtime assets to {output}")
        print("Graphics IDs:", ", ".join(map(str, manifest["map_prop"]["graphics_ids"])))
        print("Tilesets:", manifest["map_prop"]["tileset_ids"], "layouts:", manifest["map_prop"]["layout_ids"])
        print("Palette:", manifest["map_prop"]["palette_id"], "BG3 foreground:", manifest["map_prop"]["bg3_foreground"])


if __name__ == "__main__":
    main()
