use std::fs;
use std::path::Path;

use crate::assets::bgr555_to_rgb;
use crate::field_exit::{decode_entrances, FieldExit};
use crate::gfx::{Canvas, HEIGHT, WIDTH};

const MAP_TILE_PIXELS: i32 = 16;
const VRAM_TILE_BYTES: usize = 32;
const BG3_TILE_BYTES: usize = 16;
const BG1_QUADRANT_OFFSETS: [usize; 4] = [0x000, 0x200, 0x400, 0x600];

/// A software-renderable field map assembled by `tools/extract_native_field.py`.
/// Logical layer dimensions come from MapProp's horizontal and vertical masks.
#[derive(Clone)]
pub struct FieldMap {
    map_id: u16,
    world_width: usize,
    world_height: usize,
    layout_widths: [usize; 3],
    layout_heights: [usize; 3],
    bg1: Vec<u8>,
    bg2: Vec<u8>,
    bg3: Vec<u8>,
    bg1_tileset: Vec<u8>,
    bg2_tileset: Vec<u8>,
    bg12_tiles: Vec<[u8; 64]>,
    bg3_formations: Vec<u8>,
    bg3_tiles: Vec<[u8; 64]>,
    palette: Vec<u32>,
    tile_properties: Vec<u8>,
    bg3_foreground: bool,
    exits: Vec<FieldExit>,
}

impl FieldMap {
    pub fn load(directory: &Path) -> Option<Self> {
        let bg1 = fs::read(directory.join("bg1_map.bin")).ok()?;
        let bg2 = fs::read(directory.join("bg2_map.bin")).ok()?;
        let bg3 = fs::read(directory.join("bg3_map.bin")).ok()?;
        let bg1_tileset = fs::read(directory.join("bg1_tileset_runtime.bin")).ok()?;
        let bg2_tileset = fs::read(directory.join("bg2_tileset_runtime.bin")).ok()?;
        let bg12_graphics = fs::read(directory.join("vram_bg12.bin")).ok()?;
        let bg3_graphics = fs::read(directory.join("bg3_graphics.bin")).ok()?;
        let palette_bytes = fs::read(directory.join("palette.bin")).ok()?;
        let tile_properties = fs::read(directory.join("tile_properties.bin")).ok()?;
        let metadata = fs::read(directory.join("metadata.bin")).ok()?;
        let short_entrances = fs::read(directory.join("short_entrances.bin")).ok()?;
        let long_entrances = fs::read(directory.join("long_entrances.bin")).ok()?;
        let exits = decode_entrances(&short_entrances, &long_entrances)?;

        if bg1.is_empty()
            || bg2.is_empty()
            || bg3.is_empty()
            || bg1.len() % 64 != 0
            || bg2.len() % 64 != 0
            || bg3.len() % 64 != 0
            || bg1_tileset.len() != 0x800
            || bg2_tileset.len() != 0x800
            || bg12_graphics.len() != 0x8000
            || bg3_graphics.len() != 0x1040
            || palette_bytes.len() != 0x100
            || tile_properties.len() != 0x200
            || metadata.len() != 21
            || &metadata[..4] != b"F6MP"
            || metadata[4] != 2
        {
            return None;
        }

        // InitScrollClip derives both dimensions from MapProp. Byte length
        // alone cannot distinguish e.g. 32x32 from 16x64 layouts.
        let mut layout_widths = [0; 3];
        let mut layout_heights = [0; 3];
        for layer in 0..3 {
            let offset = 9 + layer * 4;
            layout_widths[layer] = u16::from_le_bytes([metadata[offset], metadata[offset + 1]]) as usize;
            layout_heights[layer] = u16::from_le_bytes([metadata[offset + 2], metadata[offset + 3]]) as usize;
            if ![16, 32, 64, 128].contains(&layout_widths[layer])
                || ![16, 32, 64, 128].contains(&layout_heights[layer]) {
                return None;
            }
        }
        for (layer, data) in [&bg1, &bg2, &bg3].iter().enumerate() {
            if data.len() != layout_widths[layer] * layout_heights[layer] {
                return None;
            }
        }
        let map_id = u16::from_le_bytes([metadata[5], metadata[6]]);

        let bg12_tiles = bg12_graphics
            .chunks_exact(VRAM_TILE_BYTES)
            .map(decode_4bpp_tile)
            .collect();
        let bg3_formations = bg3_graphics[..64].to_vec();
        let bg3_tiles = bg3_graphics[64..]
            .chunks_exact(BG3_TILE_BYTES)
            .map(decode_2bpp_tile)
            .collect();
        let palette = palette_bytes
            .chunks_exact(2)
            .map(|word| {
                let value = u16::from_le_bytes([word[0], word[1]]);
                bgr555_to_rgb(value)
            })
            .collect();

        Some(Self {
            map_id,
            world_width: layout_widths[0].max(64),
            world_height: layout_heights[0].max(64),
            layout_widths,
            layout_heights,
            bg1,
            bg2,
            bg3,
            bg1_tileset,
            bg2_tileset,
            bg12_tiles,
            bg3_formations,
            bg3_tiles,
            palette,
            tile_properties,
            bg3_foreground: metadata[7] & 1 != 0,
            exits,
        })
    }

