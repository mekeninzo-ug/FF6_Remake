#!/usr/bin/env python3
"""Read-only identity and extraction audit for the supplied local FF6 inputs."""

from __future__ import annotations

import csv
import hashlib
import json
import re
import zlib
from collections import Counter
from pathlib import Path

from extract_native_field import DEFAULT_MAP_IDS, read_map


ROOT = Path(__file__).resolve().parents[1]
ROM_EXPECTED = {
    "Final Fantasy III (USA) [Unpatched].sfc": (
        3_145_728,
        "A27F1C7A",
        "0F51B4FCA41B7FD509E4B8F9D543151F68EFA5E97B08493E4B2A0C06F5D8D5E2",
    ),
    "Final Fantasy III (USA) [Patched].sfc": (
        4_194_304,
        "FB8A31B0",
        "F385C715B3088D7D697171EC76B93B8F32623D63B3672A364121269A9E41DBB9",
    ),
}
IPS_PATH = ROOT / "Patch" / "Final Fantasy VI (TWUE 3.07) + Bug Fixes.ips"
IPS_EXPECTED = (795_104, "9B4A984E")


def check_rom_identities() -> dict[str, bytes]:
    images: dict[str, bytes] = {}
    for name, (expected_size, expected_crc, expected_sha) in ROM_EXPECTED.items():
        path = ROOT / name
        data = path.read_bytes()
        crc = f"{zlib.crc32(data) & 0xFFFFFFFF:08X}"
        sha = hashlib.sha256(data).hexdigest().upper()
        assert len(data) == expected_size, f"{name}: size {len(data)} != {expected_size}"
        assert crc == expected_crc, f"{name}: CRC32 {crc} != {expected_crc}"
        assert sha == expected_sha, f"{name}: SHA-256 {sha} != {expected_sha}"
        images[name] = data
        print(f"ROM OK: {name} ({len(data):,} bytes; CRC32 {crc})")
    return images


def check_distribution_excludes() -> None:
    manifest = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    exclude_block = re.search(r"(?ms)^exclude\s*=\s*\[(.*?)^\]", manifest)
    assert exclude_block, "Cargo.toml must explicitly exclude ROM and patch inputs"
    exclusions = set(re.findall(r'"([^"]+)"', exclude_block.group(1)))
    assert {"**/*.sfc", "**/*.ips", "Patch/**"} <= exclusions, (
        "Cargo packaging exclusions must cover SFC ROMs, IPS files and Patch/"
    )
    print("PACKAGE OK: Cargo manifest excludes SFC ROMs, IPS patches and Patch/")


def check_ips_bytes(patch_path: Path, target: bytes) -> tuple[int, int]:
    """Compare expanded IPS records to target bytes; never writes either file."""
    patch = patch_path.read_bytes()
    crc = f"{zlib.crc32(patch) & 0xFFFFFFFF:08X}"
    assert (len(patch), crc) == IPS_EXPECTED, (
        f"IPS identity {(len(patch), crc)} != {IPS_EXPECTED}"
    )
    assert patch.startswith(b"PATCH"), "IPS header is missing"
    cursor = 5
    records = written = 0
    while True:
        marker = patch[cursor : cursor + 3]
        cursor += 3
        if marker == b"EOF":
            break
        assert len(marker) == 3, "truncated IPS record offset"
        offset = int.from_bytes(marker, "big")
        assert cursor + 2 <= len(patch), "truncated IPS record length"
        size = int.from_bytes(patch[cursor : cursor + 2], "big")
        cursor += 2
        if size:
            assert cursor + size <= len(patch), f"truncated IPS data at {offset:#x}"
            payload = patch[cursor : cursor + size]
            cursor += size
        else:
            assert cursor + 3 <= len(patch), f"truncated IPS RLE record at {offset:#x}"
            run = int.from_bytes(patch[cursor : cursor + 2], "big")
            value = patch[cursor + 2]
            cursor += 3
            assert run, f"zero-length IPS RLE record at {offset:#x}"
            payload = bytes([value]) * run
        end = offset + len(payload)
        assert end <= len(target), f"IPS record at {offset:#x} extends past target image"
        assert target[offset:end] == payload, f"IPS record mismatch at {offset:#x}"
        records += 1
        written += len(payload)
    assert cursor == len(patch), "unexpected trailing data after IPS EOF"
    print(f"IPS OK: {records:,} records / {written:,} addressed bytes match patched ROM")
    return records, written


