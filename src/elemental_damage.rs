//! Target C2/0B9B..0C28 resolution after damage modifiers. The separate
//! accumulator helper matches C2/0C82..0C9D; neither applies current HP/MP.
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default)]
pub struct Context {
    pub power:u8,pub element:u8,pub flags0:u8,pub flags2:u8,pub status1:u8,
    pub monster_flags:u8,pub attack_status1:u8,pub blocked_elements:u8,
    pub absorb:u8,pub null:u8,pub half:u8,pub weak:u8,pub special:u8,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Contribution {pub amount:u16,pub flags:u8}
impl Contribution {pub fn healing(self)->bool {self.flags&1!=0}}
pub fn resolve(mut amount:u16,c:Context)->Result<Option<Contribution>,String> {
    if c.power==0 {return Ok(None);}
    let mut flags=c.flags2;
    if c.status1&0x40!=0 {amount=0;flags=0;}
    else {
        if c.flags0&8!=0 {
            if c.monster_flags&0x80!=0 {
                if c.attack_status1&0x82!=0 {return Ok(None);}
                flags=0;
                if c.flags2&2!=0 {flags^=1;}
            } else if c.status1&2!=0 && c.flags2&2!=0 {flags^=1;}
        }
        if c.element!=0 {
            if c.element&!c.blocked_elements==0 {amount=0;}
            else if c.absorb&c.element!=0 {flags^=1;}
            else if c.null&c.element!=0 {amount=0;}
            else if c.half&c.element!=0 {amount>>=1;}
            else if c.weak&c.element!=0 && amount&0x8000==0 {amount<<=1;}
        }
    }
    if c.special==4 {return Err("Ultima Weapon target-damage effect is not implemented".into());}
    Ok(Some(Contribution {amount,flags}))
}
/// FFFF means no accumulated value, rather than 65535 damage. Zero amounts
/// still replace that sentinel with zero. Caller handles routing/drain/status.
pub fn accumulate(current:u16,amount:u16)->u16 {
    (u32::from(if current==65535 {0}else{current})+u32::from(amount)).min(9999) as u16
}
pub fn capture(directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let mut seed=0x6ff6_u32;let mut rows=String::from("case,amount,power,element,flags0,flags2,status1,monster_flags,attack_status1,blocked,absorb,null,half,weak,result,flags\n");
    for case in 0..16384usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let amount=(u16::from(next())<<8)|u16::from(next());
        let c=Context {power:if case%13==0 {0}else{60},element:next(),flags0:if case%2==0 {8}else{0},flags2:[0,1,2,3,0x80,0x81][case/3%6],status1:[0,2,0x40,0x42,0x80,0x82][case/5%6],monster_flags:if case/7%2==0 {0x80}else{0},attack_status1:[0,2,0x80,0x82][case/11%4],blocked_elements:next(),absorb:next(),null:next(),half:next(),weak:next(),special:0};
        let result=resolve(amount,c)?;let value=result.map(|r|r.amount.to_string()).unwrap_or_default();let flags=result.map(|r|r.flags.to_string()).unwrap_or_default();
        writeln!(rows,"{case},{amount},{},{},{},{},{},{},{},{},{},{},{},{},{value},{flags}",c.power,c.element,c.flags0,c.flags2,c.status1,c.monster_flags,c.attack_status1,c.blocked_elements,c.absorb,c.null,c.half,c.weak).unwrap();
    }
    fs::write(directory.join("resolution.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("current,amount,healing,slot,result\n");
    for current in [0,1,9998,9999,10000,65534,65535] {for amount in [0,1,2,5000,9999,10000,32768,65535] {
        writeln!(rows,"{current},{amount},{},{},{}",amount%2,current%10,accumulate(current,amount)).unwrap();
    }}
    for case in 0..4096 {
        seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);let current=(seed>>16) as u16;
        seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);let amount=(seed>>16) as u16;
        writeln!(rows,"{current},{amount},{},{},{}",case%2,case%10,accumulate(current,amount)).unwrap();
    }
    fs::write(directory.join("accumulation.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn elemental_priority_and_multielement_blocking_follow_source_order() {
        let c=Context {power:60,element:3,blocked_elements:1,absorb:1,null:2,weak:3,..Context::default()};
        assert_eq!(resolve(100,c).unwrap(),Some(Contribution {amount:100,flags:1}));
        assert!(resolve(100,c).unwrap().unwrap().healing());
        assert_eq!(resolve(100,Context {blocked_elements:3,..c}).unwrap().unwrap().amount,0);
        assert_eq!(resolve(101,Context {absorb:0,null:0,half:2,..c}).unwrap().unwrap().amount,50);
    }
    #[test]fn petrify_precedes_undead_skip_and_weakness_avoids_word_overflow() {
        let c=Context {power:60,flags0:8,flags2:1,monster_flags:128,attack_status1:128,..Context::default()};
        assert_eq!(resolve(123,c).unwrap(),None);
        assert_eq!(resolve(123,Context {status1:0x40,..c}).unwrap(),Some(Contribution {amount:0,flags:0}));
        let c=Context {power:60,element:1,weak:1,..Context::default()};
        assert_eq!(resolve(32767,c).unwrap().unwrap().amount,65534);
        assert_eq!(resolve(32768,c).unwrap().unwrap().amount,32768);
        assert!(resolve(1,Context {special:4,..c}).is_err());
    }
    #[test]fn accumulation_respects_sentinel_zero_and_cap() {
        assert_eq!(accumulate(65535,0),0);assert_eq!(accumulate(65535,123),123);
        assert_eq!(accumulate(9998,2),9999);assert_eq!(accumulate(65534,65535),9999);
    }
}
