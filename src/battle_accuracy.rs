//! Ordinary accuracy stage C2/22FB..239B, including target Evade bug fix.
//! Called after initial CheckHit immunity, Vanish, protection and special gates.
use crate::battle_rng::{BattleRng,RngTable};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default)]
pub struct Context {
    pub physical:bool,pub hit_rate:u8,pub evade:u8,pub magic_evade:u8,
    pub status12:u16,pub status34:u16,pub attacker_blind:bool,
    pub beads:bool,pub retort:bool,pub guaranteed:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Result {pub hit:bool,pub remove_image:bool}
pub fn check(c:Context,rng:&mut BattleRng,table:&RngTable)->Result {
    let hit=Result {hit:true,remove_image:false};
    if c.status12&0x8040!=0 || c.status34&0x0210!=0 || c.guaranteed || c.hit_rate==255 {return hit;}
    let mut rate=c.hit_rate;
    let block=if c.physical {
        if c.retort {return hit;}
        if c.status12&0x0400!=0 {return Result {hit:false,remove_image:rng.next(table)<64};}
        if c.attacker_blind {rate>>=1;}
        if c.beads {rate>>=1;}
        if c.status12&0x2003!=0 || c.status34&0x0404!=0 {
            rate=(u16::from(rate)+u16::from(rate>>2)+u16::from((rate>>1)&1)).min(255) as u8;
        }
        if c.status12&0x4204!=0 || c.status34&8!=0 {rate-=rate>>2;}
        c.evade
    } else {c.magic_evade};
    let threshold=(u16::from(block)*u16::from(rate))>>8;
    Result {hit:u16::from(rng.below(table,100))<threshold,remove_image:false}
}
pub fn capture(root:&Path,directory:&Path)->std::result::Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;
    let mut rows=String::from("case,seed,physical,rate,evade,magic_evade,status12,status34,blind,beads,retort,guaranteed,hit,remove_image,index\n");
    let mut seed=0x6ff6u32;
    for case in 0..16384usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let start=next();let c=Context {physical:case%2==0,hit_rate:if case%31==0 {255}else{next()},evade:next(),magic_evade:next(),status12:[0,1,2,0x2000,0x400,0x40,0x8000,0x4000,0x200,4,0x6207][case/3%11],status34:[0,4,0x400,8,0x10,0x200,0xc][case/5%7],attacker_blind:case/7%2!=0,beads:case/11%2!=0,retort:case/13%3==0,guaranteed:case/17%7==0};
        let mut rng=BattleRng {index:start};let out=check(c,&mut rng,&table);
        writeln!(rows,"{case},{start},{},{},{},{},{},{},{},{},{},{},{},{},{}",c.physical as u8,c.hit_rate,c.evade,c.magic_evade,c.status12,c.status34,c.attacker_blind as u8,c.beads as u8,c.retort as u8,c.guaranteed as u8,out.hit as u8,out.remove_image as u8,rng.index).unwrap();
    }
    fs::write(directory.join("accuracy.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn automatic_hits_preserve_rng_and_image() {
        let mut rng=BattleRng {index:20};let c=Context {hit_rate:255,status12:0x400,..Context::default()};
        assert_eq!(check(c,&mut rng,&RngTable::identity()),Result {hit:true,remove_image:false});assert_eq!(rng.index,20);
    }
    #[test]fn physical_uses_evade_and_magic_uses_magic_evade() {
        let table=RngTable::identity();let mut rng=BattleRng {index:100};
        let c=Context {physical:true,hit_rate:100,evade:255,magic_evade:0,..Context::default()};
        assert!(check(c,&mut rng,&table).hit);assert!(!check(Context {physical:false,..c},&mut rng,&table).hit);
    }
    #[test]fn image_always_misses_and_consumes_one_byte() {
        let mut rng=BattleRng {index:0};let out=check(Context {physical:true,status12:0x400,..Context::default()},&mut rng,&RngTable::identity());
        assert_eq!(out,Result {hit:false,remove_image:true});assert_eq!(rng.index,1);
    }
}
