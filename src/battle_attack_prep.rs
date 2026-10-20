//! Target C2/32EC..33A2: monster special properties and common damage setup.
//! Facing changes, attacker effects and target execution follow separately.
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug)]
pub struct Context {pub special_index:u8,pub special_property:u8,pub special_animation:u8,pub status2:u8,pub status4:u8,pub targets:u8,pub row_flags:u8}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct State {pub magic:[u8;14],pub flags_b2:u8,pub flags_b3:u8,pub graphics_flags:u8,pub animation:u8,pub multiplier:u8,pub base:u16}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Effect {RunicAbsorb,CopyGraphics}
pub fn prepare(c:Context,s:&mut State,effects:&mut Vec<Effect>) {
    if c.special_index==6 {
        s.flags_b2|=2;s.graphics_flags|=1;s.animation=c.special_animation;
        let raw=c.special_property;
        if raw&0x40!=0 {s.magic[6]=0;s.magic[7]|=1;}
        if raw&0x80!=0 {s.magic[4]|=0x20;}
        let mut kind=raw&0x3f;
        if kind==0x30 || kind==0x31 {
            s.magic[4]|=2;if kind==0x31 {s.magic[3]|=0x80;}
        } else {
            if kind>=0x32 {s.magic[4]|=4;kind=0x17;}
            if kind>=0x20 {s.multiplier=s.multiplier.wrapping_add(kind-0x20+1);}
            else {s.magic[10+usize::from(kind/8)]|=1<<(kind%8);}
        }
    }
    let prior=s.flags_b2;s.flags_b2|=0x40;
    if prior&0x40==0 {effects.extend([Effect::RunicAbsorb,Effect::CopyGraphics]);s.graphics_flags&=!0x10;}
    if c.status4&8!=0 {s.multiplier=s.multiplier.wrapping_add(2);}
    if s.magic[2]&1!=0 && c.status2&0x10!=0 {s.multiplier=s.multiplier.wrapping_add(1);}
    if s.magic[2]&0x40==0 && c.targets>=2 {s.base>>=1;}
    if s.flags_b3&0x20==0 && c.row_flags&0x20!=0 {s.base>>=1;}
}
pub fn capture(directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let mut seed=0x6ff6u32;
    let mut rows=String::from("case,special_index,property,special_animation,status2,status4,targets,row,magic,b2,b3,graphics,animation,multiplier,base,magic_out,b2_out,graphics_out,animation_out,multiplier_out,base_out,effects\n");
    for case in 0..16384usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let c=Context {special_index:if case/256%3==0 {0}else{6},special_property:case as u8,special_animation:next(),status2:next(),status4:next(),targets:(case/7%5) as u8,row_flags:next()};
        let mut s=State {magic:std::array::from_fn(|_|next()),flags_b2:next(),flags_b3:next(),graphics_flags:next(),animation:next(),multiplier:next(),base:u16::from_le_bytes([next(),next()])};let before=s;let mut effects=Vec::new();prepare(c,&mut s,&mut effects);
        let hex=|a:&[u8]|a.iter().map(|v|format!("{v:02X}")).collect::<String>();
        writeln!(rows,"{case},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",c.special_index,c.special_property,c.special_animation,c.status2,c.status4,c.targets,c.row_flags,hex(&before.magic),before.flags_b2,before.flags_b3,before.graphics_flags,before.animation,before.multiplier,before.base,hex(&s.magic),s.flags_b2,s.graphics_flags,s.animation,s.multiplier,s.base,if effects.is_empty() {""}else{"runic|copy"}).unwrap();
    }
    fs::write(directory.join("preparation.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn state()->State {State {magic:[0;14],flags_b2:0x40,flags_b3:0,graphics_flags:0,animation:0,multiplier:0,base:101}}
    fn context()->Context {Context {special_index:6,special_property:0x20,special_animation:3,status2:0,status4:0,targets:1,row_flags:0}}
    #[test]fn guard_special_adds_one_half_damage_step() {
        let mut s=state();prepare(context(),&mut s,&mut Vec::new());assert_eq!((s.multiplier,s.animation,s.flags_b2,s.graphics_flags),(1,3,0x42,1));assert_eq!(s.base,101);
    }
    #[test]fn status_only_undodgeable_mp_drain_and_reflect_removal() {
        let mut s=state();s.magic[6]=60;prepare(Context {special_property:0xf1,..context()},&mut s,&mut Vec::new());assert_eq!((s.magic[6],s.magic[7],s.magic[4],s.magic[3]),(0,1,0x22,0x80));
        let mut s=state();prepare(Context {special_property:0x32,..context()},&mut s,&mut Vec::new());assert_eq!((s.magic[4],s.magic[12]),(4,128));
    }
    #[test]fn common_modifiers_wrap_then_split_and_row_halve_in_order() {
        let mut s=state();s.magic[2]=1;s.multiplier=254;
        prepare(Context {special_index:0,status4:8,status2:16,targets:2,row_flags:32,..context()},&mut s,&mut Vec::new());assert_eq!((s.multiplier,s.base),(1,25));
    }
    #[test]fn target_guard_special_reaches_native_damage_and_hp_application() {
        use crate::{battle_data::BattleData,battle_actor_stats,battle_rage::Traits,battle_target_init,characters::CharacterTables,equipment::ItemTable,battle_equipment,damage::{BaseDamage,Modifiers},damage_application as hp};
        let root=Path::new("extracted/twue-3.07");let data=BattleData::load(root).unwrap();let encounter=data.encounter(data.event_formation(1,0).unwrap()).unwrap();let guard=encounter.monsters.iter().find(|m|m.present).unwrap();
        let stats=battle_actor_stats::monster(&guard.properties,guard.hp,0);let mut traits=Traits {special:0,weak_half:0,allowed:[255;4],pending12:0,pending34:0,relic45:0,equip_status23:0,absorb_null:0,metamorph_flags:0};traits.apply(&guard.properties);
        let mut working=battle_target_init::State {restrictions:0,cursor:0,flags_b1:0,last_spell:0,last_item:0,magic:[0;14]};battle_target_init::Initializer::load(root).unwrap().initialize(0,239,&mut working);working.magic[6]=stats.power;
        // Strength 56 and a successful hit are controlled inputs here; the
        // monster's randomized Strength/complete Fight setup is not yet loaded.
        let base=BaseDamage {level:stats.level,power:stats.power,stat:56,physical:true,monster:true,..BaseDamage::default()}.calculate();
        let mut s=State {magic:working.magic,flags_b2:64,flags_b3:0,graphics_flags:0,animation:0,multiplier:0,base};
        prepare(Context {special_property:traits.special,special_animation:data.special_animation(guard.monster_id).unwrap(),..context()},&mut s,&mut Vec::new());
        let roster=CharacterTables::load(root).unwrap().opening_party().unwrap();let terra=roster[0].as_ref().unwrap();let equipment=ItemTable::load(root).unwrap().calculate(terra,terra.equipment(),0).unwrap();let (battle,_)=battle_equipment::project(&equipment,0);
        let modifiers=Modifiers {rng:0,flags:s.magic[2..5].try_into().unwrap(),attacker_monster:true,target_monster:false,defense:battle.defense as u8,multiplier:s.multiplier,..Modifiers::default()};
        let normal=Modifiers {multiplier:0,..modifiers}.calculate(base);let special=modifiers.calculate(s.base);
        assert_eq!((base,s.animation,s.multiplier,normal,special),(16,3,1,10,15));
        let mut target=hp::Target {hp:terra.hp,mp:terra.mp,max_hp:terra.max_hp,max_mp:terra.max_mp,status1:8,pending_status:0,update_flags:0,monster_flags:0,weapon_spell:0};
        hp::apply(&mut target,hp::Damage {taken:special,healed:65535,invincible:false,golem:255,dog:255,affects_mp:false});assert_eq!((target.hp,target.pending_status),(48,0));
    }
}
