//! Target battle actor setup components: monster C2/2CE4..2D48 and immunity
//! normalization C2/2650..26C8. Full actor loading remains a separate caller.
use crate::{battle_data::MonsterProperties,battle_equipment::invert_evade};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct MonsterStats {
    pub stamina:u8,pub power:u8,pub hand_item:u8,pub evade:u8,pub magic_evade:u8,
    pub hit_rate:u8,pub level:u8,pub speed:u8,pub magic:u8,pub retal_flags:u8,pub monster_status:u8,
}
pub fn monster(prop:&MonsterProperties,max_hp:u16,retal_flags:u8)->MonsterStats {
    let r=prop.raw;
    MonsterStats {stamina:((max_hp>>9).min(24)+16) as u8,power:r[1],hand_item:r[26],evade:invert_evade(r[3]),magic_evade:invert_evade(r[4]),hit_rate:r[2],level:r[16],speed:r[0],magic:(u16::from(r[7])+u16::from(r[7]>>1)).min(255) as u8,retal_flags:retal_flags|(r[30]&0x82),monster_status:r[19]}
}
impl MonsterStats {pub fn encode(self)->[u8;11] {[self.stamina,self.power,self.hand_item,self.evade,self.magic_evade,self.hit_rate,self.level,self.speed,self.magic,self.retal_flags,self.monster_status]}}
/// Source calls these immunity bytes, but a set bit means status is allowed.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Immunity {pub allowed:[u8;4],pub protection:u8}
impl Immunity {
    pub fn fix_death_poison(&mut self,null_elements:u8) {
        self.protection&=!4;
        if self.allowed[0]&0x80==0 {self.allowed[0]|=0x80;self.protection|=4;}
        if null_elements&8!=0 {self.allowed[0]&=!4;}
    }
    pub fn update(&mut self,equip_status23:u16,permanent_morph:u8) {
        if equip_status23&0x100!=0 {self.allowed[3]&=!0x80;}
        if permanent_morph&4!=0 {self.allowed[3]&=!8;}
        let mut pair=u16::from(self.allowed[1])|(u16::from(self.allowed[2])<<8);
        pair&=!(equip_status23&0xee78);
        if pair&0x0240!=0x0240 {pair&=!0x0240;}
        self.allowed[1]=pair as u8;self.allowed[2]=(pair>>8) as u8;
        if self.allowed[2]&0x0c!=0x0c {self.allowed[2]&=!0x0c;}
    }
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let properties=fs::read(root.join("build/native_battle/monster_properties.bin")).map_err(|e|e.to_string())?;
    if properties.len()!=384*32 {return Err("Monster property table length mismatch".into());}
    let mut rows=String::from("case,id,max_hp,retal,result\n");
    for id in 0..384 {let p=MonsterProperties {raw:properties[id*32..id*32+32].try_into().unwrap()};
        for (n,max_hp) in [p.hp(),0,511,512,12287,12288,12799,12800,65535].into_iter().enumerate() {
            let retal=(id%256) as u8;let out=monster(&p,max_hp,retal);
            writeln!(rows,"{},{id},{max_hp},{retal},{}",id*9+n,out.encode().iter().map(|v|format!("{v:02X}")).collect::<String>()).unwrap();
        }
    }
    fs::write(directory.join("monsters.csv"),rows).map_err(|e|e.to_string())?;
    let mut seed=0x6ff6u32;let mut rows=String::from("case,allowed,protection,null,equip,morph,fixed,protection_out,updated\n");
    for case in 0..8192 {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let allowed=[next(),next(),next(),next()];let protection=next();let null=next();let equip=u16::from(next())|(u16::from(next())<<8);let morph=next();
        let mut state=Immunity {allowed,protection};state.fix_death_poison(null);let fixed=state;state.update(equip,morph);
        let hex=|a:&[u8]|a.iter().map(|v|format!("{v:02X}")).collect::<String>();
        writeln!(rows,"{case},{},{protection},{null},{equip},{morph},{},{},{}",hex(&allowed),hex(&fixed.allowed),fixed.protection,hex(&state.allowed)).unwrap();
    }
    fs::write(directory.join("immunity.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn monster_stamina_caps_at_forty_despite_source_comment() {
        let p=MonsterProperties {raw:[0;32]};
        assert_eq!([monster(&p,0,0).stamina,monster(&p,512,0).stamina,monster(&p,12288,0).stamina,monster(&p,65535,0).stamina],[16,17,40,40]);
    }
    #[test]fn death_immunity_becomes_protection_and_poison_resistance_blocks_status() {
        let mut s=Immunity {allowed:[0x7f,255,255,255],protection:0x80};s.fix_death_poison(8);
        assert_eq!(s.allowed[0],0xfb);assert_eq!(s.protection,0x84);
    }
    #[test]fn paired_immunities_and_permanent_statuses_are_normalized() {
        let mut s=Immunity {allowed:[255,0xbf,255,255],protection:0};s.update(0x100,4);
        assert_eq!(s.allowed,[255,0xbf,0xfd,0x77]);
        s.allowed[2]=0xfb;s.update(0,0);assert_eq!(s.allowed[2]&0xc,0);
    }
    #[test]fn opening_guard_properties_feed_native_hit_and_damage() {
        use crate::{battle_data::BattleData,damage::{AttackTable,Modifiers},battle_hit::{self,Context,State,Outcome},battle_hit_gates::Context as Gates,battle_rng::{BattleRng,RngTable},elemental_damage as elements,damage_application as hp};
        let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("extracted/twue-3.07");
        let data=BattleData::load(&root).unwrap();let encounter=data.encounter(data.event_formation(1,0).unwrap()).unwrap();
        let guards:Vec<_>=encounter.monsters.iter().filter(|m|m.present).collect();assert_eq!(guards.len(),2);
        let guard=guards[0];let p=&guard.properties;let stats=monster(p,p.hp(),0);
        let ai=crate::battle_ai::AiTable::load(&root).unwrap();
        for guard in &guards {let pointers=ai.initialize(guard.monster_id,65535).unwrap();assert_eq!((pointers.normal,pointers.retaliation,pointers.normal_end),(0,65535,5));}

        assert_eq!((stats.stamina,stats.speed,stats.level,stats.power,stats.magic,stats.evade,stats.magic_evade),(16,30,5,16,9,255,255));
        let attack=AttackTable::load(&root).unwrap();let r=attack.record(131);
        let mut traits=crate::battle_rage::Traits {special:0,weak_half:0,allowed:[255;4],pending12:0,pending34:0,relic45:0,equip_status23:0,absorb_null:0,metamorph_flags:0};traits.apply(p);
        assert_eq!((traits.special,traits.weak_half,traits.pending12,traits.pending34,traits.metamorph_flags),(0x20,8,0,0,0x1000));
        let mut immunity=Immunity {allowed:traits.allowed,protection:0};immunity.fix_death_poison((traits.absorb_null>>8) as u8);immunity.update(traits.equip_status23,0);
        let context=Context {gates:Gates {flags0:r[2],flags1:r[3],flags2:r[4],command:29,actor_offset:(guard.slot+4)*2,protection:immunity.protection,hit_rate:255,level:stats.level,stamina:stats.stamina,magic_evade:stats.magic_evade,..Gates::default()},evade:stats.evade,target_mask:1<<(guard.slot+8),..Context::default()};
        let table=RngTable::identity();let mut rng=BattleRng::default();let mut state=State::default();
        assert_eq!(battle_hit::check(context,&mut state,&mut rng,&table).unwrap(),Outcome::Hit);assert_eq!(rng.index,0);
        let base=attack.base(131,3,39,false,29).calculate();
        let amount=Modifiers {rng:rng.next(&table),flags:r[2..5].try_into().unwrap(),element:r[1],defense:p.raw[5],magic_defense:p.raw[6],..Modifiers::default()}.calculate(base);
        let c=elements::resolve(amount,elements::Context {power:r[6],element:r[1],flags0:r[2],flags2:r[4],absorb:traits.absorb_null as u8,null:(traits.absorb_null>>8) as u8,weak:traits.weak_half as u8,..elements::Context::default()}).unwrap().unwrap();
        let mut target=hp::Target {hp:p.hp(),mp:p.mp(),max_hp:p.hp(),max_mp:p.mp(),status1:0,pending_status:0,update_flags:0,monster_flags:0,weapon_spell:0};
        hp::apply(&mut target,hp::Damage {taken:elements::accumulate(65535,c.amount),healed:65535,invincible:false,golem:255,dog:255,affects_mp:false});
        assert_eq!((base,c.amount,target.hp,target.pending_status,rng.index),(459,182,0,128,1));
    }
}