def check_patch_note_copies() -> None:
    source = ROOT / "Patch"
    copies = ROOT / "extracted" / "reference" / "patch_notes"
    notes = sorted(source.glob("*.txt"))
    assert len(notes) == 5, f"found {len(notes)} patch note/reference documents"
    for note in notes:
        copy = copies / note.name
        assert copy.is_file() and copy.read_bytes() == note.read_bytes(), (
            f"patch note reference differs: {note.name}"
        )
    print("NOTES OK: all five text documents match their preserved reference copies")


def check_asset_counts(flavor: str) -> None:
    base = ROOT / "extracted" / flavor / "assets"
    expected = {"data": 535, "gfx": 1430, "text": 30, "sound": 71}
    for kind, count in expected.items():
        actual = sum(1 for path in (base / kind).rglob("*") if path.is_file())
        assert actual == count, f"{flavor}/{kind}: {actual} files, expected {count}"
    print(f"ASSETS OK: {flavor}: 535 data, 1,430 graphics, 30 text, 71 sound files")


def check_source_map(flavor: str) -> int:
    path = ROOT / "extracted" / flavor / "provenance" / "source_asset_map.csv"
    checked = 0
    with path.open(newline="", encoding="utf-8-sig") as stream:
        for row in csv.DictReader(stream):
            pc = row.get("source_pc", "")
            cpu = row.get("source_cpu", "")
            if not pc or not cpu:
                continue
            pc_value = int(pc, 0)
            cpu_value = int(cpu.lstrip("$"), 16)
            bank = cpu_value >> 16
            address = cpu_value & 0xFFFF
            if 0xC0 <= bank <= 0xFF:
                mapped_pc = ((bank - 0xC0) << 16) | address
                assert mapped_pc == pc_value, (
                    f"{flavor}: PC/CPU mismatch for {row.get('asset')} "
                    f"{pc} / {cpu} (mapped {mapped_pc:#x})"
                )
                checked += 1
    print(f"MAP OK: {flavor}: {checked:,} canonical HiROM PC/CPU pairs")
    return checked


def check_version_diff() -> None:
    path = ROOT / "extracted" / "comparison" / "asset_diff.csv"
    counts: Counter[str] = Counter()
    with path.open(newline="", encoding="utf-8-sig") as stream:
        for row in csv.DictReader(stream):
            if row["status"] == "changed":
                group = row["asset"].split("/", 1)[0]
                counts[group] += 1
    expected = Counter({"text": 27, "gfx": 205, "data": 7})
    assert counts == expected, f"asset diff counts {counts} != {expected}"
    print("DIFF OK: 239 mapped changed assets (27 text, 205 graphics, 7 data)")


