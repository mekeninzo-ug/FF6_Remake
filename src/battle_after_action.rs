//! Complete AfterAction2 C2/083F..08C5; effects are returned for presentation.
use crate::{battle_action_init::CommandProperties,battle_commands::CommandLists,battle_scheduler::Scheduler,battle_rng::{BattleRng,RngTable},battle_readiness::{self,Mode,Effect},battle_menu_state::{self,CommandContext},battle_morph,battle_after_effects,battle_cleanup,battle_time};
use std::{fs,path::Path,collections::BTreeMap};
#[derive(Clone,Copy,Debug)]
pub struct Actor {
    pub previous_attacker:u8,pub update_high:u8,pub status2:u8,pub status3:u8,pub status4:u8,
    pub property:u8,pub ai_high:u8,pub control:u8,pub seize:u8,pub charm:u8,
    pub right_flags:u8,pub left_flags:u8,pub relic4:u8,pub relic5:u8,pub speed:u8,
    pub status_constant:u8,pub mp:u16,pub condemned:u8,pub morph_gauge:u8,
    pub commands:[[u8;3];4],pub spells:[[u8;4];79],
}
impl Default for Actor {fn default()->Self {Self {previous_attacker:255,update_high:128,status2:0,status3:0,status4:0,property:0,ai_high:255,control:255,seize:255,charm:255,right_flags:0,left_flags:0,relic4:0,relic5:0,speed:0,status_constant:64,mp:0,condemned:0,morph_gauge:0,commands:[[255,0,0];4],spells:[[255,0,0,0];79]}}}
#[derive(Clone,Copy,Debug)]
pub struct Globals {
    pub quick_target:u8,pub colosseum:u8,pub untargetable:u16,pub b1:u8,pub b8:u16,
    pub jumping:u16,pub attacker_level:u8,pub speed_factor:u8,pub used_espers:u8,pub max_cost:u8,
    pub morph:battle_morph::State,
}
impl Default for Globals {fn default()->Self {Self {quick_target:255,colosseum:0,untargetable:0,b1:0,b8:0,jumping:0,attacker_level:1,speed_factor:255,used_espers:0,max_cost:0,morph:battle_morph::State {actor:255,counter:0,rate:0,saved:0,story_flags:0,changed:0}}}}
fn command_context(slot:usize,s:&Scheduler,a:&Actor,g:&Globals)->CommandContext {CommandContext {slot,status1:s.actors[slot].status1,status2:a.status2,right_flags:a.right_flags,left_flags:a.left_flags,relic4:a.relic4,morph_counter:g.morph.saved}}
fn readiness(slot:usize,a:&Actor,g:&Globals)->battle_readiness::Context {battle_readiness::Context {slot,quick_target:g.quick_target,colosseum:g.colosseum,untargetable:g.untargetable,flags_b1:g.b1,property:a.property,ai_high:a.ai_high,control:a.control,seize:a.seize,charm:a.charm,status2:a.status2,status3:a.status3,status4:a.status4,update_high:a.update_high}}
pub fn run(s:&mut Scheduler,commands:&mut CommandLists,actors:&mut[Actor;10],g:&mut Globals,rng:&mut BattleRng,table:&RngTable,properties:&CommandProperties)->Result<Vec<Effect>,String> {
    let mut effects=Vec::new();
    for slot in (0..10).rev() {
        if s.actors[slot].flags0&1==0 {continue;}
        let a=&mut actors[slot];a.previous_attacker&=127;
        if s.actors[slot].status1&128!=0 || s.actors[slot].flags1&64!=0 {battle_readiness::run(Mode::Disable,s,commands,readiness(slot,a,g),&mut effects)?;}
        if s.actors[slot].update_flags!=0 {
            // Each memory ASL happens immediately before its handler. The final
            // ASL also consumes bit 0, after Quetzalli inspected it first.
            if s.actors[slot].update_flags&1!=0 {battle_after_effects::quetzalli(slot,s,&mut g.jumping,&mut a.update_high)?;}
            for bit in (1..=7).rev() {
                let selected=s.actors[slot].update_flags&128!=0;s.actors[slot].update_flags<<=1;
                if !selected {continue;}
                match bit {
                    7=>battle_menu_state::update_spells(slot,a.mp,s.actors[slot].status1,g.used_espers,&mut a.spells,&mut g.max_cost),
                    6=>{battle_cleanup::remove_all(slot,s,commands,g.b8,&mut g.b1)?;s.actors[slot].flags1|=128;},
                    5=>a.condemned=battle_after_effects::start_condemn(g.attacker_level,rng,table),
                    4=>a.condemned=0,
                    3=>if slot<4 {battle_menu_state::update_commands(command_context(slot,s,a,g),properties,&mut a.commands,&mut g.morph.changed)?;},
                    2=>{a.status_constant=if a.status3&4!=0 {32}else if a.status3&8!=0 {84}else{64};s.actors[slot].gauge.increment=battle_time::gauge_increment_with_factor(a.speed,a.status3,slot>=4,g.speed_factor);},
                    1=>{let c=battle_morph::Context {slot,b1:g.b1,gauge:s.actors[slot].gauge.counter,status4:a.status4,command:command_context(slot,s,a,g)};battle_morph::update(c,&mut g.morph,&mut s.actors[slot].flags0,&mut a.morph_gauge,&mut a.commands,properties)?;},
                    _=>unreachable!(),
                }
            }
            s.actors[slot].update_flags<<=1;
        }
        battle_readiness::run(Mode::Player,s,commands,readiness(slot,a,g),&mut effects)?;
        battle_readiness::run(Mode::Pending,s,commands,readiness(slot,a,g),&mut effects)?;
        if s.actors[slot].flags0&0x50!=0 {s.actors[slot].flags1&=0xfd;}
        battle_after_effects::near_fatal(slot,s,commands,a.status2,a.relic5,&mut a.update_high,&mut g.b8,&mut g.b1)?;
    }
    Ok(effects)
}
// Sparse RAM snapshots are comparison fixtures, not the game's state model.
fn snapshot(s:&Scheduler,c:&CommandLists,actors:&[Actor;10],g:&Globals,rng:&BattleRng)->String {
    let mut m=BTreeMap::<u32,u8>::new();let mut put=|address:u32,value:u16,width:usize|{for i in 0..width {m.insert(address+i as u32,(value>>(i*8)) as u8);}};
    for (addr,val,width) in [(0xb1,u16::from(g.b1),1),(0xb8,g.b8,2),(0xbe,u16::from(rng.index),1),(0x7e3404,u16::from(g.quick_target),1),(0x7e3a97,u16::from(g.colosseum),1),(0x7e2f4c,g.untargetable,2),(0x7e3f2c,g.jumping,2),(0x7e11af,u16::from(g.attacker_level),1),(0x7e3a90,u16::from(g.speed_factor),1),(0x7e3f2e,u16::from(g.used_espers),1),(0x7e3a4c,u16::from(g.max_cost),1),(0x7e3ee2,u16::from(g.morph.actor),1),(0x7e3f30,g.morph.counter,2),(0x7e3f32,g.morph.rate,2),(0x7e1cf6,u16::from(g.morph.saved),1),(0x7e3ebb,u16::from(g.morph.story_flags),1),(0x7e3a58,g.morph.changed,2),(0x7e340a,u16::from(s.queues.immediate.unwrap_or(255)),1)] {put(addr,val,width);}
    for (slot,a) in actors.iter().enumerate() {let x=slot as u32*2;let b=s.actors[slot];
        for (addr,val,width) in [(0x3aa0,u16::from(b.flags0),1),(0x3aa1,u16::from(b.flags1),1),(0x3ee4,u16::from(b.status1),1),(0x3ee5,u16::from(a.status2),1),(0x3ef8,u16::from(a.status3),1),(0x3ef9,u16::from(a.status4),1),(0x3204,u16::from(b.update_flags),1),(0x3205,u16::from(a.update_high),1),(0x3e4d,u16::from(b.extra_status),1),(0x32e0,u16::from(a.previous_attacker),1),(0x3218,b.gauge.counter,2),(0x3ac8,b.gauge.increment,2),(0x3ab4,b.gauge.advance_counter,2),(0x322c,u16::from(b.gauge.advance_duration),1),(0x3255,u16::from(a.ai_high),1),(0x32b9,u16::from(a.control),1),(0x3359,u16::from(a.seize),1),(0x3395,u16::from(a.charm),1),(0x3ba4,u16::from(a.right_flags),1),(0x3ba5,u16::from(a.left_flags),1),(0x3c58,u16::from(a.relic4),1),(0x3c59,u16::from(a.relic5),1),(0x3b19,u16::from(a.speed),1),(0x3add,u16::from(a.status_constant),1),(0x3c08,a.mp,2),(0x3b05,u16::from(a.condemned),1),(0x3b04,u16::from(a.morph_gauge),1),(0x32cc,u16::from(c.normal[slot]),1),(0x32cd,u16::from(c.retaliation[slot]),1)] {put(0x7e0000+addr+x,val,width);}
        if slot<4 {put(0x7e3ed8+x,u16::from(a.property),1);}
        put(0x7e3018+x,crate::battle_targets::actor_mask(slot),2);
        for (i,v) in a.commands.concat().iter().enumerate() {put(0x7e202e+slot as u32*12+i as u32,u16::from(*v),1);}
        if slot<4 {let base=0x5000+slot as u32*320;put(0x7e302c+x,base as u16,2);for (i,v) in a.spells.concat().iter().enumerate() {put(0x7e0000+base+i as u32,u16::from(*v),1);}}
    }
    for i in 0..128 {put(0x7e3184+i as u32,u16::from(c.links[i]),1);let p=c.payloads[i];put(0x7e3420+i as u32*2,u16::from(p.command)|(u16::from(p.attack)<<8),2);put(0x7e3520+i as u32*2,p.targets,2);put(0x7e3620+i as u32*2,u16::from(p.mp_cost),1);}
    for (base,ring) in [(0x7e3720,&s.queues.advance),(0x7e3820,&s.queues.action)] {put(if base==0x7e3720 {0x7e3a64}else{0x7e3a66},u16::from(ring.head)|(u16::from(ring.tail)<<8),2);for (i,v) in ring.bytes.iter().enumerate() {put(base+i as u32,u16::from(*v),1);}}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let props=CommandProperties::load(root)?;let table=RngTable::load(root)?;let mut seed=0x6ff6u32;
    let mut rows=String::from("case,before,after,effects\n");
    for case in 0..1024usize {
        let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let mut s=Scheduler::default();let mut commands=CommandLists::default();let mut actors=[Actor::default();10];let mut g=Globals::default();
        g.b1=next();g.b8=u16::from_le_bytes([next(),next()]);g.attacker_level=next();g.speed_factor=next();g.used_espers=next();g.quick_target=if case%3==0 {(case%10*2) as u8}else{255};g.untargetable=if case%5==0 {0}else{u16::from_le_bytes([next(),next()])};g.colosseum=if case%19==0 {1}else{0};g.jumping=u16::from_le_bytes([next(),next()]);g.max_cost=next();g.morph=battle_morph::State {actor:if case%3==0 {0}else{255},counter:u16::from_le_bytes([next(),next()]),rate:u16::from_le_bytes([next(),next()]),saved:next(),story_flags:next(),changed:u16::from_le_bytes([next(),next()])};
        for slot in 0..10 {
            let a=&mut actors[slot];let b=&mut s.actors[slot];b.flags0=if case<256 && slot!=case%10 {next()&0xfe}else{next()|1};b.flags1=next();b.status1=if case%5==0 {0}else{next()};b.extra_status=next();b.update_flags=if case<256 {case as u8}else{next()};b.gauge.counter=if case%4==0 {u16::from(next())}else{u16::from_le_bytes([next(),next()])};b.gauge.increment=u16::from_le_bytes([next(),next()]);b.gauge.advance_counter=u16::from_le_bytes([next(),next()]);b.gauge.advance_duration=next();
            a.previous_attacker=next();a.update_high=next();a.status2=if case%5==0 {2}else{next()};a.status3=if case%5==0 {0}else{next()};a.status4=if case%5==0 {0}else{next()};a.property=if case%11==0 {13}else{0};a.ai_high=if case%7==0 {0}else{255};a.control=if case%13==0 {0}else{255};a.seize=if case%17==0 {0}else{255};a.charm=255;a.right_flags=next();a.left_flags=next();a.relic4=next();a.relic5=next();a.speed=next();a.status_constant=next();a.mp=u16::from_le_bytes([next(),next()]);a.condemned=next();a.morph_gauge=next();a.commands=std::array::from_fn(|i|[if i<2 {3+i as u8}else{next()%32},next(),next()]);a.spells=std::array::from_fn(|i|[if i%7==0 {35}else{next()},next(),next(),next()]);
            for n in 0..case%3 {commands.normal(slot,crate::battle_commands::Payload {command:0,attack:n as u8,targets:1,mp_cost:0})?;}
        }
        for ring in [&mut s.queues.advance,&mut s.queues.action] {ring.head=[0,250,255,20][case/3%4];ring.tail=[0,3,255,200][case/7%4];ring.bytes=std::array::from_fn(|i|if i%5==0 {255}else{(i%10*2) as u8});}
        for n in 0..case%3 {commands.immediate(crate::battle_commands::Payload {command:35,attack:n as u8,targets:0x102,mp_cost:0},&mut s.queues)?;}
        let mut rng=BattleRng {index:next()};let before=snapshot(&s,&commands,&actors,&g,&rng);let effects=run(&mut s,&mut commands,&mut actors,&mut g,&mut rng,&table,&props)?;let effect=effects.iter().map(|e|match e {Effect::OpenMenu(arg)=>format!("open:{arg}"),Effect::CloseMenu(arg)=>format!("close:{arg}")}).collect::<Vec<_>>().join("|");
        writeln!(rows,"{case},{before},{},{effect}",snapshot(&s,&commands,&actors,&g,&rng)).unwrap();
    }
    fs::write(dir.join("after-action.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn combined_updates_obey_stop_after_start_and_consume_all_bits() {
        let root=Path::new("extracted/twue-3.07");let props=CommandProperties::load(root).unwrap();let table=RngTable::load(root).unwrap();let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[Actor::default();10];let mut g=Globals::default();let mut rng=BattleRng::default();s.actors[6].flags0=1;s.actors[6].update_flags=0x30;a[6].condemned=99;a[6].previous_attacker=255;
        run(&mut s,&mut c,&mut a,&mut g,&mut rng,&table,&props).unwrap();assert_eq!((a[6].condemned,a[6].previous_attacker,s.actors[6].update_flags,rng.index),(0,127,0,1));
    }
    #[test]fn absent_actors_are_untouched_and_do_not_consume_rng() {
        let root=Path::new("extracted/twue-3.07");let props=CommandProperties::load(root).unwrap();let table=RngTable::load(root).unwrap();let mut s=Scheduler::default();let mut c=CommandLists::default();let mut a=[Actor::default();10];let mut g=Globals::default();let mut rng=BattleRng::default();s.actors[6].update_flags=255;
        assert!(run(&mut s,&mut c,&mut a,&mut g,&mut rng,&table,&props).unwrap().is_empty());assert_eq!((a[6].previous_attacker,s.actors[6].update_flags,rng.index),(255,255,0));
    }
}
