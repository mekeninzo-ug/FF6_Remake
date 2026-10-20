#!/usr/bin/env python3
"""Render a cropped diagnostic view from extract_native_field.py output."""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image


def decode_4bpp(data: bytes) -> list[list[int]]:
    rows = [[0] * 8 for _ in range(8)]
    for y in range(8):
        for x in range(8):
            value = 0
            bit = 7 - x
            for plane in range(4):
                offset = (plane // 2) * 16 + y * 2 + (plane % 2)
                value |= ((data[offset] >> bit) & 1) << plane
            rows[y][x] = value
    return rows


def decode_2bpp(data: bytes) -> list[list[int]]:
    rows = [[0] * 8 for _ in range(8)]
    for y in range(8):
        for x in range(8):
            bit = 7 - x
            rows[y][x] = ((data[y * 2] >> bit) & 1) | (((data[y * 2 + 1] >> bit) & 1) << 1)
    return rows


def colors(raw: bytes) -> list[tuple[int, int, int]]:
    result = []
    for offset in range(0, len(raw), 2):
        value = raw[offset] | (raw[offset + 1] << 8)
        result.append(((value & 31) * 255 // 31, ((value >> 5) & 31) * 255 // 31, ((value >> 10) & 31) * 255 // 31))
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--map", type=Path, default=Path("extracted/twue-3.07/build/native_field/map-0021"))
    parser.add_argument("--x", type=int, default=26, help="player map-tile X")
    parser.add_argument("--pixel-x", type=int, help="override player world-pixel X")
    parser.add_argument("--pixel-y", type=int, help="override player world-pixel Y")
    parser.add_argument("--y", type=int, default=50, help="player map-tile Y")
    parser.add_argument("--output", type=Path, default=Path("narshe-north-entry.png"))
    parser.add_argument("--scale", type=int, default=2)
    parser.add_argument("--quadrants", choices=("initial-load", "scroll-update"), default="initial-load")
    args = parser.parse_args()
    source = args.map
    bg1 = (source / "bg1_map.bin").read_bytes()
    bg2 = (source / "bg2_map.bin").read_bytes()
    bg3 = (source / "bg3_map.bin").read_bytes()
    bg1_set = (source / "bg1_tileset_runtime.bin").read_bytes()
    bg2_set = (source / "bg2_tileset_runtime.bin").read_bytes()
    vram = (source / "vram_bg12.bin").read_bytes()
    bg3_raw = (source / "bg3_graphics.bin").read_bytes()
    pal = colors((source / "palette.bin").read_bytes())

    tile4 = [decode_4bpp(vram[n * 32 : n * 32 + 32]) for n in range(len(vram) // 32)]
    bg3_tiles = [decode_2bpp(bg3_raw[64 + n * 16 : 64 + (n + 1) * 16]) for n in range((len(bg3_raw) - 64) // 16)]
    bg3_form = bg3_raw[:64]
    metadata = (source / 'metadata.bin').read_bytes()
    if len(metadata) != 21 or metadata[:5] != b'F6MP\x02':
        raise ValueError('Rebuild field bundles with the v2 MapProp dimensions')
    layout_widths = [int.from_bytes(metadata[9+i*4:11+i*4], 'little') for i in range(3)]
    layout_heights = [int.from_bytes(metadata[11+i*4:13+i*4], 'little') for i in range(3)]
    width = max(64, layout_widths[0])
    height = max(64, layout_heights[0])
    player_x = args.x * 16 if args.pixel_x is None else args.pixel_x
    player_y = args.y * 16 if args.pixel_y is None else args.pixel_y
    camera_x = min(max(player_x + 8 - 128, 0), width * 16 - 256)
    camera_y = min(max(player_y + 8 - 112, 0), height * 16 - 224)
    image = Image.new("RGB", (256, 224), pal[0])
    pixels = image.load()
    min_x, min_y = camera_x // 16, camera_y // 16
    max_x, max_y = (camera_x + 255) // 16, (camera_y + 223) // 16

    def blit(tile: list[list[int]], tile_no: int, palette: int, sx: int, sy: int, hflip: bool, vflip: bool, bpp: int) -> None:
        for py in range(8):
            wy = 7 - py if vflip else py
            for px in range(8):
                wx = 7 - px if hflip else px
                index = tile[wy][wx]
                if not index:
                    continue
                dx, dy = sx + px, sy + py
                if 0 <= dx < 256 and 0 <= dy < 224:
                    color_index = palette * (16 if bpp == 4 else 4) + index
                    if color_index < len(pal):
                        pixels[dx, dy] = pal[color_index]

    # The two scroll paths place the four property arrays in different orders;
    # keep both available until reference runtime comparison settles this detail.
    offsets = [0x000, 0x200, 0x400, 0x600] if args.quadrants == "initial-load" else [0x000, 0x400, 0x200, 0x600]

    def draw_bg(layout: bytes, layout_width: int, layout_height: int, tileset: bytes, priority: bool) -> None:
        for my in range(min_y, min(max_y + 1, height)):
            for mx in range(min_x, min(max_x + 1, width)):
                map_id = layout[(my % layout_height) * layout_width + mx % layout_width]
                for quadrant, table_offset in enumerate(offsets):
                    index = table_offset + map_id * 2
                    word = tileset[index] | (tileset[index + 1] << 8)
                    if bool(word & 0x2000) != priority:
                        continue
                    tx, ty = (quadrant % 2) * 8, (quadrant // 2) * 8
                    tile_no = word & 0x3FF
                    tile = tile4[tile_no]
                    blit(tile, tile_no, (word >> 10) & 7, mx * 16 + tx - camera_x, my * 16 + ty - camera_y, bool(word & 0x4000), bool(word & 0x8000), 4)

    def draw_bg3() -> None:
        for my in range(min_y, min(max_y + 1, height)):
            for mx in range(min_x, min(max_x + 1, width)):
                entry = bg3[(my % layout_heights[2]) * layout_widths[2] + mx % layout_widths[2]]
                tile_id = entry & 0x3F
                palette = (bg3_form[tile_id] & 0x1C) >> 2
                for quadrant in range(4):
                    source_qx = 1 - quadrant % 2 if entry & 0x40 else quadrant % 2
                    source_qy = 1 - quadrant // 2 if entry & 0x80 else quadrant // 2
                    source_quadrant = source_qy * 2 + source_qx
                    tile_no = tile_id * 4 + source_quadrant
                    tile = bg3_tiles[tile_no]
                    blit(tile, tile_no, palette, mx * 16 + (quadrant % 2) * 8 - camera_x, my * 16 + (quadrant // 2) * 8 - camera_y, bool(entry & 0x40), bool(entry & 0x80), 2)

    if not metadata[7] & 1:
        draw_bg3()
    draw_bg(bg2, layout_widths[1], layout_heights[1], bg2_set, False)
    draw_bg(bg1, layout_widths[0], layout_heights[0], bg1_set, False)
    draw_bg(bg2, layout_widths[1], layout_heights[1], bg2_set, True)
    draw_bg(bg1, layout_widths[0], layout_heights[0], bg1_set, True)
    if metadata[7] & 1:
        draw_bg3()

    scale = max(1, args.scale)
    image.resize((256 * scale, 224 * scale), Image.Resampling.NEAREST).save(args.output)
    print(f"Map {source.name}; camera=({camera_x},{camera_y}); player tile=({args.x},{args.y}); output={args.output}")


if __name__ == "__main__":
    main()
