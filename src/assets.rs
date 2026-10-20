use std::fs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::field::FieldMap;
use crate::dialogue_text::DialogueCatalog;

#[derive(Clone)]
pub struct SpriteSheet {
    pub tiles: Vec<[u8; 64]>,
    pub palette: [u32; 16],
}

impl SpriteSheet {
    pub(crate) fn load(tile_path: &Path, palette_path: &Path) -> Option<Self> {
        let bytes = fs::read(tile_path).ok()?;
        let palette_bytes = fs::read(palette_path).ok()?;
        if bytes.len() < 32 || bytes.len() % 32 != 0 || palette_bytes.len() < 32 {
            return None;
        }
        let tiles = bytes.chunks_exact(32).map(decode_4bpp_tile).collect();
        let mut palette = [0u32; 16];
        for (i, word) in palette_bytes.chunks_exact(2).take(16).enumerate() {
            let value = u16::from_le_bytes([word[0], word[1]]);
            palette[i] = bgr555_to_rgb(value);
        }
        Some(Self { tiles, palette })
    }

    pub fn tile(&self, index: usize) -> Option<&[u8; 64]> {
        self.tiles.get(index)
    }
}

#[derive(Clone)]
pub struct Assets {
    pub root: Option<PathBuf>,
    pub terra: Option<SpriteSheet>,
    pub soldier: Option<SpriteSheet>,
    pub magitek: Option<crate::magitek::MagitekSprites>,
    pub field_maps: HashMap<u16, FieldMap>,
    pub event_script: Vec<u8>,
    pub field_events: crate::field_events::FieldEvents,
    pub actor_sprites: HashMap<(u8,u8),SpriteSheet>,
    pub dialogue_catalog: DialogueCatalog,
}

impl Assets {
    pub fn load() -> Self {
        let root = find_asset_root();
        let Some(root_path) = root.as_ref() else {
            return Self {
                root,
                terra: None,
                soldier: None,
                magitek: None,
                field_maps: HashMap::new(),
                event_script: Vec::new(),
                field_events: crate::field_events::FieldEvents::default(),
                actor_sprites: HashMap::new(),
                dialogue_catalog: DialogueCatalog::default(),
            };
        };
        let gfx = root_path.join("assets/gfx");
        let terra = SpriteSheet::load(
            &gfx.join("map_sprite_gfx/terra.4bpp"),
            &gfx.join("map_sprite_pal/terra.pal"),
        );
        let soldier = SpriteSheet::load(
            &gfx.join("map_sprite_gfx/soldier.4bpp"),
            &gfx.join("map_sprite_pal/locke.pal"),
        );
        let magitek = crate::magitek::MagitekSprites::load(root_path);
        let mut field_maps = HashMap::new();
        if let Ok(entries) = fs::read_dir(root_path.join("build/native_field")) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with("map-") {
                    if let Some(map) = FieldMap::load(&entry.path()) {
                        field_maps.insert(map.map_id(), map);
                    }
                }
            }
        }
        let field_events=crate::field_events::FieldEvents::load(root_path);
        let actor_sprites=field_events.load_sprites(root_path,field_maps.keys().copied());
        let event_script = fs::read(root_path.join("event/event_script.bin")).unwrap_or_default();
        let dialogue_catalog = fs::read_to_string(root_path.join("assets/text/dlg.en.txt"))
            .map(|source| DialogueCatalog::parse(&source)).unwrap_or_default();
        Self {
            root,
            terra,
            soldier,
            magitek,
            field_maps,
            event_script,
            field_events,
            actor_sprites,
            dialogue_catalog,
        }
    }
}

fn find_asset_root() -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("FF6_ASSET_ROOT") {
        let path = PathBuf::from(explicit);
        if path.join("assets/gfx").is_dir() {
            return Some(path);
        }
    }
    let cwd = std::env::current_dir().ok()?;
    for base in cwd.ancestors() {
        let candidate = base.join("extracted/twue-3.07");
        if candidate.join("assets/gfx").is_dir() {
            return Some(candidate);
        }
    }
    let executable = std::env::current_exe().ok()?;
    for base in executable.ancestors() {
        let candidate = base.join("extracted/twue-3.07");
        if candidate.join("assets/gfx").is_dir() {
            return Some(candidate);
        }
    }
    None
}

pub fn bgr555_to_rgb(value: u16) -> u32 {
    // Widen before multiplying: 31 * 255 exceeds u8's range.
    let red = ((value & 0x1f) * 255 / 31) as u8;
    let green = (((value >> 5) & 0x1f) * 255 / 31) as u8;
    let blue = (((value >> 10) & 0x1f) * 255 / 31) as u8;
    rgb(red, green, blue)
}

pub const fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    ((red as u32) << 16) | ((green as u32) << 8) | blue as u32
}

fn decode_4bpp_tile(data: &[u8]) -> [u8; 64] {
    let mut pixels = [0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            let bit = 7 - x;
            let mut value = 0u8;
            for plane in 0..4 {
                let offset = (plane / 2) * 16 + y * 2 + (plane % 2);
                value |= ((data[offset] >> bit) & 1) << plane;
            }
            pixels[y * 8 + x] = value;
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::{bgr555_to_rgb, decode_4bpp_tile};

    #[test]
    fn palette_endpoints_and_primary_colors() {
        assert_eq!(bgr555_to_rgb(0), 0x000000);
        assert_eq!(bgr555_to_rgb(0x7fff), 0xffffff);
        assert_eq!(bgr555_to_rgb(0x001f), 0xff0000);
        assert_eq!(bgr555_to_rgb(0x03e0), 0x00ff00);
        assert_eq!(bgr555_to_rgb(0x7c00), 0x0000ff);
        assert_eq!(bgr555_to_rgb(0xffff), 0xffffff);
    }

    #[test]
    fn decodes_planar_pixel_bitplanes() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0b1000_0000;
        bytes[1] = 0b1000_0000;
        bytes[16] = 0b1000_0000;
        bytes[17] = 0b1000_0000;
        let tile = decode_4bpp_tile(&bytes);
        assert_eq!(tile[0], 0b1111);
        assert_eq!(tile[1], 0);
    }
}
