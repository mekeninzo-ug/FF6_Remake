//! Target UpdateCmdList C2/527D..532B and UpdateEnabledMagic C2/5763..57C0.
//! Updates menu records only; it does not render or execute their commands.
use crate::battle_action_init::CommandProperties;
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug)]
pub struct CommandContext {pub slot:usize,pub status1:u8,pub status2:u8,pub right_flags:u8,pub left_flags:u8,pub relic4:u8,pub morph_counter:u8}
pub fn update_commands(context:CommandContext,properties:&CommandProperties,records:&mut[[u8;3];4],changed:&mut u16)->Result<(),String> {
    let c=context;if c.slot>=4 {return Err("Command refresh requires a character battle slot".into());}
    *changed|=1<<c.slot;
    for record in records {
        let id=record[0];
        if id<128 && id>=32 {return Err(format!("Command refresh ID {id:02X} is outside extracted command properties"));}
        let mut disabled=id&128!=0;
        if !disabled && c.status1&32!=0 && properties.bytes[usize::from(id)*2]&4==0 {disabled=true;}
        if !disabled {
            match id {
                0|6=>{if c.relic4&1!=0 {record[2]=0x4e;}},
                2|12|23=>disabled=c.status2&8!=0,
                7=>disabled=(c.right_flags|c.left_flags)&2==0,
                11=>disabled=(c.right_flags|c.left_flags)&128==0,
                3=>disabled=c.morph_counter<=15,
                _=>{},
            }
        }
        record[1]=(record[1]>>1)|if disabled {128}else{0};
    }
    Ok(())
}
/// Spell records retain the source menu's IDs, not necessarily global attack IDs.
pub fn update_spells(slot:usize,mp:u16,status1:u8,used_espers:u8,records:&mut[[u8;4];79],max_cost:&mut u8) {
    if slot>=4 {return;}
    *max_cost=if mp>=255 {255}else{mp as u8+1};
    for (index,record) in records.iter_mut().enumerate() {
        let disabled=(index==0 && used_espers&(1<<slot)!=0) || record[0]&128!=0 || (status1&32!=0 && record[0]!=35) || record[3]>=*max_cost;
        record[1]=(record[1]>>1)|if disabled {128}else{0};
    }
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let properties=CommandProperties::load(root)?;let mut seed=0x6ff6u32;
    let hex=|bytes:&[u8]|bytes.iter().map(|v|format!("{v:02X}")).collect::<String>();
    let mut rows=String::from("case,slot,status1,status2,right,left,relic,morph,changed,records,changed_out,records_out\n");
    for case in 0..8192usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let c=CommandContext {slot:case%4,status1:next(),status2:next(),right_flags:next(),left_flags:next(),relic4:next(),morph_counter:next()};
        let mut records=std::array::from_fn(|_|[if next()%7==0 {255}else{next()%32},next(),next()]);let mut changed=u16::from_le_bytes([next(),next()]);let before=records;let before_changed=changed;
        update_commands(c,&properties,&mut records,&mut changed)?;
        writeln!(rows,"{case},{},{},{},{},{},{},{},{before_changed},{},{changed},{}",c.slot,c.status1,c.status2,c.right_flags,c.left_flags,c.relic4,c.morph_counter,hex(&before.concat()),hex(&records.concat())).unwrap();
    }
    fs::write(directory.join("commands.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,slot,mp,status1,used,max_cost,records,max_cost_out,records_out\n");
    for case in 0..2048usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let slot=case%10;let mp=[0,1,254,255,256,9999,65535][case/3%7];let status1=next();let used=next();let mut max_cost=next();let before_cost=max_cost;
        let mut records=std::array::from_fn(|i|[if i%7==0 {255}else if i%5==0 {35}else{next()%128},next(),next(),next()]);let before=records;update_spells(slot,mp,status1,used,&mut records,&mut max_cost);
        writeln!(rows,"{case},{slot},{mp},{status1},{used},{before_cost},{},{max_cost},{}",hex(&before.concat()),hex(&records.concat())).unwrap();
    }
    fs::write(directory.join("spells.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn mp_cap_and_imp_exception_follow_source_comparisons() {
        let mut records=[[0,0,0,255];79];records[1]=[35,0,0,10];let mut cost=0;
        update_spells(0,999,32,0,&mut records,&mut cost);assert_eq!(cost,255);assert_ne!(records[0][1]&128,0);assert_eq!(records[1][1]&128,0);
        records[1][3]=255;update_spells(0,999,0,0,&mut records,&mut cost);assert_ne!(records[1][1]&128,0);
    }
    #[test]fn offering_changes_targeting_and_weapon_flags_gate_commands() {
        let props=CommandProperties::load(Path::new("extracted/twue-3.07")).unwrap();let mut records=[[0,0,0],[7,0,0],[11,0,0],[3,0,0]];let mut changed=0;
        update_commands(CommandContext {slot:1,status1:0,status2:0,right_flags:2,left_flags:0,relic4:1,morph_counter:16},&props,&mut records,&mut changed).unwrap();
        assert_eq!(records[0][2],0x4e);assert_eq!((records[1][1],records[2][1],records[3][1],changed),(0,128,0,2));
    }
}
