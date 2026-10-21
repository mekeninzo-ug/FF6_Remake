//! Complete InitCmdList C2/532C..5449; command menu construction, not execution.
use crate::battle_action_init::CommandProperties;
use std::{fs,path::Path,collections::BTreeMap};
#[derive(Clone,Copy,Debug)]pub struct Context {
    pub slot:usize,pub actor:u8,pub commands:[u8;4],pub status1:u8,pub global11:u8,pub global12:u8,
    pub relic_commands:u8,pub magic_count:u8,pub esper:u8,pub dances:u8,pub battle_flags:u8,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct State {
    pub records:[[u8;3];4],pub mp:u16,pub max_mp:u16,pub pending_status4:u8,pub morph_counter:u8,
}
pub fn initialize(c:Context,s:&mut State,properties:&CommandProperties)->Result<(),String> {
    if c.slot>=4 {return Err("Command initialization requires a character battle slot".into());}
    if c.commands.iter().any(|id|*id>=32 && *id!=255) {return Err("Command ID outside extracted 32-command table".into());}
    let mut commands=c.commands;
    if c.status1&8!=0 || c.global11&1!=0 {
        for id in commands.iter_mut().rev() {
            if *id==1 || *id==18 {continue;}
            if c.actor==11 && *id==16 {*id=0;}
            *id=match *id {0=>29,2=>2,_=>255};
            if c.global11&1!=0 {*id=match *id {29=>2,_=>255};}
        }
    }
    let mut has_mp=false;
    for (index,mut id) in commands.into_iter().enumerate() {
        for (old,new,mask) in [(0,22,4),(2,23,8),(13,14,16),(15,24,32),(5,6,64)] {if id==old && c.relic_commands&mask!=0 {id=new;}}
        match id {
            3=>{if c.global12&4==0 {id=255;}else if c.global11&4!=0 {s.pending_status4|=8;s.morph_counter=255;id=255;}},
            17=>{if c.battle_flags&2==0 {id=255;}},
            19=>{if c.dances==0 {id=255;}},
            2|23=>{if c.magic_count!=0 || c.esper!=255 {has_mp=true;}else{id=255;}},
            12=>has_mp=true,
            _=>{},
        }
        s.records[index]=[id,id,if id==255 {0}else{properties.bytes[usize::from(id)*2+1]}];
    }
    if !has_mp {s.mp=0;s.max_mp=0;}
    Ok(())
}
fn snapshot(c:Context,s:&State)->String {
    let mut m=BTreeMap::<u32,u8>::new();let x=c.slot as u32*2;let offset=c.slot as u16*37;
    let mut put=|addr:u32,val:u16,width:usize|{for i in 0..width {m.insert(addr+i as u32,(val>>(i*8)) as u8);}};
    for (addr,val,width) in [(0x3010+x,offset,2),(0x3ed8+x,u16::from(c.actor),1),(0x1614+u32::from(offset),u16::from(c.status1),1),(0x3ebb,u16::from(c.global11),1),(0x3ebc,u16::from(c.global12),1),(0x11d6,u16::from(c.relic_commands),1),(0x1d4c,u16::from(c.dances),1),(0x11e4,u16::from(c.battle_flags),1),(0x3c08+x,s.mp,2),(0x3c30+x,s.max_mp,2),(0x3de9+x,u16::from(s.pending_status4),1),(0x1cf6,u16::from(s.morph_counter),1),(0xf6,u16::from(c.magic_count),1),(0xf7,u16::from(c.esper),1)] {put(0x7e0000+addr,val,width);}
    for (i,v) in c.commands.iter().enumerate() {put(0x7e1616+u32::from(offset)+i as u32,u16::from(*v),1);}
    for (i,v) in s.records.concat().iter().enumerate() {put(0x7e202e+c.slot as u32*12+i as u32,u16::from(*v),1);}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let props=CommandProperties::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,slot,before,after\n");
    for case in 0..4096usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let c=Context {slot:case%4,actor:if case%5==0 {11}else{next()%18},commands:std::array::from_fn(|i|if (case+i)%33==32 {255}else{((case+i*7)%32) as u8}),status1:case as u8,global11:(case/4) as u8,global12:(case/8) as u8,relic_commands:case as u8,magic_count:if case%3==0 {0}else{next()},esper:if case%5==0 {next()}else{255},dances:if case%7==0 {0}else{next()},battle_flags:(case/16) as u8};
        let mut s=State {records:std::array::from_fn(|_|std::array::from_fn(|_|next())),mp:u16::from_le_bytes([next(),next()]),max_mp:u16::from_le_bytes([next(),next()]),pending_status4:next(),morph_counter:next()};let before=snapshot(c,&s);initialize(c,&mut s,&props)?;writeln!(rows,"{case},{},{before},{}",c.slot,snapshot(c,&s)).unwrap();
    }fs::write(dir.join("command-init.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn context()->Context {Context {slot:0,actor:0,commands:[0,2,3,1],status1:0,global11:0,global12:0,relic_commands:0,magic_count:1,esper:255,dances:0,battle_flags:0}}
    fn state()->State {State {records:[[99;3];4],mp:27,max_mp:27,pending_status4:1,morph_counter:17}}
    fn props()->CommandProperties {CommandProperties {bytes:std::array::from_fn(|i|i as u8)}}
    #[test]fn opening_magitek_preserves_magic_and_item() {let mut s=state();initialize(Context {status1:8,..context()},&mut s,&props()).unwrap();assert_eq!(s.records,[[29,29,59],[2,2,5],[255,255,0],[1,1,3]]);assert_eq!((s.mp,s.max_mp),(27,27));}
    #[test]fn magic_only_converts_fight_but_removes_existing_magic() {let mut s=state();initialize(Context {global11:1,..context()},&mut s,&props()).unwrap();assert_eq!(s.records.map(|r|r[0]),[2,255,255,1]);assert_eq!(s.mp,27);}
    #[test]fn gau_rage_becomes_magitek_but_mimic_and_item_survive() {let mut s=state();initialize(Context {actor:11,status1:8,commands:[16,18,1,5],..context()},&mut s,&props()).unwrap();assert_eq!(s.records.map(|r|r[0]),[29,18,1,255]);assert_eq!((s.mp,s.max_mp),(0,0));}
    #[test]fn relics_change_commands_before_initializers_run() {let mut s=state();initialize(Context {commands:[0,2,13,5],relic_commands:92,magic_count:0,..context()},&mut s,&props()).unwrap();assert_eq!(s.records.map(|r|r[0]),[22,255,14,6]);assert_eq!(s.mp,0);}
    #[test]fn permanent_morph_sets_pending_status_and_full_resource() {let mut s=state();initialize(Context {global11:4,global12:4,commands:[3,17,19,12],..context()},&mut s,&props()).unwrap();assert_eq!(s.records.map(|r|r[0]),[255,255,255,12]);assert_eq!((s.pending_status4,s.morph_counter,s.mp),(9,255,27));}
    #[test]fn esper_alone_keeps_magic_and_mp() {let mut s=state();initialize(Context {magic_count:0,esper:0,..context()},&mut s,&props()).unwrap();assert_eq!((s.records[1][0],s.mp),(2,27));}
    #[test]fn target_terra_commands_initialize_refresh_and_queue_fire_beam() {
        use crate::{characters::CharacterTables,battle_commands::{CommandLists,PlayerInputs,MpCosts},battle_scheduler::Scheduler,battle_menu_state::{self,CommandContext},battle_target_init};
        let root=Path::new("extracted/twue-3.07");let roster=CharacterTables::load(root).unwrap().opening_party().unwrap();let terra=roster[0].as_ref().unwrap();let properties=CommandProperties::load(root).unwrap();
        let mut s=State {mp:terra.mp,max_mp:terra.max_mp,..state()};
        // Known-magic eligibility is supplied here; ValidateSpellList remains a separate stage.
        initialize(Context {commands:terra.commands(),status1:8,..context()},&mut s,&properties).unwrap();
        assert_eq!(s.records.map(|r|r[0]),[29,255,2,1]);
        let mut changed=0;battle_menu_state::update_commands(CommandContext {slot:0,status1:8,status2:0,right_flags:0,left_flags:0,relic4:0,morph_counter:0},&properties,&mut s.records,&mut changed).unwrap();assert_eq!(s.records[0][1]&128,0);
        let mut input=PlayerInputs::default();input.bytes[..8].copy_from_slice(&[0,s.records[0][0],0,0,4,255,255,255]);let mut scheduler=Scheduler::default();let mut commands=CommandLists::default();let attacks=crate::damage::AttackTable::load(root).unwrap();let ids=input.apply(&mut scheduler,&mut commands,&MpCosts::from_attacks(&attacks),&mut 0,[0;4]).unwrap();let payload=commands.payloads[usize::from(ids[0])];assert_eq!((payload.command,payload.attack,payload.targets,payload.mp_cost),(29,131,1024,0));
        let mut working=battle_target_init::State {restrictions:0,cursor:0,flags_b1:0,last_spell:255,last_item:255,magic:[0;14]};battle_target_init::Initializer::load(root).unwrap().initialize(payload.command,payload.attack,&mut working);assert_eq!((working.last_spell,working.magic[6]),(131,60));assert_eq!(s.mp,27);
    }

}