    pub fn map_id(&self) -> u16 {
        self.map_id
    }

    pub fn exit_at(&self, x: i16, y: i16) -> Option<FieldExit> {
        self.exits.iter().copied().find(|exit| exit.contains(x, y))
    }

    pub fn camera_for_player(&self, map_x: i16, map_y: i16) -> (i32, i32) {
        self.camera_for_pixel(i32::from(map_x) * MAP_TILE_PIXELS, i32::from(map_y) * MAP_TILE_PIXELS)
    }

    pub fn camera_for_pixel(&self, x: i32, y: i32) -> (i32, i32) {
        let world_x = x + MAP_TILE_PIXELS / 2;
        let world_y = y + MAP_TILE_PIXELS / 2;
        let max_x = (self.world_width as i32 * MAP_TILE_PIXELS - WIDTH as i32).max(0);
        let max_y = (self.world_height as i32 * MAP_TILE_PIXELS - HEIGHT as i32).max(0);
        (
            (world_x - WIDTH as i32 / 2).clamp(0, max_x),
            (world_y - HEIGHT as i32 / 2).clamp(0, max_y),
        )
    }

    pub fn in_bounds(&self, map_x: i16, map_y: i16) -> bool {
        map_x >= 0
            && map_y >= 0
            && (map_x as usize) < self.world_width
            && (map_y as usize) < self.world_height
    }

    pub fn clamp_position(&self, map_x: i16, map_y: i16) -> (i16, i16) {
        (
            map_x.clamp(0, self.world_width as i16 - 1),
            map_y.clamp(0, self.world_height as i16 - 1),
        )
    }

    /// Applies the target map's directional mask, impassable/counter flags,
    /// and upper/lower-z flags for a cardinal step. Actor collisions and doors
    /// are separate systems and are not part of this map-level check.
    pub fn can_move(&self, from_x: i16, from_y: i16, to_x: i16, to_y: i16) -> bool {
        if !self.in_bounds(from_x, from_y) || !self.in_bounds(to_x, to_y) {
            return false;
        }
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        let direction_bit = match (dx, dy) {
            (0, -1) => 0x08,
            (1, 0) => 0x01,
            (0, 1) => 0x04,
            (-1, 0) => 0x02,
            _ => return false,
        };
        let layout_width = self.layout_widths[0];
        let from_index = (from_y as usize % self.layout_heights[0]) * layout_width + from_x as usize % layout_width;
        let to_index = (to_y as usize % self.layout_heights[0]) * layout_width + to_x as usize % layout_width;
        let from_tile = self.bg1[from_index] as usize;
        let to_tile = self.bg1[to_index] as usize;
        let from_flags = self.tile_properties[from_tile];
        let to_flags = self.tile_properties[to_tile];
        let from_directions = self.tile_properties[0x100 + from_tile];
        if from_flags == 0xf7
            || to_flags == 0xf7
            || from_directions & direction_bit == 0
            || to_flags & 0x07 == 0x07
        {
            return false;
        }

        let from_level = from_flags & 0x03;
        let to_level = to_flags & 0x03;
        if from_flags & 0x04 != 0 {
            // On a bridge, the current party z-level selects which transition
            // is legal. The opening slice starts on the upper level.
            let upper_party = true;
            if (upper_party && to_level & 0x02 != 0) || (!upper_party && to_level & 0x01 != 0) {
                return false;
            }
        } else if to_level != 0x03 {
            if from_level == 0x03 {
                if to_flags & 0x04 != 0 {
                    return false;
                }
            } else if (from_level ^ 0x03) & to_level != 0 {
                return false;
            }
        }
        true
    }

