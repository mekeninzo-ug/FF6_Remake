//! Post-action readiness C2/08C6..09B3, with source queue/menu helper behavior.
use crate::{battle_commands::CommandLists,battle_scheduler::Scheduler,battle_targets::actor_mask};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug)]
pub struct Context {
    pub slot:usize,pub quick_target:u8,pub colosseum:u8,pub untargetable:u16,pub flags_b1:u8,
    pub property:u8,pub ai_high:u8,pub control:u8,pub seize:u8,pub charm:u8,
    pub status2:u8,pub status3:u8,pub status4:u8,pub update_high:u8,
}
#[derive(Clone,Copy,Debug)]pub enum Mode {Pending,Player,Cancel,Disable}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Effect {OpenMenu(u8),CloseMenu(u8)}
fn cancel(s:&mut Scheduler,c:Context,effects:&mut Vec<Effect>) {
    let mut mask=0xf9;
    if c.status4&0x20==0 && (actor_mask(c.slot) as u8)&(c.untargetable as u8)==0 && s.actors[c.slot].flags0&0x80!=0 {mask=0x79;s.enqueue_advance(c.slot);}
    s.actors[c.slot].flags0&=mask;
    if c.slot<4 {effects.push(Effect::CloseMenu(c.slot as u8));}
}
pub fn run(mode:Mode,s:&mut Scheduler,commands:&CommandLists,c:Context,effects:&mut Vec<Effect>)->Result<(),String> {
    if c.slot>=10 {return Err("Readiness actor outside ten battle slots".into());}
    let offset=(c.slot*2) as u8;
    match mode {
        Mode::Cancel=>cancel(s,c,effects),
        Mode::Disable=>{
            s.actors[c.slot].flags0&=0xd3;s.actors[c.slot].flags1&=0xbf;
            s.actors[c.slot].gauge.counter=(s.actors[c.slot].gauge.counter&255)|0x100;cancel(s,c,effects);
        },
        Mode::Pending=>{
            s.actors[c.slot].flags0|=0x50;
            if c.quick_target&0x80==0 && c.quick_target!=offset {return Ok(());}
            if s.actors[c.slot].status1&0xc0!=0 || c.status3&0x10!=0 {return Ok(());}
            s.actors[c.slot].flags0&=0xef;
            if c.control&0x80==0 || c.status2&0x80!=0 || c.status4&2!=0 || c.seize&0x80==0 {return Ok(());}
            s.actors[c.slot].flags0&=0xbf;
            if s.actors[c.slot].flags1&0x80==0 {return Ok(());}
            s.actors[c.slot].flags1&=0x7f;
            if commands.normal[c.slot]==255 {return Ok(());}
            if s.actors[c.slot].flags1&1!=0 {s.queues.action.push(offset);}else{s.enqueue_advance(c.slot);}
        },
        Mode::Player=>{
            if c.slot>=4 || c.property==13 || c.ai_high&0x80==0 || c.colosseum!=0 {return Ok(());}
            s.actors[c.slot].flags0|=2;if offset==c.quick_target {s.actors[c.slot].flags0|=0x88;}
            let status12=u16::from(s.actors[c.slot].status1)|(u16::from(c.status2)<<8);let status34=u16::from(c.status3)|(u16::from(c.status4)<<8);
            if (actor_mask(c.slot) as u8)&(c.untargetable as u8)!=0 || (c.seize&c.charm)&0x80==0 || status12&0xb0c2!=0 || status34&0x2101!=0 {cancel(s,c,effects);return Ok(());}
            if s.actors[c.slot].flags0&0x80==0 || commands.normal[c.slot]&0x80==0 {return Ok(());}
            s.actors[c.slot].flags0|=8;
            // C2/11EF's additional menu gates and encoded actor argument.
            if c.update_high&0x80!=0 && c.flags_b1&0x80==0 {s.actors[c.slot].flags0|=4;effects.push(Effect::OpenMenu(c.slot as u8|((s.actors[c.slot].extra_status&1)<<7)));}
        },
    }
    Ok(())
}
pub fn capture(directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let mut seed=0x6ff6u32;
    let mut rows=String::from("case,mode,slot,quick,colosseum,untargetable,b1,property,ai,control,seize,charm,status1,status2,status3,status4,update_high,extra,flags0,flags1,gauge,head,advance_tail,action_tail,flags0_out,flags1_out,gauge_out,advance_tail_out,action_tail_out,advance,action,effects\n");
    for case in 0..16384usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let slot=case%10;let mut c=Context {slot,quick_target:[255,(slot*2) as u8,((slot+1)%10*2) as u8][case/3%3],colosseum:if case%17==0 {1}else{0},untargetable:u16::from_le_bytes([next(),next()]),flags_b1:next(),property:if case%13==0 {13}else{0},ai_high:if case%7==0 {0}else{255},control:if case/5%5==0 {0}else{255},seize:if case/7%5==0 {0}else{255},charm:if case/11%5==0 {0}else{255},status2:next(),status3:next(),status4:next(),update_high:next()};
        if case%5==0 {c.status2=0;c.status3=0;c.status4=0;}
        if case%7==0 {c.untargetable=0;}
        let mut s=Scheduler::default();let mut commands=CommandLists::default();commands.normal[slot]=[255,0,127,128][case/3%4];s.actors[slot].status1=if case%5==0 {0}else{next()};s.actors[slot].extra_status=next();s.actors[slot].flags0=next();s.actors[slot].flags1=next();s.actors[slot].gauge.counter=u16::from_le_bytes([next(),next()]);s.queues.advance.tail=next();s.queues.action.tail=next();
        let before=s.actors[slot];let tails=(s.queues.advance.tail,s.queues.action.tail);let mode=[Mode::Pending,Mode::Player,Mode::Cancel,Mode::Disable][case/10%4];let mut effects=Vec::new();run(mode,&mut s,&commands,c,&mut effects)?;
        let effect=effects.iter().map(|e|match e {Effect::OpenMenu(arg)=>format!("open:{arg}"),Effect::CloseMenu(arg)=>format!("close:{arg}")}).collect::<Vec<_>>().join("|");
        let hex=|a:&[u8]|a.iter().map(|v|format!("{v:02X}")).collect::<String>();
        writeln!(rows,"{case},{},{slot},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{effect}",case/10%4,c.quick_target,c.colosseum,c.untargetable,c.flags_b1,c.property,c.ai_high,c.control,c.seize,c.charm,before.status1,c.status2,c.status3,c.status4,c.update_high,before.extra_status,before.flags0,before.flags1,before.gauge.counter,commands.normal[slot],tails.0,tails.1,s.actors[slot].flags0,s.actors[slot].flags1,s.actors[slot].gauge.counter,s.queues.advance.tail,s.queues.action.tail,hex(&s.queues.advance.bytes),hex(&s.queues.action.bytes)).unwrap();
    }
    fs::write(directory.join("readiness.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn context(slot:usize)->Context {Context {slot,quick_target:255,colosseum:0,untargetable:0,flags_b1:0,property:0,ai_high:255,control:255,seize:255,charm:255,status2:0,status3:0,status4:0,update_high:128}}
    #[test]fn frozen_command_survives_until_status_commit_then_reenters_action_queue() {
        use crate::{battle_commands::Payload,battle_status_update as status};
        let mut scheduler=Scheduler::default();let mut commands=CommandLists::default();commands.normal(6,Payload {command:0,attack:239,targets:1,mp_cost:0}).unwrap();scheduler.actors[6].flags0=1;scheduler.actors[6].flags1=129;
        let mut c=context(6);c.status4=2;let mut effects=Vec::new();run(Mode::Pending,&mut scheduler,&commands,c,&mut effects).unwrap();assert_eq!((scheduler.actors[6].flags0,scheduler.actors[6].flags1),(0x41,0x81));assert_eq!(scheduler.queues.action.pop(),None);
        let mut actors=[status::Actor::default();10];let a=&mut actors[6];a.flags=1;a.status.current34=512;a.status.pending.clear34=512;a.status.allowed12=65535;a.status.allowed34=65535;a.status.hp=40;a.status.max_hp=40;status::update(&mut actors,&mut status::Globals::default());
        c.status4=(actors[6].status.current34>>8) as u8;run(Mode::Pending,&mut scheduler,&commands,c,&mut effects).unwrap();assert_eq!(scheduler.queues.action.pop(),Some(12));assert_eq!(scheduler.actors[6].flags1,1);assert!(effects.is_empty());
    }
    #[test]fn quick_owned_by_another_actor_leaves_work_blocked() {
        let mut scheduler=Scheduler::default();scheduler.actors[6].flags0=1;scheduler.actors[6].flags1=128;let mut c=context(6);c.quick_target=0;
        run(Mode::Pending,&mut scheduler,&CommandLists::default(),c,&mut Vec::new()).unwrap();assert_eq!((scheduler.actors[6].flags0,scheduler.actors[6].flags1),(0x51,128));
    }
    #[test]fn quick_player_can_request_menu_with_source_encoded_argument() {
        let mut scheduler=Scheduler::default();scheduler.actors[1].flags0=1;scheduler.actors[1].extra_status=1;let mut c=context(1);c.quick_target=2;let mut effects=Vec::new();
        run(Mode::Player,&mut scheduler,&CommandLists::default(),c,&mut effects).unwrap();assert_eq!(effects,vec![Effect::OpenMenu(129)]);assert_eq!(scheduler.actors[1].flags0,0x8f);
    }
    #[test]fn disabling_atb_preserves_low_gauge_and_hidden_cancel_does_not_enqueue() {
        let mut scheduler=Scheduler::default();scheduler.actors[0].flags0=255;scheduler.actors[0].flags1=255;scheduler.actors[0].gauge.counter=0xabcd;let mut c=context(0);c.status4=0x20;let mut effects=Vec::new();
        run(Mode::Disable,&mut scheduler,&CommandLists::default(),c,&mut effects).unwrap();assert_eq!((scheduler.actors[0].flags0,scheduler.actors[0].flags1,scheduler.actors[0].gauge.counter),(0xd1,0xbf,0x01cd));assert_eq!(scheduler.queues.advance.pop(),None);assert_eq!(effects,vec![Effect::CloseMenu(0)]);
    }
}
