//! CheckHit's gates before ordinary accuracy. A Miss is still subject to
//! C2/22B5 evasion animation/status processing, which the caller must perform.
use crate::battle_rng::{BattleRng,RngTable};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default)]
pub struct Context {
    pub attack_b3:u8,pub flags0:u8,pub flags1:u8,pub flags2:u8,
    pub status1:u8,pub status3:u8,pub status4:u8,pub monster_flags:u8,
    pub protection:u8,pub command:u8,pub special:u8,pub targets:u8,
    pub actor_offset:u8,pub golem_hp:u16,pub hit_rate:u8,pub level:u8,
    pub stamina:u8,pub magic_evade:u8,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Route {Ordinary,Hit,Miss,DirectMiss,Reflect}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Result {pub route:Route,pub remove_vanish:bool,pub forced_block:u8}
pub fn check(c:Context,rng:&mut BattleRng,table:&RngTable)->Result {
    let mut out=Result {route:Route::Ordinary,remove_vanish:false,forced_block:0};
    if c.attack_b3&0x80!=0 && c.status1&0x10!=0 {
        if c.flags2&0x40!=0 {out.remove_vanish=true;}
        else {out.route=if c.flags0&1!=0 {Route::Miss}else{Route::Hit};return out;}
    }
    if c.flags1&2==0 && c.status3&0x80!=0 {out.route=Route::Reflect;return out;}
    if c.flags0&2!=0 && c.protection&4!=0 {out.route=Route::Miss;return out;}
    if c.flags0&4!=0 && (c.status1^c.monster_flags)&0x80==0 {out.route=Route::Miss;return out;}
    if (c.command==0 || c.command==6) && c.special==0 && c.targets==1 && c.actor_offset<8 {
        if c.status4&0x40!=0 && rng.next(table)&1!=0 {out.forced_block=0x40;out.route=Route::Miss;return out;}
        if c.golem_hp!=0 {out.forced_block=0x20;out.route=Route::Miss;return out;}
    }
    if c.flags2&0x20!=0 {out.route=Route::Hit;}
    else if c.flags2&0x40!=0 {
        // SNES hardware division by zero gives remainder equal to dividend's
        // low byte. Level zero therefore passes the source remainder test.
        let remainder=if c.hit_rate==0 {c.level}else{c.level%c.hit_rate};
        out.route=if remainder==0 {Route::Hit}else{Route::DirectMiss};
    } else if c.flags2&0x10!=0 {
        let threshold=(u16::from(c.magic_evade)*u16::from(c.hit_rate))>>8;
        let missed=u16::from(rng.below(table,100))>=threshold;
        let resisted=if missed {false}else{c.stamina>=rng.next(table)&0x7f};
        out.route=if missed || resisted {Route::Miss}else{Route::Hit};
    }
    out
}
pub fn capture(root:&Path,directory:&Path)->std::result::Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;
    let mut rows=String::from("case,seed,b3,flags0,flags1,flags2,status1,status3,status4,monster,protection,command,special,targets,offset,golem,rate,level,stamina,magic_evade,route,vanish,block,index\n");
    let mut seed=0x6ff6u32;
    for case in 0..32768usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let start=next();let c=Context {attack_b3:if case%2==0 {0x80}else{0},flags0:next()&7,flags1:next()&2,flags2:[0,0x10,0x20,0x40,0x50,0x60][case/3%6],status1:[0,0x10,0x80,0x90,2,8][case/5%6],status3:next()&0x80,status4:next()&0x40,monster_flags:next()&0x80,protection:next()&4,command:[0,6,29,2][case/7%4],special:if case%11==0 {1}else{0},targets:if case%13==0 {2}else{1},actor_offset:[0,2,6,8,12,18][case/17%6],golem_hp:if case/19%2==0 {100}else{0},hit_rate:if case%31==0 {0}else{next()},level:next(),stamina:next(),magic_evade:next()};
        let mut rng=BattleRng {index:start};let out=check(c,&mut rng,&table);
        let route=match out.route {Route::Ordinary=>0,Route::Hit=>1,Route::Miss=>2,Route::DirectMiss=>3,Route::Reflect=>4};
        writeln!(rows,"{case},{start},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{route},{},{},{}",c.attack_b3,c.flags0,c.flags1,c.flags2,c.status1,c.status3,c.status4,c.monster_flags,c.protection,c.command,c.special,c.targets,c.actor_offset,c.golem_hp,c.hit_rate,c.level,c.stamina,c.magic_evade,out.remove_vanish as u8,out.forced_block,rng.index).unwrap();
    }
    fs::write(directory.join("gates.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn vanish_bypasses_death_protection_before_special_mode() {
        let mut rng=BattleRng {index:4};let table=RngTable::identity();
        let c=Context {attack_b3:128,status1:16,flags0:2,protection:4,..Context::default()};
        assert_eq!(check(c,&mut rng,&table).route,Route::Hit);assert_eq!(rng.index,4);
        let out=check(Context {flags2:0x40,..c},&mut rng,&table);assert!(out.remove_vanish);assert_eq!(out.route,Route::Miss);
    }
    #[test]fn interceptor_precedes_golem_and_uses_rng_low_bit() {
        let c=Context {targets:1,status4:64,golem_hp:100,..Context::default()};let table=RngTable::identity();
        let mut rng=BattleRng {index:0};assert_eq!(check(c,&mut rng,&table).forced_block,64);
        let mut rng=BattleRng {index:1};assert_eq!(check(c,&mut rng,&table).forced_block,32);assert_eq!(rng.index,2);
    }
    #[test]fn level_mode_and_stamina_preserve_branch_rng_order() {
        let table=RngTable::identity();let mut rng=BattleRng::default();
        assert_eq!(check(Context {flags2:64,hit_rate:5,level:10,..Context::default()},&mut rng,&table).route,Route::Hit);
        assert_eq!(check(Context {flags2:64,hit_rate:0,level:1,..Context::default()},&mut rng,&table).route,Route::DirectMiss);
        assert_eq!(check(Context {flags2:16,..Context::default()},&mut rng,&table).route,Route::Miss);assert_eq!(rng.index,1);
    }
}
