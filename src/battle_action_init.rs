//! Target C2/0276 queued-action initialization, including patched C2/67E1.
use crate::{battle_commands::{CommandLists,Payload},battle_rng::{BattleRng,RngTable}};
use std::{fs,path::Path};
#[derive(Clone)]pub struct CommandProperties {pub bytes:[u8;64]}
impl CommandProperties {
    pub fn load(root:&Path)->Result<Self,String> {
        let bytes=fs::read(root.join("build/native_battle/command_properties.bin")).map_err(|e|e.to_string())?;
        Ok(Self {bytes:bytes.try_into().map_err(|_|"Expected 32 two-byte command properties")?})
    }
}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct Context {pub changed_targets:u8,pub summon_used:u8,pub opponents:u16}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct InitializedAction {pub returned_command:u8,pub command:u8,pub attack:u8,pub targets:u16,pub mp_cost:u8}

pub fn initialize(payload:Payload,slot:usize,retarget:bool,status1:u8,context:&mut Context,rng:&mut BattleRng,table:&RngTable,properties:&CommandProperties)->Result<InitializedAction,String> {
    if slot>=10 {return Err("Queued action actor outside ten battle slots".into());}
    let mut action=InitializedAction {returned_command:payload.command,command:payload.command,attack:payload.attack,targets:payload.targets,mp_cost:payload.mp_cost};
    let mask=if slot<4 {1u8<<slot}else{0};
    if retarget && payload.command<29 {
        let changed=context.changed_targets&mask!=0;context.changed_targets&=!mask;
        if changed {action.targets=0;}
    }
    if status1&0x20==0 {return Ok(action);}
    // TWUE's Imp's Call helper releases a pending summon before fallback.
    if payload.command==25 {context.summon_used&=!mask;}
    if payload.command>=30 || properties.bytes[usize::from(payload.command)*2]&4!=0 {return Ok(action);}
    action.command=0;action.attack=0;action.mp_cost=0;
    let filter=if slot<4 {context.opponents|0xff00}else{context.opponents|0x00ff};
    action.targets=rng.bit(table,action.targets&filter);
    Ok(action)
}
pub fn initialize_normal(lists:&CommandLists,slot:usize,retarget:bool,status1:u8,context:&mut Context,rng:&mut BattleRng,table:&RngTable,properties:&CommandProperties)->Result<Option<InitializedAction>,String> {
    let head=*lists.normal.get(slot).ok_or("Queued action actor outside pool heads")?;
    if head&0x80!=0 {return Ok(None);}
    initialize(lists.payloads[usize::from(head)],slot,retarget,status1,context,rng,table,properties).map(Some)
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let props=CommandProperties::load(root)?;let table=RngTable::load(root)?;
    let mut rows=String::from("case,command,attack,targets,cost,slot,retarget,status1,changed,summon,opponents,seed,returned,working_command,working_attack,working_targets,working_cost,after_changed,after_summon,index\n");
    for case in 0..8192usize {
        let command=(case%256) as u8;let attack=(case*17) as u8;let targets=[0,1,0x100,15,0x3f00,0x3fff,0xffff][case/3%7];let cost=(case*11) as u8;
        let slot=(case/256)%10;let retarget=(case/7)%2==0;let status1=if (case/13)%2==0 {0x20}else{0};
        let changed=if case%3==0 {15}else{case as u8};let summon=15;let opponents=[0,0x3f00,0x00ff,0xffff,0x1555][case/5%5];let seed=(case*19) as u8;
        let mut context=Context {changed_targets:changed,summon_used:summon,opponents};let mut rng=BattleRng {index:seed};
        let action=initialize(Payload {command,attack,targets,mp_cost:cost},slot,retarget,status1,&mut context,&mut rng,&table,&props)?;
        writeln!(rows,"{case},{command},{attack},{targets},{cost},{slot},{},{status1},{changed},{summon},{opponents},{seed},{},{},{},{},{},{},{},{}",u8::from(retarget),action.returned_command,action.command,action.attack,action.targets,action.mp_cost,context.changed_targets,context.summon_used,rng.index).unwrap();
    }
    fs::write(directory.join("initialization.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn properties()->CommandProperties {let mut bytes=[0;64];bytes[0]=4;bytes[4]=4;CommandProperties {bytes}}
    #[test]fn imp_summon_fix_releases_used_bit_and_preserves_original_return_value() {
        let mut c=Context {changed_targets:0,summon_used:15,opponents:0x3f00};let mut rng=BattleRng::default();
        let p=Payload {command:25,attack:54,targets:0x101,mp_cost:30};
        let a=initialize(p,1,false,0x20,&mut c,&mut rng,&RngTable::identity(),&properties()).unwrap();
        assert_eq!(a,InitializedAction {returned_command:25,command:0,attack:0,targets:0x100,mp_cost:0});assert_eq!(c.summon_used,13);assert_eq!(rng.index,1);
    }
    #[test]fn changed_targets_gate_does_not_apply_to_magitek_or_immediate_actions() {
        let mut c=Context {changed_targets:15,..Context::default()};let mut rng=BattleRng::default();
        let p=Payload {command:29,attack:131,targets:0x100,mp_cost:0};
        assert_eq!(initialize(p,0,true,0,&mut c,&mut rng,&RngTable::identity(),&properties()).unwrap().targets,0x100);assert_eq!(c.changed_targets,15);
        let p=Payload {command:2,..p};
        assert_eq!(initialize(p,0,false,0,&mut c,&mut rng,&RngTable::identity(),&properties()).unwrap().targets,0x100);
        assert_eq!(initialize(p,0,true,0,&mut c,&mut rng,&RngTable::identity(),&properties()).unwrap().targets,0);assert_eq!(c.changed_targets,14);
    }
}
