//! Post-action Condemned, Quetzalli and near-fatal relic handlers.
use crate::{battle_rng::{BattleRng,RngTable},battle_scheduler::Scheduler,battle_commands::{CommandLists,Payload},battle_targets::actor_mask};
use std::{fs,path::Path};
/// C2/09B4. The source ADC retains SBC carry: the non-underflow branch adds 21.
pub fn start_condemn(level:u8,rng:&mut BattleRng,table:&RngTable)->u8 {
    let amount=level.wrapping_add(rng.below(table,level));
    if amount<=60 {60-amount+21}else{20}
}
/// C2/0B4A. Removes scheduled execution, not stored command lists.
pub fn quetzalli(slot:usize,s:&mut Scheduler,jumping:&mut u16,update_high:&mut u8)->Result<(),String> {
    if slot>=10 {return Err("Quetzalli actor outside ten slots".into());}
    *jumping|=actor_mask(slot);s.actors[slot].flags0=(s.actors[slot].flags0&0x9b)|8;
    let ring=&mut s.queues.action;let mut index=ring.head;
    // CPY/BCC do-while, including head==tail and unsigned wrapped intervals.
    loop {if ring.bytes[usize::from(index)]==(slot*2) as u8 {ring.bytes[usize::from(index)]=255;}index=index.wrapping_add(1);if index>=ring.tail {break;}}
    *update_high&=127;s.actors[slot].gauge.advance_counter&=255;s.actors[slot].gauge.advance_duration=224;
    Ok(())
}
/// C2/0A4A. B8 is working storage; its high byte survives spell writes.
pub fn near_fatal(slot:usize,s:&mut Scheduler,commands:&mut CommandLists,status2:u8,relic5:u8,update_high:&mut u8,working_b8:&mut u16,mp_flags:&mut u8)->Result<(),String> {
    if slot>=10 {return Err("Near-fatal actor outside ten slots".into());}
    if s.actors[slot].flags0&16!=0 || status2&2==0 || *update_high&2==0 {return Ok(());}
    *update_high^=2;
    for (bit,spell) in [(1,37u8),(2,28),(4,36)] {
        if relic5&bit!=0 {
            *working_b8=(*working_b8&0xff00)|u16::from(spell);*mp_flags&=!0x40;
            commands.immediate(Payload {command:38,attack:(slot*2) as u8,targets:*working_b8,mp_cost:0},&mut s.queues)?;
        }
    }
    Ok(())
}
fn hex(b:&[u8])->String {b.iter().map(|v|format!("{v:02X}")).collect()}
fn payloads(c:&CommandLists)->String {let mut bytes=Vec::new();for p in c.payloads {bytes.extend([p.command,p.attack]);bytes.extend(p.targets.to_le_bytes());bytes.push(p.mp_cost);}hex(&bytes)}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;
    let mut rows=String::from("level,seed,countdown,index\n");
    for level in 0..=255 {for seed in 0..=255 {let mut rng=BattleRng {index:seed};let countdown=start_condemn(level,&mut rng,&table);writeln!(rows,"{level},{seed},{countdown},{}",rng.index).unwrap();}}
    fs::write(dir.join("condemn.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,slot,jumping,flags,update,advance,duration,head,tail,queue,jumping_out,flags_out,update_out,advance_out,duration_out,queue_out\n");
    for case in 0..2048usize {
        let slot=case%10;let mut s=Scheduler::default();let mut jumping=(case as u16).wrapping_mul(97);let mut update=(case*29) as u8;s.actors[slot].flags0=(case*17) as u8;s.actors[slot].gauge.advance_counter=(case as u16).wrapping_mul(111);s.actors[slot].gauge.advance_duration=(case*53) as u8;
        s.queues.action.head=[0,250,255,20][case/3%4];s.queues.action.tail=[0,3,255,200][case/7%4];s.queues.action.bytes=std::array::from_fn(|i|if i%3==0 {(slot*2) as u8}else{((slot+1)%10*2) as u8});
        let before=format!("{case},{slot},{jumping},{},{update},{},{},{},{},{}",s.actors[slot].flags0,s.actors[slot].gauge.advance_counter,s.actors[slot].gauge.advance_duration,s.queues.action.head,s.queues.action.tail,hex(&s.queues.action.bytes));
        quetzalli(slot,&mut s,&mut jumping,&mut update)?;
        writeln!(rows,"{before},{jumping},{},{update},{},{},{}",s.actors[slot].flags0,s.actors[slot].gauge.advance_counter,s.actors[slot].gauge.advance_duration,hex(&s.queues.action.bytes)).unwrap();
    }
    fs::write(dir.join("quetzalli.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,slot,flags,status2,relic,update,b8,b1,links,immediate,payloads,update_out,b8_out,b1_out,links_out,immediate_out,payloads_out\n");
    for case in 0..4096usize {
        let slot=case%10;let mut s=Scheduler::default();let mut c=CommandLists::default();let flags=if case/8%3==0 {16}else{1};s.actors[slot].flags0=flags;let status2=if case/16%3==0 {0}else{2};let relic=(case/2) as u8;let mut update=if case/32%3==0 {128}else{130};let mut b8=(case as u16).wrapping_mul(89);let mut b1=(case*13) as u8;
        for i in 0..case%4 {c.immediate(Payload {command:35,attack:i as u8,targets:0x102,mp_cost:0},&mut s.queues)?;}
        let before=format!("{case},{slot},{flags},{status2},{relic},{update},{b8},{b1},{},{},{}",hex(&c.links),s.queues.immediate.unwrap_or(255),payloads(&c));
        near_fatal(slot,&mut s,&mut c,status2,relic,&mut update,&mut b8,&mut b1)?;
        writeln!(rows,"{before},{update},{b8},{b1},{},{},{}",hex(&c.links),s.queues.immediate.unwrap_or(255),payloads(&c)).unwrap();
    }
    fs::write(dir.join("near-fatal.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn condemn_preserves_carry_and_wrapping_level_addition() {
        let table=RngTable::identity();let mut rng=BattleRng {index:255};assert_eq!(start_condemn(0,&mut rng,&table),81);
        let mut rng=BattleRng {index:255};assert_eq!(start_condemn(60,&mut rng,&table),21);
        let mut rng=BattleRng {index:255};assert_eq!(start_condemn(61,&mut rng,&table),20);
        let mut rng=BattleRng {index:255};assert_eq!(start_condemn(255,&mut rng,&table),20);
        let mut rng=BattleRng {index:254};assert_eq!(start_condemn(255,&mut rng,&table),20);
        let mut rng=BattleRng {index:254};assert_eq!(start_condemn(130,&mut rng,&table),78);
    }
    #[test]fn quetzalli_removes_execution_but_retains_stored_command() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();c.normal(0,Payload {command:0,attack:239,targets:256,mp_cost:0}).unwrap();s.queues.action.push(0);s.actors[0].flags0=255;s.actors[0].gauge.advance_counter=0xabcd;let mut jumping=0;let mut update=255;
        quetzalli(0,&mut s,&mut jumping,&mut update).unwrap();assert_eq!(s.queues.action.pop(),None);assert_ne!(c.normal[0],255);assert_eq!((jumping,update,s.actors[0].flags0,s.actors[0].gauge.advance_counter,s.actors[0].gauge.advance_duration),(1,127,155,205,224));
    }
    #[test]fn near_fatal_is_once_per_battle_and_keeps_spell_order() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut update=130;let mut b8=0xabcd;let mut b1=255;
        near_fatal(0,&mut s,&mut c,2,7,&mut update,&mut b8,&mut b1).unwrap();let id=s.queues.immediate.unwrap();assert_eq!(c.payloads[id as usize].targets,0xab25);let second=c.links[id as usize];assert_eq!(c.payloads[second as usize].targets,0xab1c);let last=c.links[second as usize];assert_eq!(c.payloads[last as usize].targets,0xab24);assert_eq!((update,b8,b1),(128,0xab24,191));
        let before=c.clone();near_fatal(0,&mut s,&mut c,2,7,&mut update,&mut b8,&mut b1).unwrap();assert_eq!(c,before);
    }
}
