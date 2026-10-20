//! Complete bounded CheckHit C2/220D..23EC, returning effects for the battle
//! state. Reflection continuation, status commit and rendering are separate.
use crate::{battle_accuracy,battle_hit_gates::{self,Context as Gates,Route},battle_rng::{BattleRng,RngTable}};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default)]
pub struct Context {
    pub gates:Gates,pub status2:u8,pub evade:u8,pub attacker_blind:bool,
    pub beads:bool,pub retort:bool,pub guaranteed:bool,
    pub physical_block:u8,pub magical_block:u8,pub target_mask:u16,pub show_miss:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct State {
    pub attack_b2:u8,pub attack_status:[u8;4],pub weapon_spell:u8,
    pub pending_remove:u16,pub reflected:u16,pub miss_targets:u16,
    pub golem_target:u8,pub dog_target:u8,pub block_graphic:u8,
}
impl Default for State {fn default()->Self {Self {attack_b2:0,attack_status:[0;4],weapon_spell:0,pending_remove:0,reflected:0,miss_targets:0,golem_target:255,dog_target:255,block_graphic:0}}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Outcome {Hit,Miss,Reflect}
pub fn check(c:Context,s:&mut State,rng:&mut BattleRng,table:&RngTable)->Result<Outcome,String> {
    let g=c.gates;
    if g.actor_offset>18 || g.actor_offset&1!=0 {return Err("CheckHit target offset must identify one of ten actors".into());}
    let gate=battle_hit_gates::check(g,rng,table);
    if gate.remove_vanish {s.pending_remove|=0x10;}
    let mut route=gate.route;
    if route==Route::Ordinary {
        let accuracy=battle_accuracy::check(battle_accuracy::Context {physical:g.flags0&1!=0,hit_rate:g.hit_rate,evade:c.evade,magic_evade:g.magic_evade,status12:u16::from(g.status1)|(u16::from(c.status2)<<8),status34:u16::from(g.status3)|(u16::from(g.status4)<<8),attacker_blind:c.attacker_blind,beads:c.beads,retort:c.retort,guaranteed:c.guaranteed},rng,table);
        if accuracy.remove_image {s.pending_remove|=0x400;}
        route=if accuracy.hit {Route::Hit} else if g.flags0&1!=0 && c.status2&4!=0 {Route::DirectMiss}else{Route::Miss};
    }
    if route==Route::Hit {return Ok(Outcome::Hit);}
    if route==Route::Reflect {s.reflected|=c.target_mask;return Ok(Outcome::Reflect);}
    if route==Route::Miss && g.status1&0x1a==0 && g.actor_offset<8 {
        let physical=g.flags0&1!=0;
        let mask=(if physical {c.physical_block}else{c.magical_block})|gate.forced_block;
        if mask!=0 {
            let selected=rng.bit(table,u16::from(mask)) as u8;
            let offset=g.actor_offset+u8::from(!physical);
            if selected&0x40!=0 {s.dog_target=offset;}
            if selected&0x20!=0 {s.golem_target=offset;}
            s.block_graphic=selected.trailing_zeros() as u8+1;
            if s.block_graphic>=6 {s.attack_status=[0;4];return Ok(Outcome::Hit);}
        }
    }
    s.attack_b2|=2;s.weapon_spell=0;
    if c.show_miss {s.miss_targets|=c.target_mask;}
    Ok(Outcome::Miss)
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;
    let mut rows=String::from("case,seed,b3,flags0,flags1,flags2,status1,status2,status3,status4,monster,protection,command,special,targets,offset,golem,rate,level,stamina,magic_evade,evade,blind,beads,retort,guaranteed,physical_block,magical_block,mask,show_miss,outcome,b2,weapon,remove,reflected,miss_targets,golem_target,dog_target,graphic,status,index\n");
    let mut seed=0x6ff6u32;
    for case in 0..32768usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let start=next();let offset=[0,2,6,8,12,18][case/17%6];
        let gates=Gates {attack_b3:next()&128,flags0:next()&7,flags1:next()&2,flags2:[0,16,32,64,80,96][case/3%6],status1:[0,1,2,8,16,64,128][case/5%7],status3:[0,4,8,16,128][case/7%5],status4:[0,2,4,64][case/11%4],monster_flags:next()&128,protection:next()&4,command:[0,6,29,2][case/13%4],special:if case%11==0 {1}else{0},targets:if case%13==0 {2}else{1},actor_offset:offset,golem_hp:if case/19%2==0 {100}else{0},hit_rate:if case%31==0 {255}else{next()},level:next(),stamina:next(),magic_evade:next()};
        let c=Context {gates,status2:[0,2,4,32,64,128][case/23%6],evade:next(),attacker_blind:case%2==0,beads:case/29%2==0,retort:case/31%3==0,guaranteed:case/37%11==0,physical_block:next(),magical_block:next(),target_mask:if offset<8 {1<<(offset/2)}else{1<<(offset/2+4)},show_miss:case%3==0};
        let mut state=State {attack_b2:0x40,attack_status:[1,2,4,8],weapon_spell:19,pending_remove:0x2000,reflected:0x4000,miss_targets:0x4000,block_graphic:9,..State::default()};
        let mut rng=BattleRng {index:start};let out=check(c,&mut state,&mut rng,&table)?;
        let outcome=match out {Outcome::Hit=>0,Outcome::Miss=>1,Outcome::Reflect=>2};
        writeln!(rows,"{case},{start},{},{},{},{},{},{},{},{},{},{},{},{},{},{offset},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{outcome},{},{},{},{},{},{},{},{},{:02X}{:02X}{:02X}{:02X},{}",gates.attack_b3,gates.flags0,gates.flags1,gates.flags2,gates.status1,c.status2,gates.status3,gates.status4,gates.monster_flags,gates.protection,gates.command,gates.special,gates.targets,gates.golem_hp,gates.hit_rate,gates.level,gates.stamina,gates.magic_evade,c.evade,c.attacker_blind as u8,c.beads as u8,c.retort as u8,c.guaranteed as u8,c.physical_block,c.magical_block,c.target_mask,c.show_miss as u8,state.attack_b2,state.weapon_spell,state.pending_remove,state.reflected,state.miss_targets,state.golem_target,state.dog_target,state.block_graphic,state.attack_status[0],state.attack_status[1],state.attack_status[2],state.attack_status[3],rng.index).unwrap();
    }
    fs::write(directory.join("check-hit.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn golem_block_clears_attack_status_and_returns_hit() {
        let mut s=State {attack_status:[1;4],..State::default()};let mut rng=BattleRng::default();
        let c=Context {gates:Gates {targets:1,golem_hp:100,..Gates::default()},..Context::default()};
        assert_eq!(check(c,&mut s,&mut rng,&RngTable::identity()).unwrap(),Outcome::Hit);assert_eq!(s.attack_status,[0;4]);assert_eq!((s.golem_target,s.block_graphic),(1,6));
    }
    #[test]fn image_bypasses_block_graphics_and_marks_miss() {
        let mut s=State {weapon_spell:19,..State::default()};let mut rng=BattleRng::default();
        let c=Context {gates:Gates {command:29,flags0:1,..Gates::default()},status2:4,physical_block:255,target_mask:1,show_miss:true,..Context::default()};
        assert_eq!(check(c,&mut s,&mut rng,&RngTable::identity()).unwrap(),Outcome::Miss);assert_eq!((s.pending_remove,s.miss_targets,s.weapon_spell,rng.index),(0x400,1,0,1));
    }
    #[test]fn reflection_preserves_weapon_and_attack_flags() {
        let mut s=State {weapon_spell:19,..State::default()};let mut rng=BattleRng::default();
        let c=Context {gates:Gates {status3:128,..Gates::default()},target_mask:2,..Context::default()};
        assert_eq!(check(c,&mut s,&mut rng,&RngTable::identity()).unwrap(),Outcome::Reflect);assert_eq!((s.reflected,s.weapon_spell,s.attack_b2),(2,19,0));
    }
    #[test]fn hit_then_native_damage_pipeline_queues_guard_death() {
        use crate::{damage::{BaseDamage,Modifiers},elemental_damage as elements,damage_application as hp};
        let table=RngTable::identity();let mut rng=BattleRng::default();let mut state=State::default();
        let c=Context {gates:Gates {command:29,actor_offset:8,hit_rate:255,..Gates::default()},target_mask:0x100,..Context::default()};
        assert_eq!(check(c,&mut state,&mut rng,&table).unwrap(),Outcome::Hit);assert_eq!(rng.index,0);
        let base=BaseDamage {level:3,power:60,stat:39,command:29,..BaseDamage::default()}.calculate();
        let modified=Modifiers {rng:rng.next(&table),magic_defense:140,..Modifiers::default()}.calculate(base);
        let contribution=elements::resolve(modified,elements::Context {power:60,element:1,..elements::Context::default()}).unwrap().unwrap();
        let mut target=hp::Target {hp:40,mp:15,max_hp:40,max_mp:15,status1:0,pending_status:0,update_flags:0,monster_flags:0,weapon_spell:0};
        hp::apply(&mut target,hp::Damage {taken:elements::accumulate(65535,contribution.amount),healed:65535,invincible:false,golem:255,dog:255,affects_mp:false});
        assert_eq!((modified,target.hp,target.pending_status,rng.index),(182,0,128,1));
    }
}
