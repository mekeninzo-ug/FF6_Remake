/// Entrance records interpreted by field/entrance.asm. Map-word flags remain
/// intact so parent maps, z-levels and world transitions can be ported explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldExit {
    pub source_x: u8,
    pub source_y: u8,
    pub extent: u8,
    pub vertical: bool,
    pub map_word: u16,
    pub destination_x: i16,
    pub destination_y: i16,
}

impl FieldExit {
    pub fn destination_map(self) -> u16 {
        self.map_word & 0x01ff
    }

    pub fn facing(self) -> usize {
        ((self.map_word >> 12) & 3) as usize
    }

    pub fn contains(self, x: i16, y: i16) -> bool {
        let start_x = i16::from(self.source_x);
        let start_y = i16::from(self.source_y);
        if self.vertical {
            x == start_x && y >= start_y && y <= start_y + i16::from(self.extent)
        } else {
            y == start_y && x >= start_x && x <= start_x + i16::from(self.extent)
        }
    }
}

pub fn decode_entrances(short: &[u8], long: &[u8]) -> Option<Vec<FieldExit>> {
    if short.len() % 6 != 0 || long.len() % 7 != 0 {
        return None;
    }
    let mut exits = Vec::new();
    // CheckEntrances invokes CheckLongEntrance before CheckShortEntrance.
    for record in long.chunks_exact(7) {
        exits.push(FieldExit {
            source_x: record[0], source_y: record[1],
            extent: record[2] & 0x7f, vertical: record[2] & 0x80 != 0,
            map_word: u16::from_le_bytes([record[3], record[4]]),
            destination_x: i16::from(record[5]), destination_y: i16::from(record[6]),
        });
    }
    for record in short.chunks_exact(6) {
        exits.push(FieldExit {
            source_x: record[0], source_y: record[1], extent: 0, vertical: false,
            map_word: u16::from_le_bytes([record[2], record[3]]),
            destination_x: i16::from(record[4]), destination_y: i16::from(record[5]),
        });
    }
    Some(exits)
}

#[cfg(test)]
mod tests {
    use super::decode_entrances;

    #[test]
    fn patched_narshe_exit_includes_both_endpoints() {
        let exits = decode_entrances(&[], &[35, 1, 5, 39, 0, 26, 52]).unwrap();
        let exit = exits[0];
        assert!(exit.contains(35, 1));
        assert!(exit.contains(40, 1));
        assert!(!exit.contains(34, 1));
        assert!(!exit.contains(41, 1));
        assert!(!exit.contains(38, 2));
        assert_eq!((exit.destination_map(), exit.destination_x, exit.destination_y, exit.facing()),
            (39, 26, 52, 0));
    }

    #[test]
    fn vertical_and_point_triggers_preserve_flags_and_order() {
        let exits = decode_entrances(&[10, 20, 19, 0x2c, 38, 2],
            &[10, 20, 0x82, 41, 0x10, 38, 33]).unwrap();
        assert!(exits[0].contains(10, 22));
        assert!(!exits[0].contains(10, 23));
        assert!(!exits[0].contains(11, 20));
        assert_eq!(exits[0].destination_map(), 41);
        assert_eq!(exits[1].map_word, 0x2c13);
        assert_eq!(exits[1].facing(), 2);
        assert!(exits[1].contains(10, 20));
        assert!(!exits[1].contains(10, 21));
        assert!(decode_entrances(&[0], &[]).is_none());
        assert!(decode_entrances(&[], &[0; 6]).is_none());
    }
}
