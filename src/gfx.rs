use crate::assets::SpriteSheet;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 224;

pub const BLACK: u32 = 0x00050912;
pub const WHITE: u32 = 0x00F3F6FF;
pub const PALE_BLUE: u32 = 0x00B8D6F4;
pub const GOLD: u32 = 0x00F2C96D;
pub const RED: u32 = 0x00D25B55;

pub struct Canvas {
    pub pixels: Vec<u32>,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            pixels: vec![BLACK; WIDTH * HEIGHT],
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Save the software framebuffer directly, without a window or graphics API.
    pub fn save_bmp(&self, path: &std::path::Path) -> std::io::Result<()> {
        let image_bytes = (WIDTH * HEIGHT * 4) as u32;
        let mut bytes = Vec::with_capacity(54 + image_bytes as usize);
        bytes.extend_from_slice(b"BM");
        bytes.extend_from_slice(&(54 + image_bytes).to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&54u32.to_le_bytes());
        bytes.extend_from_slice(&40u32.to_le_bytes());
        bytes.extend_from_slice(&(WIDTH as i32).to_le_bytes());
        bytes.extend_from_slice(&(-(HEIGHT as i32)).to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&image_bytes.to_le_bytes());
        bytes.extend_from_slice(&[0; 16]);
        for color in &self.pixels {
            bytes.extend_from_slice(&color.to_le_bytes());
        }
        std::fs::write(path, bytes)
    }

    pub fn pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
            self.pixels[y as usize * WIDTH + x as usize] = color;
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        let left = x.max(0);
        let top = y.max(0);
        let right = (x + width).min(WIDTH as i32);
        let bottom = (y + height).min(HEIGHT as i32);
        for row in top..bottom {
            let start = row as usize * WIDTH + left as usize;
            let end = row as usize * WIDTH + right as usize;
            if start < end {
                self.pixels[start..end].fill(color);
            }
        }
    }

    pub fn frame(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        self.rect(x, y, width, 1, color);
        self.rect(x, y + height - 1, width, 1, color);
        self.rect(x, y, 1, height, color);
        self.rect(x + width - 1, y, 1, height, color);
    }

    pub fn panel(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.rect(x, y, width, height, 0x000A1427);
        self.frame(x, y, width, height, 0x00D6E8FF);
        self.frame(x + 2, y + 2, width - 4, height - 4, 0x004D668A);
    }

    pub fn text(&mut self, x: i32, y: i32, text: &str, color: u32) {
        let mut pen_x = x;
        for ch in text.chars() {
            if ch == '\n' {
                pen_x = x;
                continue;
            }
            let rows = glyph(ch.to_ascii_uppercase());
            for (row, bits) in rows.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        self.pixel(pen_x + col, y + row as i32, color);
                    }
                }
            }
            pen_x += 6;
        }
    }

    pub fn centered_text(&mut self, y: i32, text: &str, color: u32) {
        let width = text.chars().count() as i32 * 6 - 1;
        self.text((WIDTH as i32 - width) / 2, y, text, color);
    }

    /// Four 8x8 tiles arranged as one SNES 16x16 OBJ. Flip the whole OBJ.
    pub fn sprite_16x16(&mut self, sprite: &SpriteSheet, tiles: [usize; 4],
        x: i32, y: i32, horizontal: bool, vertical: bool) {
        for py in 0..16usize {
            for px in 0..16usize {
                let sx = if horizontal { 15 - px } else { px };
                let sy = if vertical { 15 - py } else { py };
                if let Some(tile) = sprite.tile(tiles[(sy/8)*2 + sx/8]) {
                    let index = usize::from(tile[(sy%8)*8 + sx%8]);
                    if index != 0 { self.pixel(x+px as i32, y+py as i32, sprite.palette[index]); }
                }
            }
        }
    }

    pub fn sprite_16x24(
        &mut self,
        sprite: &SpriteSheet,
        tile_indices: [usize; 6],
        x: i32,
        y: i32,
        flip_horizontal: bool,
    ) {
        for tile_y in 0..3 {
            for tile_x in 0..2 {
                let source_tile_x = if flip_horizontal { 1 - tile_x } else { tile_x };
                let index = tile_indices[tile_y * 2 + source_tile_x];
                let Some(tile) = sprite.tile(index) else {
                    return;
                };
                for py in 0..8 {
                    for px in 0..8 {
                        let source_x = if flip_horizontal { 7 - px } else { px };
                        let color_index = tile[py * 8 + source_x] as usize;
                        if color_index != 0 {
                            self.pixel(
                                x + tile_x as i32 * 8 + px as i32,
                                y + tile_y as i32 * 8 + py as i32,
                                sprite.palette[color_index],
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice = error * 2;
            if twice >= dy {
                error += dy;
                x0 += sx;
            }
            if twice <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }
}

fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0F, 0x10, 0x10, 0x10, 0x10, 0x10, 0x0F],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0F, 0x10, 0x10, 0x13, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x12, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
        '6' => [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E],
        ':' => [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x04, 0x04, 0x08],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        '?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '\'' => [0x04, 0x04, 0x02, 0x00, 0x00, 0x00, 0x00],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::Canvas;
    use crate::assets::SpriteSheet;

    #[test]
    fn sixteen_pixel_obj_flips_both_tile_order_and_local_pixels() {
        let mut palette = [0;16];
        for i in 1..16 { palette[i] = i as u32; }
        let tiles = (0..4).map(|tile| std::array::from_fn(|pixel| (1+tile*3+pixel%3) as u8)).collect();
        let sprite = SpriteSheet { tiles, palette };
        let mut normal = Canvas::new();
        normal.sprite_16x16(&sprite, [0,1,2,3], 0,0,false,false);
        for horizontal in [false,true] {
            for vertical in [false,true] {
                let mut flipped = Canvas::new();
                flipped.sprite_16x16(&sprite, [0,1,2,3], 0,0,horizontal,vertical);
                for y in 0..16 { for x in 0..16 {
                    let sx = if horizontal {15-x} else {x};
                    let sy = if vertical {15-y} else {y};
                    assert_eq!(flipped.pixels[y*super::WIDTH+x], normal.pixels[sy*super::WIDTH+sx]);
                }}
            }
        }
    }

    #[test]
    fn horizontal_flip_reverses_the_entire_six_tile_actor() {
        let mut palette = [0; 16];
        palette[1] = 0xff0000;
        palette[2] = 0x0000ff;
        let sprite = SpriteSheet {
            tiles: vec![[1; 64], [2; 64]],
            palette,
        };
        let mut normal = Canvas::new();
        let mut flipped = Canvas::new();
        normal.sprite_16x24(&sprite, [0, 1, 0, 1, 0, 1], 0, 0, false);
        flipped.sprite_16x24(&sprite, [0, 1, 0, 1, 0, 1], 0, 0, true);
        for y in 0..24 {
            for x in 0..16 {
                assert_eq!(normal.pixels[y * super::WIDTH + x],
                    flipped.pixels[y * super::WIDTH + 15 - x]);
            }
        }
    }
}
