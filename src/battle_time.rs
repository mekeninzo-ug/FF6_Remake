//! Bounded arithmetic from C2/111B, C2/1193, C2/11BB. Action queues,
//! eligibility/status gates and random initial gauges are separate systems.

/// C2/09D2: hardware multiplication uses the high byte (/256), despite
/// the source comment saying /255. Slow takes precedence over haste.
pub fn gauge_increment(speed: u8, status3: u8, monster: bool, battle_speed: u8) -> u16 {
    gauge_increment_with_factor(speed,status3,monster,255-(battle_speed&7)*24)
}
/// The dispatcher receives the already computed RAM factor at 7E/3A90.
pub fn gauge_increment_with_factor(speed:u8,status3:u8,monster:bool,factor:u8)->u16 {
    let status_constant: u16=if status3&4!=0 {32} else if status3&8!=0 {84} else {64};
    let mut adjusted=u16::from(speed.wrapping_add(20));
    if monster {adjusted=(adjusted*u16::from(factor))>>8;}
    (adjusted*(status_constant+status_constant/2))>>4
}

pub fn capture(directory: &std::path::Path) -> Result<(),String> {
    use std::fmt::Write;
    let mut rows=String::from("speed,status,monster,battle_speed,increment\n");
    for speed in 0..=255 {for status in [0,4,8,12] {for monster in [false,true] {for setting in 0..8 {
        writeln!(rows,"{speed},{status},{},{setting},{}",u8::from(monster),gauge_increment(speed,status,monster,setting)).unwrap();
    }}}}
    std::fs::write(directory.join("gauge-increments.csv"),rows).map_err(|e|e.to_string())?;
    let mut clock=BattleClock::default();
    let mut gauge=Gauge {counter:0xf000,increment:gauge_increment(30,0,true,0),advance_counter:0x1234,advance_duration:7};
    let mut rows=String::from("frame,turns,tick,gauge,ready,advance,enqueue\n");
    for frame in 0..512u16 {
        let tick=clock.update(frame as u8,if (40..60).contains(&frame) {1} else {0},1);
        let ready=tick && gauge.tick();
        let enqueue=tick && frame>=400 && gauge.tick_advance();
        writeln!(rows,"{frame},{},{},{},{},{},{}",clock.turns,u8::from(tick),gauge.counter,u8::from(ready),gauge.advance_counter,u8::from(enqueue)).unwrap();
    }
    std::fs::write(directory.join("clock-gauge.csv"),rows).map_err(|e|e.to_string())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BattleClock { pub previous_frame: u8, pub turns: u16 }
impl BattleClock {
    /// Uses the cartridge's wrapping eight-bit frame counter. A wait pause
    /// resets the previous frame; it does not accumulate elapsed turns.
    pub fn update(&mut self, current_frame: u8, program_stop: u8, wait_mode: u8) -> bool {
        if program_stop & wait_mode != 0 {
            self.previous_frame=current_frame;
            return false;
        }
        if current_frame==self.previous_frame || current_frame==self.previous_frame.wrapping_add(1) {return false;}
        self.turns=self.turns.wrapping_add(1);
        self.previous_frame=current_frame;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gauge {
    pub counter: u16,
    pub increment: u16,
    pub advance_counter: u16,
    pub advance_duration: u8,
}
impl Gauge {
    /// The caller must apply present/status gates. A zero high byte is the
    /// source's ready sentinel; 65535 is not the ready representation.
    pub fn tick(&mut self) -> bool {
        if self.counter>>8==0 {return false;}
        let (next,carry)=self.counter.overflowing_add(self.increment);
        self.counter=next;
        if carry {
            self.counter&=0xff;
            self.advance_counter&=0xff;
            self.advance_duration=0xff;
        }
        carry
    }
    /// Returns an enqueue request, not a completed action. The source caller
    /// must set its queued flag to avoid calling this again for that action.
    pub fn tick_advance(&mut self) -> bool {
        let (next,carry)=self.advance_counter.overflowing_add(self.increment>>1);
        self.advance_counter=next;
        if carry || (next>>8)>=u16::from(self.advance_duration) {
            self.advance_duration=0xff;
            return true;
        }
        false
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn frame_wrap_pause_and_no_catchup_match_source() {
        let mut clock=BattleClock {previous_frame:254,turns:65535};
        assert!(!clock.update(255,0,1));assert!(clock.update(0,0,1));assert_eq!(clock.turns,0);
        assert!(!clock.update(100,1,1));assert!(!clock.update(101,0,1));assert!(clock.update(102,0,1));
        assert!(clock.update(200,0,1));assert_eq!(clock.turns,2);
    }
    #[test] fn gauge_carry_clears_only_high_bytes() {
        for counter in 0..=65535u32 {
            for increment in [0u16,1,300,65535] {
                let mut gauge=Gauge {counter:counter as u16,increment,advance_counter:0xabcd,advance_duration:7};
                let ready=gauge.tick();
                let active=counter>=256;
                let sum=counter+u32::from(increment);
                assert_eq!(ready,active && sum>65535);
                assert_eq!(gauge.counter,if !active {counter as u16} else if ready {(sum&255) as u16} else {sum as u16});
                assert_eq!(gauge.advance_counter,if ready {0xcd} else {0xabcd});
                assert_eq!(gauge.advance_duration,if ready {255} else {7});
            }
        }
    }
    #[test] fn advance_threshold_and_overflow_request_enqueue() {
        let mut gauge=Gauge {counter:0,increment:100,advance_counter:0x1ce,advance_duration:2};
        assert!(gauge.tick_advance());assert_eq!(gauge.advance_counter,0x200);
        gauge.advance_counter=65530;gauge.advance_duration=255;
        assert!(gauge.tick_advance());assert_eq!(gauge.advance_counter,44);
    }
}