    pub fn draw(&self, canvas: &mut Canvas, player_x: i16, player_y: i16) {
        self.draw_camera(canvas, self.camera_for_player(player_x, player_y));
    }

    pub fn draw_camera(&self, canvas: &mut Canvas, (camera_x, camera_y): (i32, i32)) {
        canvas.clear(self.palette.first().copied().unwrap_or(0));
        if !self.bg3_foreground {
            self.draw_bg3(canvas, camera_x, camera_y);
        }
        self.draw_bg12(canvas, &self.bg2, self.layout_widths[1], self.layout_heights[1], &self.bg2_tileset, camera_x, camera_y, false);
        self.draw_bg12(canvas, &self.bg1, self.layout_widths[0], self.layout_heights[0], &self.bg1_tileset, camera_x, camera_y, false);
        self.draw_bg12(canvas, &self.bg2, self.layout_widths[1], self.layout_heights[1], &self.bg2_tileset, camera_x, camera_y, true);
        self.draw_bg12(canvas, &self.bg1, self.layout_widths[0], self.layout_heights[0], &self.bg1_tileset, camera_x, camera_y, true);
        if self.bg3_foreground {
            self.draw_bg3(canvas, camera_x, camera_y);
        }
    }

    fn draw_bg12(
        &self,
        canvas: &mut Canvas,
        layout: &[u8],
        layout_width: usize,
        layout_height: usize,
        tileset: &[u8],
        camera_x: i32,
        camera_y: i32,
        high_priority: bool,
    ) {
        let first_x = camera_x / MAP_TILE_PIXELS;
        let first_y = camera_y / MAP_TILE_PIXELS;
        let last_x = ((camera_x + WIDTH as i32 - 1) / MAP_TILE_PIXELS)
            .min(self.world_width as i32 - 1);
        let last_y = ((camera_y + HEIGHT as i32 - 1) / MAP_TILE_PIXELS)
            .min(self.world_height as i32 - 1);
        for map_y in first_y..=last_y {
            for map_x in first_x..=last_x {
                let layout_x = map_x as usize % layout_width;
                let map_tile = layout[(map_y as usize % layout_height) * layout_width + layout_x] as usize;
                for (quadrant, table_offset) in BG1_QUADRANT_OFFSETS.iter().copied().enumerate() {
                    let table_index = table_offset + map_tile * 2;
                    let word = u16::from_le_bytes([tileset[table_index], tileset[table_index + 1]]);
                    if (word & 0x2000 != 0) != high_priority {
                        continue;
                    }
                    let tile_number = (word & 0x03ff) as usize;
                    let Some(tile) = self.bg12_tiles.get(tile_number) else { continue };
                    let palette = ((word >> 10) & 0x07) as usize;
                    let tile_x = map_x * MAP_TILE_PIXELS + (quadrant % 2) as i32 * 8 - camera_x;
                    let tile_y = map_y * MAP_TILE_PIXELS + (quadrant / 2) as i32 * 8 - camera_y;
                    self.blit_tile(
                        canvas,
                        tile,
                        palette,
                        16,
                        tile_x,
                        tile_y,
                        word & 0x4000 != 0,
                        word & 0x8000 != 0,
                    );
                }
            }
        }
    }

    fn draw_bg3(&self, canvas: &mut Canvas, camera_x: i32, camera_y: i32) {
        let first_x = camera_x / MAP_TILE_PIXELS;
        let first_y = camera_y / MAP_TILE_PIXELS;
        let last_x = ((camera_x + WIDTH as i32 - 1) / MAP_TILE_PIXELS)
            .min(self.world_width as i32 - 1);
        let last_y = ((camera_y + HEIGHT as i32 - 1) / MAP_TILE_PIXELS)
            .min(self.world_height as i32 - 1);
        for map_y in first_y..=last_y {
            for map_x in first_x..=last_x {
                let layout_width = self.layout_widths[2];
                let layout_x = map_x as usize % layout_width;
                let entry = self.bg3[(map_y as usize % self.layout_heights[2]) * layout_width + layout_x];
                let metatile = (entry & 0x3f) as usize;
                let palette = ((self.bg3_formations[metatile] & 0x1c) >> 2) as usize;
                for quadrant in 0..4 {
                    let dest_x = quadrant % 2;
                    let dest_y = quadrant / 2;
                    let source_x = if entry & 0x40 != 0 { 1 - dest_x } else { dest_x };
                    let source_y = if entry & 0x80 != 0 { 1 - dest_y } else { dest_y };
                    let tile_number = metatile * 4 + source_y * 2 + source_x;
                    let Some(tile) = self.bg3_tiles.get(tile_number) else { continue };
                    let tile_x = map_x * MAP_TILE_PIXELS + dest_x as i32 * 8 - camera_x;
                    let tile_y = map_y * MAP_TILE_PIXELS + dest_y as i32 * 8 - camera_y;
                    self.blit_tile(
                        canvas,
                        tile,
                        palette,
                        4,
                        tile_x,
                        tile_y,
                        entry & 0x40 != 0,
                        entry & 0x80 != 0,
                    );
                }
            }
        }
    }

