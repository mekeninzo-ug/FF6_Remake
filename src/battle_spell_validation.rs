//! ValidateSpellList C2/568D..574A; typed native spell menu preparation.
use crate::{damage::AttackTable,battle_command_init,battle_action_init::CommandProperties};
use std::{fs,path::Path,collections::BTreeMap};
#[derive(Clone,Copy,Debug)]pub struct Context {pub slot:usize,pub actor:u8,pub esper:u8,pub relic3:u8,pub learning:[u8;54]}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct State {pub records:[[u8;4];79],pub known:u8,pub cached_esper:u8,pub cached_target:u8}
pub fn modified_cost(mut cost:u8,relic3:u8)->u8 {if relic3&32!=0 {cost=cost.wrapping_add(1)>>1;}if relic3&64!=0 {cost=1;}cost}
pub fn validate(c:Context,s:&mut State,attacks:&AttackTable)->Result<(),String> {
    if c.slot>=4 {return Err("Spell validation requires a character battle slot".into());}
    if s.records[1..55].iter().any(|r|r[0]!=255 && r[0]>=54) {return Err("Magic menu ID outside 54-entry learning table".into());}
    s.known=0;
    if c.esper<128 {let r=attacks.record(c.esper.wrapping_add(54));s.cached_esper=c.esper;s.cached_target=r[0];s.records[0][0]=c.esper;s.records[0][2]=r[0];s.records[0][3]=modified_cost(r[5],c.relic3);}
    for index in (1..79).rev() {
        let record=&mut s.records[index];let magic=index<55;
        let allowed=record[0]!=255 && (c.actor==12 || (c.actor<12 && (!magic || c.learning[usize::from(record[0])]==255)));
        if allowed {if magic {s.known=s.known.wrapping_add(1);}record[3]=modified_cost(record[3],c.relic3);}
        else {record[0]=255;record[3]=0;}
    }
    Ok(())
}
/// Connect validation outputs to InitCmdList instead of supplying guessed eligibility.
pub fn validate_commands(c:Context,s:&mut State,mut command: battle_command_init::Context,menu:&mut battle_command_init::State,attacks:&AttackTable,properties:&CommandProperties)->Result<(),String> {
    if c.slot!=command.slot || c.actor!=command.actor {return Err("Spell and command contexts identify different actors".into());}
    validate(c,s,attacks)?;command.magic_count=s.known;command.esper=c.esper;battle_command_init::initialize(command,menu,properties)
}
fn snapshot(c:Context,s:&State)->String {
    let mut m=BTreeMap::<u32,u8>::new();let x=c.slot as u32*2;let field=c.slot as u16*37;let learning_actor=c.actor.min(11);
    let mut put=|addr:u32,value:u16,width:usize|{for i in 0..width {m.insert(addr+i as u32,(value>>(i*8)) as u8);}};
    for (addr,value,width) in [(0x3010+x,field,2),(0x302c+x,0x5000,2),(0x161e+u32::from(field),u16::from(c.esper),1),(0x3ed8+x,u16::from(c.actor),1),(0x3c45+x,u16::from(c.relic3),1),(0x3344+x,u16::from(s.cached_esper),1),(0x3345+x,u16::from(s.cached_target),1),(0x3cf8+x,u16::from(s.known),1)] {put(0x7e0000+addr,value,width);}
    for (i,v) in s.records.concat().iter().enumerate() {put(0x7e5000+i as u32,u16::from(*v),1);}
    for (i,v) in c.learning.iter().enumerate() {put(0x7e1a6e+u32::from(learning_actor)*54+i as u32,u16::from(*v),1);}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let attacks=AttackTable::load(root)?;let mut seed=0x6ff6u32;
    let mut rows=String::from("case,slot,before,after\n");
    for case in 0..2048usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let c=Context {slot:case%4,actor:if case%17==16 {255}else{(case%17) as u8},esper:case as u8,relic3:(case/8) as u8,learning:std::array::from_fn(|i|if (case+i)%3==0 {255}else{next()%255})};
        let mut s=State {records:std::array::from_fn(|i|[if (case+i)%7==0 {255}else{next()%if i<55 {54}else{24}},next(),next(),next()]),known:next(),cached_esper:next(),cached_target:next()};
        let before=snapshot(c,&s);validate(c,&mut s,&attacks)?;writeln!(rows,"{case},{},{before},{}",c.slot,snapshot(c,&s)).unwrap();
    }fs::write(dir.join("validation.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("cost,relic,result\n");for cost in 0..=255u8 {for relic in 0..=255u8 {writeln!(rows,"{cost},{relic},{}",modified_cost(cost,relic)).unwrap();}}
    fs::write(dir.join("costs.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn attacks()->AttackTable {AttackTable::load(Path::new("extracted/twue-3.07")).unwrap()}
    fn context()->Context {Context {slot:0,actor:0,esper:255,relic3:0,learning:[0;54]}}
    fn state()->State {State {records:std::array::from_fn(|i|[if i==0 {255}else{((i-1)%54) as u8},85,67,9]),known:99,cached_esper:33,cached_target:44}}
    #[test]fn normal_magic_requires_full_learning_but_lore_does_not() {let mut c=context();c.learning[0]=255;c.learning[1]=254;let mut s=state();validate(c,&mut s,&attacks()).unwrap();assert_eq!(s.known,1);assert_eq!(s.records[1],[0,85,67,9]);assert_eq!(s.records[2],[255,85,67,0]);assert_eq!(s.records[55][3],9);assert_eq!((s.cached_esper,s.cached_target),(33,44));}
    #[test]fn gogo_retains_all_present_magic_and_guests_have_none() {let mut s=state();validate(Context {actor:12,..context()},&mut s,&attacks()).unwrap();assert_eq!(s.known,54);assert_eq!(s.records[78][3],9);let mut s=state();validate(Context {actor:13,..context()},&mut s,&attacks()).unwrap();assert_eq!(s.known,0);assert!(s.records[1..].iter().all(|r|r[0]==255 && r[3]==0));}
    #[test]fn esper_uses_target_attack_data_and_preserves_disabled_byte() {let mut s=state();let a=attacks();validate(Context {esper:0,relic3:32,..context()},&mut s,&a).unwrap();let prop=a.record(54);assert_eq!(s.records[0],[0,85,prop[0],modified_cost(prop[5],32)]);assert_eq!((s.cached_esper,s.cached_target),(0,prop[0]));}
    #[test]fn invalid_esper_does_not_clear_existing_record_or_caches() {let mut s=state();s.records[0]=[9,10,11,12];validate(Context {esper:128,..context()},&mut s,&attacks()).unwrap();assert_eq!(s.records[0],[9,10,11,12]);assert_eq!((s.cached_esper,s.cached_target),(33,44));}
    #[test]fn relic_costs_keep_byte_wrap_and_economizer_precedence() {assert_eq!((modified_cost(255,32),modified_cost(5,32),modified_cost(0,64),modified_cost(255,96)),(0,3,1,1));}
    #[test]fn removed_spells_keep_flags_and_targeting_but_clear_cost() {let mut s=state();s.records[1]=[255,23,45,67];validate(context(),&mut s,&attacks()).unwrap();assert_eq!(s.records[1],[255,23,45,0]);}
    #[test]fn validation_drives_magic_command_and_battle_mp_eligibility() {let mut c=context();c.learning[0]=255;let mut s=state();let mut menu=battle_command_init::State {records:[[0;3];4],mp:27,max_mp:27,pending_status4:0,morph_counter:0};let props=CommandProperties::load(Path::new("extracted/twue-3.07")).unwrap();let cmd=battle_command_init::Context {slot:0,actor:0,commands:[0,3,2,1],status1:8,global11:0,global12:0,relic_commands:0,magic_count:0,esper:0,dances:0,battle_flags:0};validate_commands(c,&mut s,cmd,&mut menu,&attacks(),&props).unwrap();assert_eq!((s.known,menu.records[2][0],menu.mp),(1,2,27));let mut s=state();validate_commands(context(),&mut s,cmd,&mut menu,&attacks(),&props).unwrap();assert_eq!((s.known,menu.records[2][0],menu.mp,menu.max_mp),(0,255,0,0));}
}
