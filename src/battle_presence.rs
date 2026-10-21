//! C2/1429..14AC: monster-death gating and present/absent reconciliation.
use crate::{battle_after_action::{self,Actor as BattleActor,Globals as BattleGlobals},battle_special_status::{self,Actor as SpecialActor,Globals as SpecialGlobals},battle_commands::{CommandLists,Payload},battle_scheduler::Scheduler,battle_targets::actor_mask,battle_cleanup::{self,Living},battle_rng::{BattleRng,RngTable},battle_action_init::CommandProperties};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default)]pub struct Actor {pub retaliation_flags:u8,pub monster_status:u8}
pub fn check_dead(a:&[BattleActor;10],meta:&[Actor;10],c:&CommandLists,g:&mut SpecialGlobals) {
    for slot in (4..10).rev() {let mask=(actor_mask(slot)>>8) as u8;
        if g.dead_monsters&mask!=0 && (a[slot].status4&32!=0 || meta[slot].retaliation_flags&128!=0 || c.retaliation[slot]==255) {g.reraise_exclusion&=!mask;}
    }
}
pub fn present(s:&mut Scheduler,a:&mut[BattleActor;10],meta:&[Actor;10],g:&mut SpecialGlobals) {
    for slot in (0..10).rev() {let mask=actor_mask(slot);if g.skip_targets&mask==0 {continue;}g.skip_targets&=!mask;g.reraise_exclusion|=(mask>>8) as u8;s.actors[slot].flags0|=1;s.actors[slot].status1&=0x1d;a[slot].status4&=0xdf;
        if meta[slot].monster_status&2!=0 {s.actors[slot].flags0|=8;s.actors[slot].gauge.counter&=255;s.actors[slot].gauge.advance_counter|=0xff00;s.enqueue_advance(slot);}
    }
}
pub fn absent(s:&mut Scheduler,c:&mut CommandLists,a:&mut[BattleActor;10],bg:&mut BattleGlobals,extra:&mut[SpecialActor;10],g:&mut SpecialGlobals)->Result<(),String> {
    for slot in (0..10).rev() {let mask=actor_mask(slot);if bg.untargetable&mask==0 {continue;}bg.untargetable&=!mask;g.reraise_exclusion&=!((mask>>8) as u8);s.actors[slot].flags0&=254;a[slot].status4|=32;battle_special_status::cleanup_relations(slot,s,c,a,bg,extra,g)?;}
    Ok(())
}
pub fn update_living(s:&Scheduler,bg:&BattleGlobals,g:&mut SpecialGlobals,living:&mut Living) {
    living.untargetable=bg.untargetable;living.override_targets=g.skip_targets;living.dead_monsters=g.dead_monsters;
    battle_cleanup::update_living(&std::array::from_fn(|slot|battle_cleanup::Actor {present:s.actors[slot].flags0&1!=0,status1:s.actors[slot].status1}),living);g.skip_targets=living.override_targets;
}
fn snapshot(s:&Scheduler,c:&CommandLists,a:&[BattleActor;10],bg:&BattleGlobals,extra:&[SpecialActor;10],g:&SpecialGlobals,meta:&[Actor;10],living:&Living,rng:&BattleRng)->String {
    use std::fmt::Write;let mut result=battle_special_status::snapshot(s,c,a,bg,extra,g,rng);let mut put=|addr:u32,val:u16,width:usize| {for i in 0..width {write!(result,"{:06X}{:02X}",addr+i as u32,(val>>(i*8)) as u8).unwrap();}};
    for (slot,m) in meta.iter().enumerate() {put(0x7e3e4c+slot as u32*2,u16::from(m.retaliation_flags),1);put(0x7e3c80+slot as u32*2,u16::from(m.monster_status),1);}
    for (addr,val,width) in [(0x3408,living.revived,2),(0x3a40,u16::from(living.enemy_characters),1),(0x3a78,living.valid,2),(0x3a74,living.alive,2),(0x3a42,living.enemy_alive,2),(0x3a76,u16::from(living.allies),1),(0x3a77,u16::from(living.enemies),1)] {put(0x7e0000+addr,val,width);}
    result
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let props=CommandProperties::load(root)?;let table=RngTable::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,mode,before,after,effects\n");
    for case in 0..4096usize {
        let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let mut extra=[SpecialActor::default();10];let mut bg=BattleGlobals::default();let mut g=SpecialGlobals::default();let mut meta=[Actor::default();10];
        bg.b1=next();bg.b8=u16::from_le_bytes([next(),next()]);bg.untargetable=u16::from_le_bytes([next(),next()]);bg.quick_target=if case%3==0 {(case%10*2) as u8}else{255};bg.attacker_level=next();bg.morph.saved=next();bg.morph.story_flags=next();g.skip_targets=u16::from_le_bytes([next(),next()]);g.dead_monsters=next();g.reraise_exclusion=next();g.controlled_flip=next();g.quick_counter=next();if case%5==0 {g.zinger_target=(case%10*2) as u8;g.zinger_attacker=((case+4)%10*2) as u8;}
        let mut living=Living {untargetable:0,override_targets:0,valid:u16::from_le_bytes([next(),next()]),alive:u16::from_le_bytes([next(),next()]),enemy_alive:u16::from_le_bytes([next(),next()]),revived:u16::from_le_bytes([next(),next()]),enemy_characters:next(),dead_monsters:0,allies:next(),enemies:next()};
        for slot in 0..10 {let b=&mut s.actors[slot];let actor=&mut a[slot];b.flags0=next();b.flags1=next();b.status1=next();b.update_flags=next();b.extra_status=next();b.gauge.counter=u16::from_le_bytes([next(),next()]);b.gauge.advance_counter=u16::from_le_bytes([next(),next()]);actor.status2=next();actor.status3=next();actor.status4=next();actor.update_high=next();actor.relic4=next();actor.relic5=next();actor.speed=next();actor.mp=u16::from_le_bytes([next(),next()]);actor.commands=std::array::from_fn(|i|[if i<2 {3+i as u8}else{next()%32},next(),next()]);actor.spells=std::array::from_fn(|i|[if i%7==0 {35}else{next()},next(),next(),next()]);meta[slot]=Actor {retaliation_flags:next(),monster_status:next()};}
        for (source,target) in [(0,5),(2,7),(4,9)] {if case%3!=0 {extra[source].love_target=(target*2) as u8;extra[target].love_attacker=(source*2) as u8;}if case%5!=0 {extra[source].charm_target=(target*2) as u8;a[target].charm=(source*2) as u8;}}
        for ring in [&mut s.queues.advance,&mut s.queues.action] {ring.head=[0,250,255,20][case/3%4];ring.tail=[0,3,255,200][case/7%4];ring.bytes=std::array::from_fn(|i|if i%5==0 {255}else{(i%10*2) as u8});}
        for slot in 0..10 {for n in 0..case%3 {c.normal(slot,Payload {command:0,attack:n as u8,targets:1,mp_cost:0})?;}if (slot+case)%3==0 {c.retaliation(slot,Payload {command:31,attack:238,targets:1,mp_cost:0},&mut s.queues)?;}}
        let mut rng=BattleRng {index:next()};let mode=case%4;let before=snapshot(&s,&c,&a,&bg,&extra,&g,&meta,&living,&rng);let mut effects=Vec::new();
        match mode {0=>check_dead(&a,&meta,&c,&mut g),1=>present(&mut s,&mut a,&meta,&mut g),2=>absent(&mut s,&mut c,&mut a,&mut bg,&mut extra,&mut g)?,3=>{check_dead(&a,&meta,&c,&mut g);update_living(&s,&bg,&mut g,&mut living);present(&mut s,&mut a,&meta,&mut g);effects=battle_after_action::run(&mut s,&mut c,&mut a,&mut bg,&mut rng,&table,&props)?;absent(&mut s,&mut c,&mut a,&mut bg,&mut extra,&mut g)?;},_=>unreachable!()}
        let effect=effects.iter().map(|e|match e {crate::battle_readiness::Effect::OpenMenu(arg)=>format!("open:{arg}"),crate::battle_readiness::Effect::CloseMenu(arg)=>format!("close:{arg}")}).collect::<Vec<_>>().join("|");
        writeln!(rows,"{case},{mode},{before},{},{effect}",snapshot(&s,&c,&a,&bg,&extra,&g,&meta,&living,&rng)).unwrap();
    }
    fs::write(dir.join("presence.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn dead_monster_keeps_alive_bit_until_retaliation_finishes_unless_hidden() {
        let mut a=[BattleActor::default();10];let meta=[Actor::default();10];let mut c=CommandLists::default();let mut g=SpecialGlobals::default();g.dead_monsters=4;g.reraise_exclusion=255;c.retaliation[6]=127;check_dead(&a,&meta,&c,&mut g);assert_eq!(g.reraise_exclusion,255);a[6].status4=32;check_dead(&a,&meta,&c,&mut g);assert_eq!(g.reraise_exclusion,251);
    }
    #[test]fn first_strike_preserves_low_gauges_and_enqueues_descending() {
        let mut s=Scheduler::default();let mut a=[BattleActor::default();10];let mut meta=[Actor::default();10];let mut g=SpecialGlobals::default();g.skip_targets=0x501;
        for slot in [0,4,6] {meta[slot].monster_status=2;s.actors[slot].flags1=1;s.actors[slot].status1=255;s.actors[slot].gauge.counter=0xabcd;s.actors[slot].gauge.advance_counter=0x1234;a[slot].status4=255;}
        present(&mut s,&mut a,&meta,&mut g);assert_eq!(g.skip_targets,0);for slot in [0,4,6] {assert_eq!((s.actors[slot].gauge.counter,s.actors[slot].gauge.advance_counter,s.actors[slot].status1,a[slot].status4,s.actors[slot].flags1),(0xcd,0xff34,0x1d,0xdf,0));}assert_eq!((s.queues.advance.pop(),s.queues.advance.pop(),s.queues.advance.pop()),(Some(12),Some(8),Some(0)));
    }
    #[test]fn disappearing_target_hides_without_erasing_hp_and_cancels_quick() {
        let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[BattleActor::default();10];let mut bg=BattleGlobals::default();let mut e=[SpecialActor::default();10];let mut g=SpecialGlobals::default();s.actors[6].flags0=255;e[6].hp=123;bg.untargetable=1024;bg.quick_target=12;g.reraise_exclusion=255;
        absent(&mut s,&mut c,&mut a,&mut bg,&mut e,&mut g).unwrap();assert_eq!((s.actors[6].flags0,a[6].status4,e[6].hp,bg.untargetable,bg.quick_target,g.quick_counter,g.reraise_exclusion),(254,32,123,0,255,255,251));
    }
}
