//! FF6 field Magitek composition from target formation/frame tables.
use crate::assets::SpriteSheet;
use crate::gfx::Canvas;
use std::path::Path;

#[derive(Clone)]
pub struct MagitekSprites {
    pub vehicle: SpriteSheet,
    formations: [[u16; 16]; 4],
    frames: [[usize; 6]; 58],
}

impl MagitekSprites {
    pub fn load(root: &Path) -> Option<Self> {
        let data = std::fs::read(root.join("build/native_sprites/magitek_tiles.bin")).ok()?;
        let offsets = std::fs::read(root.join("build/native_sprites/frame_offsets.bin")).ok()?;
        if data.len() != 128 || offsets.len() != 696 { return None; }
        let vehicle = SpriteSheet::load(&root.join("assets/gfx/vehicle.4bpp"),
            &root.join("assets/gfx/map_sprite_pal/vehicle.pal"))?;
        let mut formations = [[0; 16]; 4];
        for (i, pair) in data.chunks_exact(2).enumerate() {
            let word = u16::from_le_bytes([pair[0], pair[1]]);
            let tile = usize::from(word & 0x1ff);
            // TfrVehicleGfx loads $1c00 bytes at $7200, with OBJ base $6000.
            if !(0x120..=0x1ee).contains(&tile) || (word >> 9) & 7 != 7 { return None; }
            formations[i / 16][i % 16] = word;
        }
        let mut frames = [[0; 6]; 58];
        for (i, pair) in offsets.chunks_exact(2).enumerate() {
            let value = usize::from(u16::from_le_bytes([pair[0], pair[1]]));
            if value % 32 != 0 { return None; }
            frames[i / 6][i % 6] = value / 32;
        }
        Some(Self { vehicle, formations, frames })
    }

    /// Object pixel coordinates before camera subtraction select gait, even
    /// when stationary. DrawMagitek uses X only with horizontal velocity.
    pub fn formation_words(&self, facing: u8, world_x: i32, world_y: i32, horizontal: bool) -> [u16; 4] {
        let coordinate = if horizontal { world_x } else { world_y };
        let index = usize::from(facing & 3) * 4 + ((coordinate >> 3) & 3) as usize;
        std::array::from_fn(|piece| self.formations[piece][index])
    }

    pub fn draw(&self, canvas: &mut Canvas, rider: Option<&SpriteSheet>, facing: u8,
        world: (i32, i32), screen: (i32, i32), horizontal: bool, moving: bool, show_rider: bool) {
        let (world_x, world_y) = world;
        let (x, y) = screen;
        let words = self.formation_words(facing, world_x, world_y, horizontal);
        for (piece, word) in words.into_iter().enumerate() {
            let base = usize::from(word & 0x1ff) - 0x120;
            canvas.sprite_16x16(&self.vehicle, [base, base+1, base+16, base+17],
                x + (piece % 2) as i32 * 16, y - 24 + (piece / 2) as i32 * 16,
                word & 0x4000 != 0, word & 0x8000 != 0);
        }
        if show_rider {
            if let Some(rider) = rider {
                let coordinate = if horizontal { world_x } else { world_y };
                let phase = if moving { ((coordinate >> 3) & 3) as usize } else { 0 };
                let frame = match facing & 3 {
                    0 => [4, 5, 4, 3][phase],
                    2 => [1, 2, 1, 0][phase],
                    _ => [0x2e, 0x2f, 0x2e, 0x2f][phase],
                };
                let tiles = self.frames[frame];
                let bob = (coordinate >> 3) & 1;
                canvas.sprite_16x16(rider, [tiles[0], tiles[1], tiles[2], tiles[3]],
                    x + 8, y - 30 + bob, facing & 3 == 1, false);
            }
        }
    }
}

pub fn capture_atlas(directory: &Path) -> std::io::Result<()> {
    let assets = crate::assets::Assets::load();
    let sprites = assets.magitek.as_ref().ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::NotFound, "Magitek bundle missing; run tools/extract_native_sprites.py"))?;
    std::fs::create_dir_all(directory)?;
    for show in [false, true] {
        let mut canvas = Canvas::new();
        canvas.clear(0);
        for facing in 0..4u8 {
            for phase in 0..4i32 {
                sprites.draw(&mut canvas, assets.terra.as_ref(), facing, (phase*8, phase*8),
                    (phase*64+16, i32::from(facing)*56+43), matches!(facing, 1 | 3), true, show);
            }
        }
        canvas.save_bmp(&directory.join(if show { "riders.bmp" } else { "vehicles.bmp" }))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_magitek_words_follow_direction_and_world_coordinate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("extracted/twue-3.07");
        let sprites = MagitekSprites::load(&root).unwrap();
        assert_eq!(sprites.formation_words(0, 0, 0, false), [0x2fac,0x6fac,0x2fae,0x6fae]);
        assert_eq!(sprites.formation_words(0, 0, 8, false), [0x2fc0,0x6fc4,0x2fc2,0x6fc6]);
        assert_eq!(sprites.formation_words(3, 24, 0, true), [0x2fe8,0x2fea,0x2fec,0x2fee]);
    }
}
