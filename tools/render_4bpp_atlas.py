#!/usr/bin/env python3
"""Render a SNES planar 4bpp asset and BGR555 palette as a color tile atlas."""

import argparse
from pathlib import Path

from PIL import Image


def decode_tile(data: bytes) -> list[int]:
    pixels = [0] * 64
    for y in range(8):
        for x in range(8):
            bit = 7 - x
            value = 0
            for plane in range(4):
                offset = (plane // 2) * 16 + y * 2 + (plane % 2)
                value |= ((data[offset] >> bit) & 1) << plane
            pixels[y * 8 + x] = value
    return pixels


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("graphics", type=Path)
    parser.add_argument("palette", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--columns", type=int, default=14)
    parser.add_argument("--scale", type=int, default=4)
    args = parser.parse_args()

    raw = args.graphics.read_bytes()
    pal_bytes = args.palette.read_bytes()
    if len(raw) % 32 or len(pal_bytes) < 32 or args.columns < 1:
        raise SystemExit("Expected complete 4bpp tiles and a 16-color BGR555 palette")

    colors = []
    for offset in range(0, 32, 2):
        value = int.from_bytes(pal_bytes[offset : offset + 2], "little")
        colors.append(tuple((((value >> shift) & 31) * 255 + 15) // 31 for shift in (0, 5, 10)))

    tiles = [decode_tile(raw[i : i + 32]) for i in range(0, len(raw), 32)]
    rows = (len(tiles) + args.columns - 1) // args.columns
    image = Image.new("RGB", (args.columns * 8, rows * 8), (255, 0, 255))
    for index, tile in enumerate(tiles):
        ox, oy = (index % args.columns) * 8, (index // args.columns) * 8
        for y in range(8):
            for x in range(8):
                color = colors[tile[y * 8 + x]]
                image.putpixel((ox + x, oy + y), color)

    image = image.resize((image.width * args.scale, image.height * args.scale), Image.Resampling.NEAREST)
    image.save(args.output)
    print(f"Wrote {args.output} ({len(tiles)} tiles, {args.columns} columns)")


if __name__ == "__main__":
    main()
