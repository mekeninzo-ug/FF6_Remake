//! LoadCharProp C2/27A8..283B and CalcMaxHPMP C2/283C..2860.
use std::{fs,path::Path,collections::BTreeMap};
#[derive(Clone,Copy,Debug)]pub struct Context {pub slot:usize,pub field_index:u8,pub refresh:u16,pub equip_status23:u16}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct State {pub hp:u16,pub mp:u16,pub max_hp:u16,pub max_mp:u16,pub pending:[u8;4],pub update:u8,pub level:u8,pub mimic_command:u8,pub mimic_attack:u8}
pub fn boosted(raw:u16)->u16 {let base=raw&0x3fff;base+match raw>>14 {0=>0,1=>base>>2,2=>base>>1,3=>base>>3,_=>unreachable!()}}
pub fn load(c:Context,field:&mut[u8;37],s:&mut State)->Result<(),String> {
    if c.slot>=4 || c.field_index>=16 {return Err("Character load requires four battle/sixteen field slots".into());}
    let word=|i|u16::from_le_bytes([field[i],field[i+1]]);s.hp=word(9);s.mp=word(13);s.max_hp=boosted(word(11)).min(9999);s.max_mp=boosted(word(15)).min(999);
    if c.refresh&(1<<c.slot)!=0 {s.hp=s.max_hp;s.mp=s.max_mp;field[20]&=0x2d;}
    s.pending[1]=c.equip_status23 as u8;if s.pending[1]&1!=0 {s.update&=!16;}
    s.pending[0]=field[20];if field[20]&8!=0 {s.mimic_command=29;s.mimic_attack=131;}
    let high=(c.equip_status23>>8) as u8;s.pending[2]=high&254;s.pending[3]=(field[21]&192)|if high&1!=0 {128}else{0};s.level=field[8];Ok(())
}
fn snapshot(c:Context,field:&[u8;37],s:&State)->String {
    let mut m=BTreeMap::<u32,u8>::new();let x=c.slot as u32*2;let offset=u16::from(c.field_index)*37;let mut put=|addr:u32,value:u16,width:usize|{for i in 0..width {m.insert(addr+i as u32,(value>>(i*8)) as u8);}};
    for (i,v) in field.iter().enumerate() {put(0x7e1600+u32::from(offset)+i as u32,u16::from(*v),1);}
    for (addr,value,width) in [(0x3010+x,offset,2),(0x3018+x,1<<c.slot,2),(0xb8,c.refresh,2),(0x3c6c+x,c.equip_status23,2),(0x3bf4+x,s.hp,2),(0x3c08+x,s.mp,2),(0x3c1c+x,s.max_hp,2),(0x3c30+x,s.max_mp,2),(0x3dd4+x,u16::from(s.pending[0]),1),(0x3dd5+x,u16::from(s.pending[1]),1),(0x3de8+x,u16::from(s.pending[2]),1),(0x3de9+x,u16::from(s.pending[3]),1),(0x3204+x,u16::from(s.update),1),(0x3b18+x,u16::from(s.level),1),(0x3f20,u16::from(s.mimic_command),1),(0x3f21,u16::from(s.mimic_attack),1)] {put(0x7e0000+addr,value,width);}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let mut seed=0x6ff6u32;let mut rows=String::from("case,slot,before,after\n");
    for case in 0..8192usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let c=Context {slot:case%4,field_index:(case/4%16) as u8,refresh:if case%3==0 {0}else{65535},equip_status23:((case%256) as u16)|(((case/32%256) as u16)<<8)};let mut field=std::array::from_fn(|_|next());let mut s=State {hp:0,mp:0,max_hp:0,max_mp:0,pending:std::array::from_fn(|_|next()),update:next(),level:next(),mimic_command:next(),mimic_attack:next()};let before=snapshot(c,&field,&s);load(c,&mut field,&mut s)?;writeln!(rows,"{case},{},{before},{}",c.slot,snapshot(c,&field,&s)).unwrap();}
    fs::write(dir.join("characters.csv"),rows).map_err(|e|e.to_string())?;let mut rows=String::from("raw,result\n");for raw in 0..=65535 {writeln!(rows,"{raw},{}",boosted(raw)).unwrap();}fs::write(dir.join("boosts.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn context()->Context {Context {slot:0,field_index:0,refresh:0,equip_status23:0}}
    fn field()->[u8;37] {let mut r=[0;37];r[8]=3;for (i,v) in [(9,63u16),(11,63),(13,27),(15,27)] {r[i..i+2].copy_from_slice(&v.to_le_bytes());}r}
    fn state()->State {State {hp:0,mp:0,max_hp:0,max_mp:0,pending:[255;4],update:255,level:0,mimic_command:99,mimic_attack:100}}
    #[test]fn packed_boosts_use_source_flag_order() {assert_eq!([boosted(80),boosted(0x4050),boosted(0x8050),boosted(0xc050)],[80,100,120,90]);}
    #[test]fn limits_apply_only_to_maximums_without_refresh() {let mut r=field();r[9..17].fill(255);let mut s=state();load(context(),&mut r,&mut s).unwrap();assert_eq!((s.hp,s.mp,s.max_hp,s.max_mp),(65535,65535,9999,999));}
    #[test]fn refresh_restores_resources_and_clears_only_source_status_bits() {let mut r=field();r[20]=255;r[21]=127;let mut s=state();load(Context {refresh:1,..context()},&mut r,&mut s).unwrap();assert_eq!((s.hp,s.mp,r[20],r[21]),(63,27,45,127));}
    #[test]fn equipment_status_is_pending_and_condemned_stops_counter_update() {let mut r=field();r[20]=4;r[21]=64;let mut s=state();load(Context {equip_status23:0xff01,..context()},&mut r,&mut s).unwrap();assert_eq!(s.pending,[4,1,254,192]);assert_eq!(s.update,239);}
    #[test]fn magitek_sets_shared_mimic_attack_without_committing_status() {let mut r=field();r[20]=8;let mut s=state();load(context(),&mut r,&mut s).unwrap();assert_eq!((s.pending[0],s.mimic_command,s.mimic_attack),(8,29,131));}
    #[test]fn non_magitek_preserves_previous_mimic_request_and_level_is_raw() {let mut r=field();r[8]=255;let mut s=state();load(context(),&mut r,&mut s).unwrap();assert_eq!((s.mimic_command,s.mimic_attack,s.level),(99,100,255));}
    #[test]fn refresh_mask_uses_battle_slot_not_field_record_number() {let mut r=field();r[9]=1;let mut s=state();load(Context {slot:2,field_index:15,refresh:4,..context()},&mut r,&mut s).unwrap();assert_eq!(s.hp,63);}
    #[test]fn actual_opening_roster_resolves_party_slots_then_loads_resources() {
        use crate::{battle_party as party,characters::CharacterTables};let root=Path::new("extracted/twue-3.07");let roster=CharacterTables::load(root).unwrap().opening_party().unwrap();let mut c=party::Context {fields:std::array::from_fn(|i|{let mut f=[0;37];f[0]=i as u8;f}),membership:[0;16],active_party:1,party_mode:255,battle:2,gau_switch:0,monster6:255};
        for (slot,id) in [14usize,15,0].into_iter().enumerate() {let actor=roster[id].as_ref().unwrap();let f=&mut c.fields[id];f[0]=actor.property_id;f[8]=actor.level;for (offset,value) in [(9,actor.hp),(11,actor.max_hp),(13,actor.mp),(15,actor.max_mp)] {f[offset..offset+2].copy_from_slice(&value.to_le_bytes());}f[20]=8;c.membership[id]=1|((slot as u8)<<3);}
        let mut p=party::State::default();party::initialize(&c,&mut p,&party::Tables::load(root).unwrap()).unwrap();assert_eq!(p.slots.map(|a|a.actor),[32,33,0,255]);let mut loaded=[state();3];for i in 0..3 {let field_index=p.slots[i].field_index;load(Context {slot:i,field_index,refresh:p.valid,equip_status23:0},&mut c.fields[usize::from(field_index)],&mut loaded[i]).unwrap();}assert_eq!((loaded[0].hp,loaded[1].hp,loaded[2].hp,loaded[2].mp),(68,70,63,27));assert_eq!(loaded[2].pending[0],8);
    }
}