def check_field_sprite_frame_map(images: dict[str, bytes]) -> None:
    # First nine 16x24 walk frames from MapSpriteTileOffsets in
    # ff6/src/field/sprite_data.asm. Each word is a byte offset into the
    # decompressed object sprite; each graphics tile is 32 bytes.
    offsets = (
        (0x0000, 0x0020, 0x0040, 0x0060, 0x0080, 0x00A0),
        (0x0000, 0x0020, 0x00C0, 0x00E0, 0x0100, 0x0120),
        (0x0000, 0x0020, 0x0140, 0x0160, 0x0080, 0x00A0),
        (0x0180, 0x01A0, 0x01C0, 0x01E0, 0x0200, 0x0220),
        (0x0180, 0x01A0, 0x0240, 0x0260, 0x0280, 0x02A0),
        (0x0180, 0x01A0, 0x02C0, 0x02E0, 0x0200, 0x0220),
        (0x0300, 0x0320, 0x0340, 0x0360, 0x0380, 0x03A0),
        (0x03C0, 0x03E0, 0x0400, 0x0420, 0x0440, 0x0460),
        (0x0300, 0x0320, 0x0480, 0x04A0, 0x04C0, 0x04E0),
    )
    expected = b"".join(value.to_bytes(2, "little") for frame in offsets for value in frame)
    pc_offset = 0x00CE3A
    for name, image in images.items():
        assert image[pc_offset : pc_offset + len(expected)] == expected, (
            f"{name}: field walk frame table differs at PC {pc_offset:#08x}"
        )
        assert image.count(expected) == 1, f"{name}: expected one exact walk-frame table prefix"

    rust_source = (ROOT / "src" / "app.rs").read_text(encoding="utf-8")
    for rust_name, source_frames in (
        ("WALK_DOWN", offsets[0:3]),
        ("WALK_UP", offsets[3:6]),
        ("WALK_LEFT", offsets[6:9]),
    ):
        match = re.search(
            rf"const {rust_name}: \[\[usize; 6\]; 3\] = \[(.*?)\];",
            rust_source,
            flags=re.DOTALL,
        )
        assert match, f"Rust source is missing {rust_name}"
        actual_frames = [
            [int(value) for value in re.findall(r"\d+", frame)]
            for frame in re.findall(r"\[([\d,\s]+)\]", match.group(1))
        ]
        expected_frames = [[value // 32 for value in frame] for frame in source_frames]
        assert actual_frames == expected_frames, (
            f"Rust {rust_name} differs from source byte offsets: {actual_frames} != {expected_frames}"
        )

    vanilla_gfx = ROOT / "extracted" / "vanilla" / "assets" / "gfx" / "map_sprite_gfx"
    patched_gfx = ROOT / "extracted" / "twue-3.07" / "assets" / "gfx" / "map_sprite_gfx"
    for asset in ("terra.4bpp", "soldier.4bpp"):
        assert (vanilla_gfx / asset).read_bytes() != (patched_gfx / asset).read_bytes(), (
            f"expected patched target graphics to differ for {asset}"
        )
    print(
        "SPRITES OK: walk-frame map at C0:CE3A matches both ROMs and Rust; "
        "patched Terra/Soldier graphics differ from clean exports"
    )


def check_opening_lore_text() -> None:
    dialogue_path = ROOT / "extracted" / "twue-3.07" / "assets" / "text" / "dlg.en.txt"
    dialogue = dialogue_path.read_text(encoding="utf-8")
    start = dialogue.index("#text:DLG_6")
    end = dialogue.index("#text:DLG_10", start)
    extracted = re.sub(r"(?m)^#text:.*$", "", dialogue[start:end])
    extracted = re.sub(r"\{[^}]*\}", "", extracted)

    rust_source = (ROOT / "src" / "app.rs").read_text(encoding="utf-8")
    intro = re.search(r"const INTRO: \[\[&str; 4\]; 4\] = \[(.*?)\n\];", rust_source, re.DOTALL)
    assert intro, "Rust source is missing the four opening text cards"
    rust_words = "".join(re.findall(r'"([^"\\]*(?:\\.[^"\\]*)*)"', intro.group(1)))

    normalize = lambda value: re.sub(r"[^a-z]", "", value.lower().replace("_", ""))
    assert normalize(extracted) == normalize(rust_words), (
        "Rust intro text does not match target DLG_6–DLG_9 after removing layout controls"
    )
    print("TEXT OK: four Rust opening cards match patched DLG_6-DLG_9 text")

    start = dialogue.index("#text:DLG_1")
    end = dialogue.index("#text:DLG_6", start)
    extracted = re.sub(r"(?m)^#text:.*$", "", dialogue[start:end])
    extracted = re.sub(r"\{[^}]*\}", "", extracted)
    opening = re.search(r"const OPENING_DIALOGUE: \[&str; 7\] = \[(.*?)\n\];", rust_source, re.DOTALL)
    assert opening, "Rust source is missing the opening Biggs/Wedge dialogue"
    rust_words = "".join(re.findall(r'"([^"\\]*(?:\\.[^"\\]*)*)"', opening.group(1)))
    assert normalize(extracted) == normalize(rust_words), (
        "Rust opening dialogue does not match patched DLG_1-DLG_5 text"
    )
    print("TEXT OK: seven Rust opening exchanges match patched DLG_1-DLG_5 text")

    start = dialogue.index("#text:DLG_11")
    end = dialogue.find("#text:", start + 1)
    extracted = re.sub(r"(?m)^#text:.*$", "", dialogue[start : end if end >= 0 else None])
    extracted = re.sub(r"\{[^}]*\}", "", extracted)
    assert 'self.assets.dialogue_catalog.literal(id)' in rust_source
    assert 'const POST_CUTSCENE_DIALOGUE' not in rust_source
    controls = re.findall(r'\{([^}]+)\}', dialogue[start : end if end >= 0 else None])
    assert set(controls) <= {'n', '0'}
    print("TEXT OK: post-cutscene dialogue is consumed directly from the target catalog through supported literal controls")


def check_native_field_map(target: bytes) -> None:
    """Compare diagnostic and opening map bundles back to target inputs."""
    flavor = ROOT / "extracted" / "twue-3.07"
    generated = flavor / "build" / "native_field" / "map-0021"
    with (flavor / "provenance" / "source_asset_map.csv").open(
        newline="", encoding="utf-8-sig"
    ) as stream:
        rows = list(csv.DictReader(stream))

    def mapped(prefix: str, item_index: int) -> dict[str, str]:
        matches = [
            row
            for row in rows
            if row["asset"].startswith(prefix)
            and row["kind"] == "data"
            and row["status"] == "ok"
            and int(row["item_index"]) == item_index
        ]
        assert len(matches) == 1, f"Map21 asset mapping is ambiguous: {prefix} index {item_index}"
        return matches[0]

    prop_row = mapped("assets/data/field/map_prop.bin", 0)
    prop_table = (flavor / prop_row["asset"]).read_bytes()
    prop = prop_table[21 * 33 : 22 * 33]
    word = lambda offset: int.from_bytes(prop[offset : offset + 2], "little")
    graphics_ids = [
        prop[7] & 0x7F,
        ((word(7) << 1) & 0x7F00) >> 8,
        ((word(8) << 2) & 0x7F00) >> 8,
        ((word(9) << 3) & 0x7F00) >> 8,
    ]
    bg3_id = (word(10) >> 4) & 0x3F
    tileset_ids = [(word(11) >> 2) & 0x7F, (word(12) >> 1) & 0x7F]
    layout_ids = [word(13) & 0x3FF, (word(14) >> 2) & 0x3FF, (word(15) >> 4) & 0x3FF]
    palette_id = prop[25]

    manifest = json.loads((generated / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["map_id"] == 21
    assert manifest["map_prop"]["pc"] == "0x2D91B5"
    assert manifest["map_prop"]["cpu"] == "$ED:91B5"
    assert manifest["map_prop"]["graphics_ids"] == graphics_ids == [10, 30, 31, 32]
    assert manifest["map_prop"]["bg3_graphics_id"] == bg3_id == 3
    assert manifest["map_prop"]["tileset_ids"] == tileset_ids == [19, 20]
    assert manifest["map_prop"]["layout_ids"] == layout_ids == [113, 114, 190]
    assert manifest["map_prop"]["palette_id"] == palette_id == 12
    assert manifest["map_prop"]["tile_property_id"] == prop[4] == 8

    for key, layout_id in zip(("bg1_map", "bg2_map", "bg3_map"), layout_ids):
        row = mapped("assets/data/field/sub_tilemap/", layout_id)
        assert (generated / f"{key}.bin").read_bytes() == (flavor / row["asset"]).read_bytes()
    for key, tileset_id in zip(("bg1_tileset", "bg2_tileset"), tileset_ids):
        row = mapped("assets/data/field/map_tileset/", tileset_id)
        raw = (flavor / row["asset"]).read_bytes()
        assert len(raw) == 0x800
        expected = bytearray(0x800)
        for index in range(0x400):
            expected[index * 2] = raw[index]
            expected[index * 2 + 1] = raw[0x400 + index]
        assert (generated / f"{key}.bin").read_bytes() == raw
        assert (generated / f"{key}_runtime.bin").read_bytes() == expected

    expected_vram = bytearray(0x8000)
    for slot, (graphics_id, word_address) in enumerate(zip(graphics_ids, (0, 0x1000, 0x1800, 0x2000))):
        row = mapped("assets/gfx/map_gfx/", graphics_id)
        pc = int(row["source_pc"], 16)
        start = word_address * 2
        expected_vram[start : start + 0x2000] = target[pc : pc + 0x2000]
        assert manifest["assets"][f"map_gfx_{slot}"]["pc"] == row["source_pc"]
    assert (generated / "vram_bg12.bin").read_bytes() == expected_vram

    tile_prop_row = mapped("assets/data/field/map_tile_prop/", prop[4])
    palette_row = mapped("assets/gfx/map_pal/", palette_id)
    bg3_gfx_row = mapped("assets/gfx/map_gfx_bg3/", bg3_id)
    assert (generated / "tile_properties.bin").read_bytes() == (flavor / tile_prop_row["asset"]).read_bytes()
    assert (generated / "palette.bin").read_bytes() == (flavor / palette_row["asset"]).read_bytes()
    assert (generated / "bg3_graphics.bin").read_bytes() == (flavor / bg3_gfx_row["asset"]).read_bytes()
    metadata = (generated / "metadata.bin").read_bytes()
    assert metadata == b"F6MP\x02\x15\x00\x01\x08" + b'\x40\x00' * 6

    # The prototype spawn is selected for a visible, walkable sample area.
    # This verifies the target map data used by the Rust movement path; it does
    # not claim that GameStart uses this position.
    bg1 = (generated / "bg1_map.bin").read_bytes()
    tile_properties = (generated / "tile_properties.bin").read_bytes()
    spawn_x, spawn_y = 26, 50
    spawn_index = spawn_y * 64 + spawn_x
    spawn_tile = bg1[spawn_index]
    assert spawn_tile == 0x92
    assert tile_properties[spawn_tile] == 0x01
    directions = ((0, -1, 0x08), (1, 0, 0x01), (0, 1, 0x04), (-1, 0, 0x02))
    for dx, dy, direction_bit in directions:
        next_x, next_y = spawn_x + dx, spawn_y + dy
        next_tile = bg1[next_y * 64 + next_x]
        assert tile_properties[0x100 + spawn_tile] & direction_bit
        assert tile_properties[next_tile] not in (0xF7,)
        assert tile_properties[next_tile] & 0x07 != 0x07

    print(
        "FIELD MAP OK: MapProp 21 at ED:91B5 assembles target BG1/BG2/BG3, "
        "tilesets, 4 VRAM DMA windows, palette, and walkable demo spawn"
    )

    # The opening event source is a serialized event program, not 65816 code.
    event_source = ROOT / "extracted" / "reference" / "ff6-disassembly-event" / "event_main.asm"
    event_lines = event_source.read_text(encoding="utf-8").splitlines()
    start = next(i for i, line in enumerate(event_lines) if line.strip() == "_cc985b:")
    stop = next(i for i, line in enumerate(event_lines[start + 1 :], start + 1) if line.strip() == "_cc9a4f:")
    cutscene = "\n".join(event_lines[start:stop])
    map_calls = [
        (int(map_id), int(x), int(y))
        for map_id, x, y in re.findall(r"load_map\s+(\d+),\s*\{(\d+),\s*(\d+)\}", cutscene)
    ]
    dialog_ids = [int(value) for value in re.findall(r"dlg DLG_(\d+)", cutscene)]
    assert map_calls == [(23, 18, 25), (20, 38, 54), (244, 7, 27), (18, 11, 7)]
    assert dialog_ids == [6, 7, 8, 9, 1, 2, 3, 4, 5]
    assert "load_map 19, {38, 58}, UP" in "\n".join(event_lines[stop : stop + 4])
    assert "dlg DLG_11" in "\n".join(event_lines[stop : stop + 100])
    opening_entry = "\n".join(event_lines[stop : stop + 100])
    terra_scripts = re.findall(r"obj_script TERRA(?:, ASYNC)?\s+(.*?)\s+end", opening_entry, re.DOTALL)
    terra_steps = [int(n) for body in terra_scripts for n in re.findall(r"move UP, (\d+)", body)]
    assert terra_steps == [8, 1]
    dialogue_y = 58 - terra_steps[0]
    control_y = dialogue_y - terra_steps[1]
    assert (dialogue_y, control_y) == (50, 49)
    # Runtime command boundaries and resulting state are checked by the
    # independent source encoder in verify_opening_event.py, rather than
    # requiring the app to hard-code the event's coordinates.

    event_offset = 0x0C985B - 0x0A0000
    end_offset = 0x0C9A4F - 0x0A0000
    final_map_end_offset = 0x0C9AD5 - 0x0A0000
    vanilla_event = (ROOT / "extracted" / "vanilla" / "event" / "event_script.bin").read_bytes()
    patched_event = (ROOT / "extracted" / "twue-3.07" / "event" / "event_script.bin").read_bytes()
    assert vanilla_event[event_offset:end_offset] == patched_event[event_offset:end_offset]
    assert vanilla_event[end_offset:final_map_end_offset] == patched_event[end_offset:final_map_end_offset]

    # Check every additional opening map's generated runtime files directly
    # against its target record, mapped exports and patched graphics windows.
    for map_id in DEFAULT_MAP_IDS:
        if map_id == 21:
            continue
        expected_manifest, expected_files = read_map(flavor, ROOT / "Final Fantasy III (USA) [Patched].sfc", map_id)
        map_directory = flavor / "build" / "native_field" / f"map-{map_id:04d}"
        actual_manifest = json.loads((map_directory / "manifest.json").read_text(encoding="utf-8"))
        assert actual_manifest == expected_manifest
        record_pc = 0x2D8F00 + map_id * 33
        raw_prop = target[record_pc:record_pc + 33]
        dimension_values = [16 << ((raw_prop[offset] >> shift) & 3)
                            for offset, shift in ((23, 6), (23, 4), (23, 2), (23, 0), (24, 6), (24, 4))]
        expected_dimensions = b''.join(value.to_bytes(2, 'little') for value in dimension_values)
        assert (map_directory / 'metadata.bin').read_bytes()[9:] == expected_dimensions
        for filename, expected_data in expected_files.items():
            assert (map_directory / f"{filename}.bin").read_bytes() == expected_data
        if map_id == 23:
            assert expected_manifest['map_prop']['layer_dimensions'] == [[32, 32]] * 3
            assert expected_files['metadata'][9:] == b'\x20\x00' * 6
        if map_id in (18, 244):
            assert expected_files["bg3_map"] == bytes(0x400 if map_id == 18 else 0x1000)
    print("OPENING EVENT OK: source order, Terra's final scripted step and unchanged clean/target bytes cover maps 23/20/244/18 and map 19")
    print(f"FIELD BUNDLES OK: {len(DEFAULT_MAP_IDS)} runtime bundles match patched assets, entrance records and MapProp width/height masks")


def main() -> None:
    images = check_rom_identities()
    check_distribution_excludes()
    target = images["Final Fantasy III (USA) [Patched].sfc"]
    check_ips_bytes(IPS_PATH, target)
    check_patch_note_copies()
    check_asset_counts("vanilla")
    check_asset_counts("twue-3.07")
    check_source_map("vanilla")
    check_source_map("twue-3.07")
    check_version_diff()
    check_field_sprite_frame_map(images)
    check_opening_lore_text()
    check_native_field_map(target)
    print("Audit passed. This script only reads ROMs, IPS, maps, exports, and Rust source.")


if __name__ == "__main__":
    main()