    fn blit_tile(
        &self,
        canvas: &mut Canvas,
        tile: &[u8; 64],
        palette: usize,
        colors_per_palette: usize,
        x: i32,
        y: i32,
        flip_x: bool,
        flip_y: bool,
    ) {
        for py in 0..8 {
            for px in 0..8 {
                let source_x = if flip_x { 7 - px } else { px };
                let source_y = if flip_y { 7 - py } else { py };
                let color_index = tile[source_y * 8 + source_x] as usize;
                if color_index == 0 {
                    continue;
                }
                let palette_index = palette * colors_per_palette + color_index;
                if let Some(&color) = self.palette.get(palette_index) {
                    canvas.pixel(x + px as i32, y + py as i32, color);
                }
            }
        }
    }
}

fn decode_4bpp_tile(data: &[u8]) -> [u8; 64] {
    let mut pixels = [0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            let bit = 7 - x;
            for plane in 0..4 {
                let offset = (plane / 2) * 16 + y * 2 + (plane % 2);
                pixels[y * 8 + x] |= ((data[offset] >> bit) & 1) << plane;
            }
        }
    }
    pixels
}

fn decode_2bpp_tile(data: &[u8]) -> [u8; 64] {
    let mut pixels = [0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            let bit = 7 - x;
            pixels[y * 8 + x] = ((data[y * 2] >> bit) & 1) | (((data[y * 2 + 1] >> bit) & 1) << 1);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::FieldMap;

    fn collision_map() -> FieldMap {
        let mut properties = vec![0u8; 0x200];
        properties[1] = 0x01;
        properties[2] = 0x01;
        properties[0x101] = 0x0f;
        properties[0x102] = 0x0f;
        FieldMap {
            map_id: 21,
            world_width: 2,
            world_height: 2,
            layout_widths: [2, 2, 2],
            layout_heights: [2, 2, 2],
            bg1: vec![1, 2, 1, 1],
            bg2: vec![0; 4],
            bg3: vec![0; 4],
            bg1_tileset: vec![0; 0x800],
            bg2_tileset: vec![0; 0x800],
            bg12_tiles: vec![[0; 64]; 1024],
            bg3_formations: vec![0; 64],
            bg3_tiles: vec![[0; 64]; 256],
            palette: vec![0; 128],
            tile_properties: properties,
            bg3_foreground: false,
            exits: Vec::new(),
        }
    }

    #[test]
    fn map_movement_obeys_current_direction_and_destination_flags() {
        let mut map = collision_map();
        assert!(map.can_move(0, 0, 1, 0));
        assert!(map.can_move(0, 0, 0, 1));
        assert!(!map.can_move(0, 0, 0, -1));
        assert!(!map.can_move(0, 0, 1, 1));

        map.tile_properties[0x101] = 0x08;
        assert!(!map.can_move(0, 0, 1, 0));

        map.tile_properties[0x101] = 0x0f;
        map.tile_properties[2] = 0xf7;
        assert!(!map.can_move(0, 0, 1, 0));

        map.tile_properties[2] = 0x07;
        assert!(!map.can_move(0, 0, 1, 0));
    }

    #[test]
    fn compact_layout_repeats_inside_the_64_tile_world() {
        let mut map = collision_map();
        map.world_width = 64;
        map.world_height = 64;
        map.layout_widths = [2, 2, 2];
        map.layout_heights = [2, 2, 2];
        map.bg1 = vec![1; 4];
        map.bg1[0] = 2;
        assert!(map.can_move(1, 0, 2, 0));
        assert!(map.can_move(1, 1, 1, 2));
        assert_eq!(map.camera_for_player(18, 25), (168, 296));
    }
}
