//! Tile-aligned field movement using FF6's 8.8 pixel units and TWUE dash speed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerMotion {
    pub direction: u8,
    pub speed: u8,
    pub progress: u16,
    pub active: bool,
}

impl Default for PlayerMotion {
    fn default() -> Self { Self { direction: 0, speed: 2, progress: 0, active: false } }
}

/// Target $C0:D674: base 2, plus Cancel ($07 bit 7), plus Sprint Shoes
/// ($11DF bit 5), supplied by enabled members of the active party.
pub const fn patched_speed(cancel: bool, sprint_shoes: bool) -> u8 {
    2 + cancel as u8 + sprint_shoes as u8
}

/// Sliding Dash's target $C0:D68D coordinate adjustment for on-foot gait.
pub const fn patched_walk_phase(coordinate: i32, direction: u8) -> usize {
    let shifted = (coordinate as u8) >> 2;
    let adjusted = shifted + if direction == 1 || direction == 2 { 1 } else { 0 };
    ((adjusted >> 1) & 3) as usize
}

impl PlayerMotion {
    pub fn start(&mut self, direction: u8, speed: u8) {
        self.direction = direction & 3;
        self.speed = speed.clamp(2,4);
        self.progress = 0;
        self.active = true;
    }

    pub fn offset(&self) -> (i32,i32) {
        if !self.active { return (0,0); }
        let (dx,dy) = [(0,-1),(1,0),(0,1),(-1,0)][usize::from(self.direction)];
        (dx*i32::from(self.progress),dy*i32::from(self.progress))
    }

    /// Called after commands once per logical frame; returns a completed tile.
    pub fn advance(&mut self) -> Option<(i16,i16)> {
        if !self.active { return None; }
        self.progress += 64u16 << self.speed;
        if self.progress < 4096 { return None; }
        let direction = self.direction;
        self.active = false;
        self.progress = 0;
        Some([(0,-1),(1,0),(0,1),(-1,0)][usize::from(direction)])
    }

    pub fn restore(direction: u8, speed: u8, progress: u16, active: bool) -> Option<Self> {
        if direction > 3 || !(2..=4).contains(&speed) || progress >= 4096
            || (!active && progress != 0) || progress % (64u16 << speed) != 0 { return None; }
        Some(Self { direction, speed, progress, active })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patched_controls_select_one_two_and_four_pixels_per_frame() {
        assert_eq!([patched_speed(false,false),patched_speed(true,false),patched_speed(false,true),patched_speed(true,true)], [2,3,3,4]);
        for speed in 2..=4 {
            let mut motion = PlayerMotion::default();
            motion.start(1,speed);
            let frames = 64 >> speed;
            for frame in 1..frames {
                assert_eq!(motion.advance(),None);
                assert_eq!(motion.offset(),(frame*(64<<speed),0));
            }
            assert_eq!(motion.advance(),Some((1,0)));
            assert!(!motion.active);
            assert_eq!(motion.offset(),(0,0));
        }
    }
    #[test]
    fn sliding_dash_hook_adjusts_right_and_down_phases() {
        assert_eq!((0..16).map(|x| patched_walk_phase(x,0)).collect::<Vec<_>>(),
            vec![0,0,0,0,0,0,0,0,1,1,1,1,1,1,1,1]);
        assert_eq!((0..16).map(|x| patched_walk_phase(x,1)).collect::<Vec<_>>(),
            vec![0,0,0,0,1,1,1,1,1,1,1,1,2,2,2,2]);
        for x in 0..512 {assert_eq!(patched_walk_phase(x,1),patched_walk_phase(x,2));}
    }

    #[test]
    fn cardinal_offsets_and_saved_motion_are_checked() {
        for (direction,expected) in [(0,(0,-256)),(1,(256,0)),(2,(0,256)),(3,(-256,0))] {
            let mut motion = PlayerMotion::default();motion.start(direction,2);motion.advance();
            assert_eq!(motion.offset(),expected);
            assert_eq!(PlayerMotion::restore(direction,2,256,true),Some(motion));
        }
        assert!(PlayerMotion::restore(4,2,0,false).is_none());
        assert!(PlayerMotion::restore(0,3,256,true).is_none());
        assert!(PlayerMotion::restore(0,2,256,false).is_none());
    }
}
