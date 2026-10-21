//! AfterAction1 C2/069B..083E, including character graphics-buffer projection.
use crate::{battle_after_action::{self,Actor as BattleActor,Globals as BattleGlobals},battle_commands::{CommandLists,Payload},battle_scheduler::Scheduler,battle_targets::actor_mask,battle_rng::{BattleRng,RngTable},battle_action_init::CommandProperties};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug)]
pub struct Actor {pub hp:u16,pub max_hp:u16,pub max_mp:u16,pub control_target:u8,pub love_target:u8,pub love_attacker:u8,pub charm_target:u8}
impl Default for Actor {fn default()->Self {Self {hp:0,max_hp:0,max_mp:0,control_target:255,love_target:255,love_attacker:255,charm_target:255}}}
#[derive(Clone,Debug)]
pub struct Globals {pub skip_targets:u16,pub dead_monsters:u8,pub reraise_exclusion:u8,pub controlled_flip:u8,pub zinger_attacker:u8,pub zinger_target:u8,pub quick_counter:u8,pub end_event:u8,pub graphics:[[u16;6];4]}
impl Default for Globals {fn default()->Self {Self {skip_targets:0,dead_monsters:0,reraise_exclusion:0,controlled_flip:0,zinger_attacker:255,zinger_target:255,quick_counter:0,end_event:0,graphics:[[0;6];4]}}}
fn index(offset:u8)->Result<usize,String> {if offset<20 && offset&1==0 {Ok(usize::from(offset/2))}else{Err(format!("Special-status relationship offset {offset:02X} is outside ten aligned actors"))}}
fn update_seize(slot:usize,s:&mut Scheduler) {s.actors[slot].flags1|=64;s.actors[slot].update_flags|=64;s.actors[slot].flags0&=127;}
fn fix_dead(slot:usize,s:&mut Scheduler) {s.actors[slot].status1&=127;s.actors[slot].update_flags&=191;}
fn immediate(command:u8,attack:u8,commands:&mut CommandLists,s:&mut Scheduler,g:&mut BattleGlobals)->Result<(),String> {g.b1&=!64;commands.immediate(Payload {command,attack,targets:g.b8,mp_cost:0},&mut s.queues)?;Ok(())}
fn remove_control(target:usize,controller:usize,s:&mut Scheduler,a:&mut[BattleActor;10],extra:&mut[Actor;10],g:&mut Globals) {
    s.actors[controller].extra_status&=254;a[controller].status4&=239;a[target].control=255;extra[controller].control_target=255;g.controlled_flip&=!((actor_mask(target)>>8) as u8);update_seize(target,s);update_seize(controller,s);
}
pub(crate) fn cleanup_relations(slot:usize,s:&mut Scheduler,commands:&mut CommandLists,a:&mut[BattleActor;10],bg:&mut BattleGlobals,extra:&mut[Actor;10],g:&mut Globals)->Result<(),String> {
            if g.zinger_target==(slot*2) as u8 {
                let attacker=index(g.zinger_attacker)?;bg.b8=(u16::from((actor_mask(attacker)>>8) as u8)<<8)|4;
                immediate(36,0,commands,s,bg)?;bg.b8=(bg.b8&0xff00)|2;immediate(36,8,commands,s,bg)?;g.zinger_attacker=255;g.zinger_target=255;
            }
            if extra[slot].love_target&128==0 {let target=index(extra[slot].love_target)?;extra[slot].love_target=255;extra[target].love_attacker=255;}
            if extra[slot].love_attacker&128==0 {let attacker=index(extra[slot].love_attacker)?;extra[attacker].love_target=255;extra[slot].love_attacker=255;}
            if extra[slot].charm_target&128==0 {let target=index(extra[slot].charm_target)?;extra[slot].charm_target=255;a[target].charm=255;}
            if a[slot].charm&128==0 {let attacker=index(a[slot].charm)?;extra[attacker].charm_target=255;a[slot].charm=255;}
            if bg.quick_target==(slot*2) as u8 {bg.quick_target=255;g.quick_counter=255;}
    Ok(())
}
pub fn run(s:&mut Scheduler,commands:&mut CommandLists,a:&mut[BattleActor;10],bg:&mut BattleGlobals,extra:&mut[Actor;10],g:&mut Globals)->Result<(),String> {
    for slot in (0..10).rev() {
        if s.actors[slot].flags0&1==0 || g.skip_targets&actor_mask(slot)!=0 {continue;}
        let status12=u16::from(s.actors[slot].status1)|(u16::from(a[slot].status2)<<8);let status34=u16::from(a[slot].status3)|(u16::from(a[slot].status4)<<8);
        if status12&0xb0c2!=0 || status34&0x0311!=0 {extra[slot].control_target|=128;a[slot].control|=128;}
        if s.actors[slot].status1&0x82!=0 {extra[slot].hp=0;}
        if s.actors[slot].status1&0xc2!=0 {
            g.dead_monsters|=(actor_mask(slot)>>8) as u8;
            cleanup_relations(slot,s,commands,a,bg,extra,g)?;
        }
        if s.actors[slot].status1&128==0 {continue;}
        if slot<4 && a[slot].property==14 {g.end_event=6;}
        if bg.jumping&actor_mask(slot)!=0 {fix_dead(slot,s);a[slot].update_high&=251;}
        if s.actors[slot].status1&2!=0 {fix_dead(slot,s);}
        if s.actors[slot].status1&128!=0 && a[slot].status4&4!=0 {a[slot].status4&=251;g.reraise_exclusion&=!((actor_mask(slot)>>8) as u8);bg.b8=(bg.b8&0xff00)|48;immediate(38,(slot*2) as u8,commands,s,bg)?;}
    }
    // This pass is unconditional, including absent and skipped actors.
    for slot in (0..10).rev() {
        if a[slot].control!=255 && a[slot].control&128!=0 {let controller=index(a[slot].control&127)?;remove_control(slot,controller,s,a,extra,g);}
        if extra[slot].control_target!=255 && extra[slot].control_target&128!=0 {let target=index(extra[slot].control_target&127)?;remove_control(target,slot,s,a,extra,g);}
    }
    for slot in 0..4 {g.graphics[slot]=[extra[slot].hp,extra[slot].max_hp,a[slot].mp,extra[slot].max_mp,u16::from(s.actors[slot].status1)|(u16::from(a[slot].status2)<<8),u16::from(a[slot].status3)|(u16::from(a[slot].status4)<<8)];}
    Ok(())
}
pub(crate) fn snapshot(s:&Scheduler,c:&CommandLists,a:&[BattleActor;10],bg:&BattleGlobals,extra:&[Actor;10],g:&Globals,rng:&BattleRng)->String {
    use std::fmt::Write;let mut result=battle_after_action::snapshot(s,c,a,bg,rng);let mut put=|addr:u32,val:u16,width:usize| {for i in 0..width {write!(result,"{:06X}{:02X}",addr+i as u32,(val>>(i*8)) as u8).unwrap();}};
    for (addr,val,width) in [(0x2f4e,g.skip_targets,2),(0x3a3a,u16::from(g.dead_monsters),1),(0x2f2f,u16::from(g.reraise_exclusion),1),(0x2f54,u16::from(g.controlled_flip),1),(0x33f8,u16::from(g.zinger_attacker),1),(0x33f9,u16::from(g.zinger_target),1),(0x3402,u16::from(g.quick_counter),1),(0x3a6e,u16::from(g.end_event),1)] {put(0x7e0000+addr,val,width);}
    for (slot,e) in extra.iter().enumerate() {let x=slot as u32*2;for (addr,val,width) in [(0x3bf4,e.hp,2),(0x3c1c,e.max_hp,2),(0x3c30,e.max_mp,2),(0x32b8,u16::from(e.control_target),1),(0x336c,u16::from(e.love_target),1),(0x336d,u16::from(e.love_attacker),1),(0x3394,u16::from(e.charm_target),1)] {put(0x7e0000+addr+x,val,width);}}
    for slot in 0..4 {for field in 0..6 {put(0x7e2e78+field as u32*8+slot as u32*2,g.graphics[slot][field],2);}}
    result
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let props=CommandProperties::load(root)?;let table=RngTable::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,combined,before,after,effects\n");
    for case in 0..2048usize {
        let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let mut s=Scheduler::default();let mut commands=CommandLists::default();let mut a=[BattleActor::default();10];let mut extra=[Actor::default();10];let mut bg=BattleGlobals::default();let mut g=Globals::default();
        bg.b1=next();bg.b8=u16::from_le_bytes([next(),next()]);bg.attacker_level=next();bg.jumping=if case%4==0 {0}else{u16::from_le_bytes([next(),next()])};bg.quick_target=if case%3==0 {(case%10*2) as u8}else{255};bg.untargetable=if case%5==0 {0}else{u16::from_le_bytes([next(),next()])};bg.used_espers=next();bg.morph.saved=next();bg.morph.story_flags=next();bg.speed_factor=next();
        g.skip_targets=if case%4==0 {0}else{u16::from_le_bytes([next(),next()])};g.dead_monsters=next();g.reraise_exclusion=next();g.controlled_flip=next();g.end_event=next();g.quick_counter=next();if case%5==0 {g.zinger_target=(case%10*2) as u8;g.zinger_attacker=((case+4)%10*2) as u8;}
        for slot in 0..10 {let b=&mut s.actors[slot];let actor=&mut a[slot];let e=&mut extra[slot];b.flags0=if case<256 && slot!=case%10 {next()&254}else{next()|1};b.flags1=next();b.status1=if case<256 {case as u8}else{next()};b.update_flags=next();b.extra_status=next();b.gauge.counter=u16::from_le_bytes([next(),next()]);b.gauge.increment=u16::from_le_bytes([next(),next()]);b.gauge.advance_counter=u16::from_le_bytes([next(),next()]);b.gauge.advance_duration=next();actor.status2=next();actor.status3=next();actor.status4=next();actor.update_high=next();actor.previous_attacker=next();actor.property=if case%7==0 {14}else{0};actor.relic4=next();actor.relic5=next();actor.speed=next();actor.mp=u16::from_le_bytes([next(),next()]);actor.commands=std::array::from_fn(|i|[if i<2 {3+i as u8}else{next()%32},next(),next()]);actor.spells=std::array::from_fn(|i|[if i%7==0 {35}else{next()},next(),next(),next()]);e.hp=u16::from_le_bytes([next(),next()]);e.max_hp=u16::from_le_bytes([next(),next()]);e.max_mp=u16::from_le_bytes([next(),next()]);}
        // Consistent paired relations, sometimes already marked for release.
        for (controller,target) in [(0,6),(1,7),(2,8),(3,9)] {if case/(controller+1)%3!=0 {let marked=if case%3==0 {128}else{0};extra[controller].control_target=(target*2) as u8|marked;a[target].control=(controller*2) as u8|marked;a[controller].status4|=16;s.actors[controller].extra_status|=1;}}
        for (source,target) in [(0,5),(2,7),(4,9)] {if case%3!=0 {extra[source].love_target=(target*2) as u8;extra[target].love_attacker=(source*2) as u8;}if case%5!=0 {extra[source].charm_target=(target*2) as u8;a[target].charm=(source*2) as u8;}}
        for ring in [&mut s.queues.advance,&mut s.queues.action] {ring.head=[0,250,255,20][case/3%4];ring.tail=[0,3,255,200][case/7%4];ring.bytes=std::array::from_fn(|i|if i%5==0 {255}else{(i%10*2) as u8});}
        for slot in 0..10 {for n in 0..case%3 {commands.normal(slot,Payload {command:0,attack:n as u8,targets:1,mp_cost:0})?;}}
        for n in 0..case%3 {commands.immediate(Payload {command:35,attack:n as u8,targets:0x102,mp_cost:0},&mut s.queues)?;}
        let mut rng=BattleRng {index:next()};let before=snapshot(&s,&commands,&a,&bg,&extra,&g,&rng);run(&mut s,&mut commands,&mut a,&mut bg,&mut extra,&mut g)?;let combined=case%2==1;
        let effects=if combined {battle_after_action::run(&mut s,&mut commands,&mut a,&mut bg,&mut rng,&table,&props)?}else{Vec::new()};let effect=effects.iter().map(|e|match e {crate::battle_readiness::Effect::OpenMenu(arg)=>format!("open:{arg}"),crate::battle_readiness::Effect::CloseMenu(arg)=>format!("close:{arg}")}).collect::<Vec<_>>().join("|");
        writeln!(rows,"{case},{},{before},{},{effect}",u8::from(combined),snapshot(&s,&commands,&a,&bg,&extra,&g,&rng)).unwrap();
    }
    fs::write(dir.join("special-status.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn jumping_banon_death_sets_event_before_fixing_dead_and_keeps_hp_zero() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let mut bg=BattleGlobals::default();let mut e=[Actor::default();10];let mut g=Globals::default();s.actors[0].flags0=1;s.actors[0].status1=128;s.actors[0].update_flags=255;a[0].property=14;a[0].update_high=255;e[0].hp=100;bg.jumping=1;
        run(&mut s,&mut c,&mut a,&mut bg,&mut e,&mut g).unwrap();assert_eq!((g.end_event,e[0].hp,s.actors[0].status1,s.actors[0].update_flags,a[0].update_high),(6,0,0,191,251));assert_eq!(g.graphics[0][0],0);
    }
    #[test]fn reraise_enqueues_raise_without_fabricating_restored_hp() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let mut bg=BattleGlobals::default();let mut e=[Actor::default();10];let mut g=Globals::default();s.actors[6].flags0=1;s.actors[6].status1=128;a[6].status4=4;g.reraise_exclusion=255;bg.b8=0xabcd;
        run(&mut s,&mut c,&mut a,&mut bg,&mut e,&mut g).unwrap();assert_eq!((a[6].status4,s.actors[6].status1,e[6].hp,g.reraise_exclusion),(0,128,0,251));assert_eq!(c.payloads[s.queues.immediate.unwrap() as usize],Payload {command:38,attack:12,targets:0xab30,mp_cost:0});
    }
    #[test]fn absent_controller_still_releases_marked_link_and_updates_both_actors() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let mut bg=BattleGlobals::default();let mut e=[Actor::default();10];let mut g=Globals::default();e[0].control_target=140;a[6].control=128;a[0].status4=16;s.actors[0].extra_status=1;g.controlled_flip=255;
        run(&mut s,&mut c,&mut a,&mut bg,&mut e,&mut g).unwrap();assert_eq!((e[0].control_target,a[6].control,a[0].status4,s.actors[0].extra_status,g.controlled_flip),(255,255,0,0,251));for slot in [0,6] {assert_eq!((s.actors[slot].flags1,s.actors[slot].update_flags),(64,64));}
    }
    #[test]fn petrify_cancels_quick_and_zinger_without_zeroing_hp() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let mut bg=BattleGlobals::default();let mut e=[Actor::default();10];let mut g=Globals::default();s.actors[0].flags0=1;s.actors[0].status1=64;e[0].hp=100;bg.quick_target=0;bg.b1=0x41;g.zinger_target=0;g.zinger_attacker=12;
        run(&mut s,&mut c,&mut a,&mut bg,&mut e,&mut g).unwrap();assert_eq!((e[0].hp,bg.quick_target,g.quick_counter,g.zinger_attacker,g.zinger_target,bg.b1),(100,255,255,255,255,1));let head=s.queues.immediate.unwrap() as usize;let next=c.links[head] as usize;
        assert_eq!(c.payloads[head],Payload {command:36,attack:0,targets:0x0404,mp_cost:0});assert_eq!(c.payloads[next],Payload {command:36,attack:8,targets:0x0402,mp_cost:0});
    }
    #[test]fn released_control_handoff_removes_normal_actions_and_disables_atb() {
        let root=Path::new("extracted/twue-3.07");let props=CommandProperties::load(root).unwrap();let table=RngTable::load(root).unwrap();let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let mut bg=BattleGlobals::default();let mut e=[Actor::default();10];let mut g=Globals::default();
        for slot in [0,6] {s.actors[slot].flags0=129;s.actors[slot].gauge.counter=0xabcd;c.normal(slot,Payload {command:0,attack:239,targets:1,mp_cost:0}).unwrap();}s.actors[0].status1=64;e[0].control_target=12;a[6].control=0;
        run(&mut s,&mut c,&mut a,&mut bg,&mut e,&mut g).unwrap();assert_eq!((e[0].control_target,a[6].control),(255,255));assert_ne!(c.normal[6],255);
        battle_after_action::run(&mut s,&mut c,&mut a,&mut bg,&mut BattleRng::default(),&table,&props).unwrap();for slot in [0,6] {assert_eq!((c.normal[slot],s.actors[slot].gauge.counter,s.actors[slot].update_flags),(255,0x01cd,0));}assert!(s.queues.immediate.is_some());
    }
}
