//! UpdateMorph C2/0AA8..0B35 and SaveMorphCounter C2/0B36..0B49.
use crate::{battle_action_init::CommandProperties,battle_menu_state::{self,CommandContext}};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug)]
pub struct State {pub actor:u8,pub counter:u16,pub rate:u16,pub saved:u8,pub story_flags:u8,pub changed:u16}
#[derive(Clone,Copy,Debug)]
pub struct Context {pub slot:usize,pub b1:u8,pub gauge:u16,pub status4:u8,pub command:CommandContext}
pub fn save_counter(s:&mut State) {if s.actor&128==0 {s.saved=((u16::from(s.saved)*(s.counter>>8))>>8) as u8;}}
pub fn update(c:Context,s:&mut State,flags0:&mut u8,morph_gauge:&mut u8,records:&mut[[u8;3];4],properties:&CommandProperties)->Result<(),String> {
    if c.slot>=10 {return Err("Morph actor outside ten slots".into());}
    if c.b1&1==0 && c.gauge>>8==0 {*flags0|=0x88;}
    let morphed=c.status4&8!=0;let find=if morphed {3}else{4};
    for r in records.iter_mut() {if r[0]==find {r[0]^=7;}}
    if !morphed {
        save_counter(s);s.actor=255;*morph_gauge=0;
        if c.slot<4 {let mut command=c.command;command.slot=c.slot;command.morph_counter=s.saved;battle_menu_state::update_commands(command,properties,records,&mut s.changed)?;}
        s.counter=0;
    }else if s.actor&128!=0 && s.story_flags&4==0 {
        s.actor=(c.slot*2) as u8;s.counter=65535;
        // Hardware divide-by-zero returns FFFF; the original routine does not guard it.
        let mut quotient=if s.saved==0 {65535}else{65535/u16::from(s.saved)};
        if s.story_flags&2!=0 {quotient>>=1;}
        quotient>>=3;s.rate=quotient.min(2047)+1;
    }
    Ok(())
}
fn hex(records:&[[u8;3];4])->String {records.concat().iter().map(|v|format!("{v:02X}")).collect()}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let props=CommandProperties::load(root)?;let mut seed=0x6ff6u32;
    let mut rows=String::from("case,slot,b1,gauge,status4,status1,status2,right,left,relic,actor,counter,rate,saved,story,changed,flags,morph_gauge,records,actor_out,counter_out,rate_out,saved_out,changed_out,flags_out,morph_gauge_out,records_out\n");
    for case in 0..8192usize {
        let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let slot=case%10;
        let command=CommandContext {slot,status1:next(),status2:next(),right_flags:next(),left_flags:next(),relic4:next(),morph_counter:0};
        let c=Context {slot,b1:next(),gauge:if case%3==0 {u16::from(next())}else{u16::from_le_bytes([next(),next()])},status4:next(),command};
        let mut s=State {actor:[255,0,2,18][case/5%4],counter:u16::from_le_bytes([next(),next()]),rate:u16::from_le_bytes([next(),next()]),saved:[0,1,15,16,255,next()][case/7%6],story_flags:next(),changed:u16::from_le_bytes([next(),next()])};let before=s;
        let mut flags=next();let mut morph_gauge=next();let mut records=std::array::from_fn(|i|[if i<2 {3+i as u8}else{next()%32},next(),next()]);
        let prefix=format!("{case},{slot},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{flags},{morph_gauge},{}",c.b1,c.gauge,c.status4,command.status1,command.status2,command.right_flags,command.left_flags,command.relic4,before.actor,before.counter,before.rate,before.saved,before.story_flags,before.changed,hex(&records));
        update(c,&mut s,&mut flags,&mut morph_gauge,&mut records,&props)?;
        writeln!(rows,"{prefix},{},{},{},{},{},{flags},{morph_gauge},{}",s.actor,s.counter,s.rate,s.saved,s.changed,hex(&records)).unwrap();
    }
    fs::write(dir.join("morph.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn context(slot:usize,morphed:bool)->Context {Context {slot,b1:0,gauge:0,status4:if morphed {8}else{0},command:CommandContext {slot,status1:0,status2:0,right_flags:0,left_flags:0,relic4:0,morph_counter:0}}}
    fn state()->State {State {actor:255,counter:0,rate:0,saved:16,story_flags:0,changed:0}}
    #[test]fn morph_and_revert_preserve_remaining_resource_and_menu() {
        let props=CommandProperties::load(Path::new("extracted/twue-3.07")).unwrap();let mut s=state();let mut flags=1;let mut gauge=77;let mut records=[[3,0,0],[4,0,0],[0,0,0],[255,0,0]];
        update(context(0,true),&mut s,&mut flags,&mut gauge,&mut records,&props).unwrap();assert_eq!((s.actor,s.counter,s.rate,flags,records[0][0]),(0,65535,512,137,4));s.counter=0x8000;
        update(context(0,false),&mut s,&mut flags,&mut gauge,&mut records,&props).unwrap();assert_eq!((s.saved,s.actor,s.counter,gauge,records[0][0]),(8,255,0,0,3));assert_ne!(records[0][1]&128,0);
    }
    #[test]fn permanent_or_existing_morph_does_not_reinitialize_timer() {
        let props=CommandProperties::load(Path::new("extracted/twue-3.07")).unwrap();let mut s=state();s.story_flags=4;s.counter=123;s.rate=77;let mut records=[[3,0,0];4];let mut flags=0;let mut gauge=1;
        update(context(1,true),&mut s,&mut flags,&mut gauge,&mut records,&props).unwrap();assert_eq!((s.actor,s.counter,s.rate),(255,123,77));s.story_flags=0;s.actor=0;
        update(context(1,true),&mut s,&mut flags,&mut gauge,&mut records,&props).unwrap();assert_eq!((s.actor,s.counter,s.rate),(0,123,77));
    }
    #[test]fn zero_saved_resource_and_post_phunbaba_rate_follow_hardware_math() {
        let props=CommandProperties::load(Path::new("extracted/twue-3.07")).unwrap();let mut s=state();s.saved=0;let mut records=[[3,0,0];4];let mut flags=0;let mut gauge=0;
        update(context(0,true),&mut s,&mut flags,&mut gauge,&mut records,&props).unwrap();assert_eq!(s.rate,2048);
        s=state();s.story_flags=2;update(context(0,true),&mut s,&mut flags,&mut gauge,&mut records,&props).unwrap();assert_eq!(s.rate,256);
    }
}
