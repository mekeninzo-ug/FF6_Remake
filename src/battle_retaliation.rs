//! CheckRetal C2/4C5B..4D1E, including Retort, Interceptor and Black Belt.
use crate::{battle_after_action::{self,Actor as BattleActor,Globals as BattleGlobals},battle_presence::Actor as PresenceActor,battle_commands::{CommandLists,MpCosts,Payload},battle_scheduler::Scheduler,battle_targets::actor_mask,battle_rng::{BattleRng,RngTable},damage::AttackTable};
use std::{fs,path::Path};
#[derive(Clone,Debug)]pub struct State {pub enabled:u8,pub pending_attackers:u16,pub newly_dead:u16,pub physical_flags:u8,pub interceptor:u8,pub blackbelt_blocked:u8,pub ai_high:[u8;10],pub command:u8,pub attack:u8}
impl Default for State {fn default()->Self {Self {enabled:1,pending_attackers:0,newly_dead:0,physical_flags:1,interceptor:255,blackbelt_blocked:0,ai_high:[255;10],command:255,attack:255}}}
fn emit(slot:usize,s:&mut Scheduler,c:&mut CommandLists,bg:&mut BattleGlobals,r:&State,costs:&MpCosts)->Result<(),String> {
    let cost=costs.get(r.command,r.attack,(slot*2) as u8,&mut bg.b1)?;c.retaliation(slot,Payload {command:r.command,attack:r.attack,targets:bg.b8,mp_cost:cost},&mut s.queues)?;Ok(())
}
pub fn run(s:&mut Scheduler,c:&mut CommandLists,a:&[BattleActor;10],meta:&[PresenceActor;10],bg:&mut BattleGlobals,r:&mut State,rng:&mut BattleRng,table:&RngTable,costs:&MpCosts)->Result<(),String> {
    for slot in (0..10).rev() {
        if s.actors[slot].flags0&1==0 || r.enabled==0 {continue;}bg.b8=0;
        let previous=a[slot].previous_attacker;
        if previous&128!=0 {
            let offset=previous.wrapping_mul(2);
            if offset!=(slot*2) as u8 {if offset>=20 || offset&1!=0 {return Err(format!("Retaliation previous attacker {previous:02X} is outside ten actors"));}bg.b8=actor_mask(usize::from(offset/2));r.pending_attackers&=!actor_mask(slot);}
        }
        if r.newly_dead&actor_mask(slot)==0 && (bg.b1&1!=0 || bg.b8==0) {continue;}
        if r.ai_high[slot]&128==0 && c.retaliation[slot]&128!=0 {r.command=31;emit(slot,s,c,bg,r,costs)?;continue;}
        if slot>=4 || r.physical_flags&1==0 {continue;}
        if meta[slot].retaliation_flags&1!=0 {r.command=7;r.attack=86;emit(slot,s,c,bg,r,costs)?;continue;}
        if bg.b8>>8==0 {continue;}
        if r.interceptor==(slot*2) as u8 {let roll=rng.next(table);if roll&1!=0 {r.command=2;r.attack=252+((roll>>1)&1);emit(slot,s,c,bg,r,costs)?;continue;}}
        if (actor_mask(slot) as u8)&r.blackbelt_blocked!=0 || a[slot].relic4&2==0 {continue;}
        if rng.next(table)>=192 {continue;}
        let status12=u16::from(s.actors[slot].status1)|(u16::from(a[slot].status2)<<8);let status34=u16::from(a[slot].status3)|(u16::from(a[slot].status4)<<8);
        if status12&0xa0ca!=0 || status34&0x3211!=0 {continue;}
        r.command=0;r.attack=0;emit(slot,s,c,bg,r,costs)?;
    }
    Ok(())
}
fn snapshot(s:&Scheduler,c:&CommandLists,a:&[BattleActor;10],bg:&BattleGlobals,meta:&[PresenceActor;10],r:&State,rng:&BattleRng,costs:&MpCosts)->String {
    use std::fmt::Write;let mut result=battle_after_action::snapshot(s,c,a,bg,rng);let mut put=|addr:u32,val:u16,width:usize|{for i in 0..width {write!(result,"{:06X}{:02X}",addr+i as u32,(val>>(i*8)) as u8).unwrap();}};
    for (addr,val,width) in [(0x341a,u16::from(r.enabled),1),(0x33fe,r.pending_attackers,2),(0x3a56,r.newly_dead,2),(0x11a2,u16::from(r.physical_flags),1),(0x3416,u16::from(r.interceptor),1),(0x3419,u16::from(r.blackbelt_blocked),1),(0x3a7a,u16::from(r.command),1),(0x3a7b,u16::from(r.attack),1)] {put(0x7e0000+addr,val,width);}
    for slot in 0..10 {put(0x7e3269+slot as u32*2,u16::from(r.ai_high[slot]),1);put(0x7e3e4c+slot as u32*2,u16::from(meta[slot].retaliation_flags),1);}
    put(0x7e3a68,u16::from(s.queues.retaliation.head)|(u16::from(s.queues.retaliation.tail)<<8),2);
    for (i,v) in s.queues.retaliation.bytes.iter().enumerate() {put(0x7e3920+i as u32,u16::from(*v),1);}
    for (i,v) in costs.master.iter().enumerate() {put(0x7e3084+i as u32,u16::from(*v),1);}
    result
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let attacks=AttackTable::load(root)?;let table=RngTable::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,before,after,effects\n");
    for case in 0..4096usize {
        let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let mut meta=[PresenceActor::default();10];let mut bg=BattleGlobals::default();let mut r=State::default();let mut costs=MpCosts::from_attacks(&attacks);
        bg.b1=next();bg.b8=u16::from_le_bytes([next(),next()]);r.enabled=if case%11==0 {0}else{1};r.pending_attackers=u16::from_le_bytes([next(),next()]);r.newly_dead=if case%2==0 {0}else{u16::from_le_bytes([next(),next()])};r.physical_flags=if case%3==0 {0}else{1};r.interceptor=(case%4*2) as u8;r.blackbelt_blocked=if case%5==0 {0}else{next()};r.command=next();r.attack=next();
        for slot in 0..10 {let b=&mut s.actors[slot];let actor=&mut a[slot];b.flags0=if case%3==0 {next()}else{1};b.status1=if case%5==0 {0}else{next()};actor.previous_attacker=[127,128|slot as u8,128|((slot+case)%10) as u8][(case+slot)%3];actor.status2=if case%5==0 {0}else{next()};actor.status3=if case%5==0 {0}else{next()};actor.status4=if case%5==0 {0}else{next()};actor.relic4=if case%5==0 {2}else{next()};actor.spells=std::array::from_fn(|_|[next(),next(),next(),next()]);r.ai_high[slot]=if (case+slot)%3==0 {0}else{255};meta[slot].retaliation_flags=if case%5==0 {0}else{next()};}
        for slot in 0..4 {costs.character[slot]=a[slot].spells.iter().map(|r|r[3]).collect();}
        costs.master=std::array::from_fn(|i|if (i+case)%3==0 {255}else{(i%79) as u8});
        for slot in 0..10 {if (slot+case)%5==0 {c.retaliation(slot,Payload {command:31,attack:238,targets:1,mp_cost:0},&mut s.queues)?;}}
        s.queues.retaliation.head=[0,250,255,20][case/3%4];s.queues.retaliation.tail=[0,3,255,200][case/7%4];s.queues.retaliation.bytes=std::array::from_fn(|i|if i%5==0 {255}else{(i%10*2) as u8});
        let mut rng=BattleRng {index:next()};let before=snapshot(&s,&c,&a,&bg,&meta,&r,&rng,&costs);run(&mut s,&mut c,&a,&meta,&mut bg,&mut r,&mut rng,&table,&costs)?;
        writeln!(rows,"{case},{before},{},",snapshot(&s,&c,&a,&bg,&meta,&r,&rng,&costs)).unwrap();
    }
    fs::write(dir.join("retaliation.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn costs()->MpCosts {MpCosts {master:[255;256],character:std::array::from_fn(|_|vec![0;79]),attacks:[0;256]}}
    #[test]fn newly_dead_actor_can_schedule_ai_during_retaliation_without_targets() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];a[6].previous_attacker=127;let meta=[PresenceActor::default();10];let mut bg=BattleGlobals::default();let mut r=State::default();let mut rng=BattleRng::default();s.actors[6].flags0=1;bg.b1=1;r.newly_dead=1024;r.ai_high[6]=0;r.attack=91;
        run(&mut s,&mut c,&a,&meta,&mut bg,&mut r,&mut rng,&RngTable::identity(),&costs()).unwrap();assert_eq!(c.payloads[c.retaliation[6] as usize],Payload {command:31,attack:91,targets:0,mp_cost:0});assert_eq!(s.queues.retaliation.pop(),Some(12));assert_eq!(rng.index,0);
    }
    #[test]fn interceptor_uses_two_low_random_bits_and_modified_character_mp_cost() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let meta=[PresenceActor::default();10];let mut bg=BattleGlobals::default();let mut r=State::default();let mut rng=BattleRng {index:2};let mut costs=costs();s.actors[0].flags0=1;a[0].previous_attacker=134;r.interceptor=0;costs.master[253]=0;costs.character[0][0]=17;
        run(&mut s,&mut c,&a,&meta,&mut bg,&mut r,&mut rng,&RngTable::identity(),&costs).unwrap();assert_eq!(c.payloads[c.retaliation[0] as usize],Payload {command:2,attack:253,targets:1024,mp_cost:17});assert_eq!(rng.index,3);
    }
    #[test]fn black_belt_roll_precedes_status_gate_and_uses_attack_zero() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let meta=[PresenceActor::default();10];let mut bg=BattleGlobals::default();let mut r=State::default();let mut rng=BattleRng::default();s.actors[0].flags0=1;s.actors[0].status1=64;a[0].previous_attacker=134;a[0].relic4=2;
        run(&mut s,&mut c,&a,&meta,&mut bg,&mut r,&mut rng,&RngTable::identity(),&costs()).unwrap();assert_eq!((rng.index,c.retaliation[0]),(1,255));s.actors[0].status1=0;
        run(&mut s,&mut c,&a,&meta,&mut bg,&mut r,&mut rng,&RngTable::identity(),&costs()).unwrap();assert_eq!(c.payloads[c.retaliation[0] as usize].attack,0);assert_eq!(rng.index,2);
    }
    #[test]fn retort_has_priority_even_when_attacker_is_a_character() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let mut meta=[PresenceActor::default();10];let mut bg=BattleGlobals::default();let mut r=State::default();let mut rng=BattleRng::default();s.actors[0].flags0=1;a[0].previous_attacker=129;a[0].relic4=2;meta[0].retaliation_flags=1;r.interceptor=0;
        run(&mut s,&mut c,&a,&meta,&mut bg,&mut r,&mut rng,&RngTable::identity(),&costs()).unwrap();assert_eq!(c.payloads[c.retaliation[0] as usize],Payload {command:7,attack:86,targets:2,mp_cost:0});assert_eq!(rng.index,0);
    }
}
