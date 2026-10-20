//! UpdateDead C2/4AB9..4B52 and RemoveAllActions C2/0A0F..0A40.
use crate::{battle_commands::{CommandLists,Payload},battle_scheduler::Scheduler,battle_targets::actor_mask};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default)]
pub struct Actor {pub present:bool,pub status1:u8}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Living {
    pub untargetable:u16,pub override_targets:u16,pub valid:u16,pub alive:u16,pub enemy_alive:u16,
    pub revived:u16,pub enemy_characters:u8,pub dead_monsters:u8,pub allies:u8,pub enemies:u8,
}
pub fn update_living(actors:&[Actor;10],s:&mut Living) {
    s.override_targets&=!s.untargetable;s.valid=s.override_targets;s.alive=0;s.enemy_alive=0;
    for slot in (0..10).rev() {
        let mask=actor_mask(slot);let a=actors[slot];if s.untargetable&mask!=0 {continue;}
        if s.override_targets&mask==0 {
            if !a.present {continue;}
            s.valid|=mask;
            if (slot>=4 && s.dead_monsters&(mask>>8) as u8!=0) || a.status1&0xc2!=0 {continue;}
        }
        let living=mask&s.revived;s.alive|=living;
        if slot<4 {let enemy=living&u16::from(s.enemy_characters);s.enemy_alive|=enemy;s.alive&=!enemy;}
    }
    s.allies=(s.alive as u8).count_ones() as u8;s.enemies=((s.alive>>8) as u8).count_ones() as u8+s.enemy_alive.count_ones() as u8;
}
pub fn remove_all(slot:usize,scheduler:&mut Scheduler,commands:&mut CommandLists,working_targets:u16,mp_flags:&mut u8)->Result<(),String> {
    if slot>=10 {return Err("Action cleanup actor outside ten slots".into());}
    for _ in 0..128 {if commands.normal[slot]&128!=0 {break;}commands.remove_normal(slot)?;}
    if commands.normal[slot]&128==0 {return Err("Action cleanup command chain did not terminate".into());}
    // Source uses an unsigned CPY/BCC do-while, not a normalized ring iterator.
    let actor=(slot*2) as u8;let ring=&mut scheduler.queues.advance;let mut index=ring.head;
    loop {if ring.bytes[usize::from(index)]==actor {ring.bytes[usize::from(index)]=255;}index=index.wrapping_add(1);if index>=ring.tail {break;}}
    if scheduler.actors[slot].gauge.counter>>8==0 {scheduler.actors[slot].gauge.counter|=0xff00;scheduler.actors[slot].flags0&=0xd3;}
    if slot<4 {
        *mp_flags&=!0x40;
        commands.immediate(Payload {command:44,attack:actor,targets:working_targets,mp_cost:0},&mut scheduler.queues)?;
    }
    Ok(())
}
fn payload_bytes(commands:&CommandLists)->Vec<u8> {
    let mut b=Vec::new();for p in commands.payloads {b.extend([p.command,p.attack]);b.extend(p.targets.to_le_bytes());b.push(p.mp_cost);}b
}
pub fn capture(directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let mut seed=0x6ff6u32;
    let mut rows=String::from("case,actors,untargetable,override,revived,enemy,dead,override_out,valid,alive,enemy_alive,allies,enemies\n");
    for case in 0..16384 {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u16};
        let actors=std::array::from_fn(|_|Actor {present:next()&1!=0,status1:next() as u8});
        let mut s=Living {untargetable:next(),override_targets:next(),valid:next(),alive:next(),enemy_alive:next(),revived:next(),enemy_characters:next() as u8,dead_monsters:next() as u8,allies:0,enemies:0};let before=s;update_living(&actors,&mut s);
        let actor_hex=actors.iter().map(|a|format!("{:02X}{:02X}",a.present as u8,a.status1)).collect::<String>();
        writeln!(rows,"{case},{actor_hex},{},{},{},{},{},{},{},{},{},{},{}",before.untargetable,before.override_targets,before.revived,before.enemy_characters,before.dead_monsters,s.override_targets,s.valid,s.alive,s.enemy_alive,s.allies,s.enemies).unwrap();
    }
    fs::write(directory.join("living.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,slot,flags,gauge,head,tail,queue,links,normal,retal,immediate,payloads,mp_flags,targets,flags_out,gauge_out,queue_out,links_out,normal_out,retal_out,immediate_out,payloads_out,mp_flags_out\n");
    for case in 0..1024usize {
        let slot=case%10;let mut scheduler=Scheduler::default();let mut commands=CommandLists::default();
        for actor in [slot,(slot+1)%10] {for n in 0..case%4 {commands.normal(actor,Payload {command:2,attack:n as u8,targets:0x103,mp_cost:17})?;}commands.retaliation(actor,Payload {command:0,attack:239,targets:1,mp_cost:0},&mut scheduler.queues)?;}
        if case%3==0 {commands.immediate(Payload {command:35,attack:1,targets:0x100,mp_cost:0},&mut scheduler.queues)?;}
        let ring=&mut scheduler.queues.advance;ring.head=[0,250,255,20][case/3%4];ring.tail=[0,3,255,200][case/7%4];
        ring.bytes=std::array::from_fn(|i|if i%3==0 {(slot*2) as u8}else{((slot+1)%10*2) as u8});
        let flags=(case*17) as u8;let gauge=if case%3==0 {(case%256) as u16}else{(case as u16).wrapping_mul(139)};scheduler.actors[slot].flags0=flags;scheduler.actors[slot].gauge.counter=gauge;
        let mut mp_flags=if case%2==0 {0x41}else{1};let targets=(case as u16).wrapping_mul(37);
        let hex=|b:&[u8]|b.iter().map(|v|format!("{v:02X}")).collect::<String>();
        let before=format!("{case},{slot},{flags},{gauge},{},{},{},{},{},{},{},{},{mp_flags},{targets}",scheduler.queues.advance.head,scheduler.queues.advance.tail,hex(&scheduler.queues.advance.bytes),hex(&commands.links),hex(&commands.normal),hex(&commands.retaliation),scheduler.queues.immediate.unwrap_or(255),hex(&payload_bytes(&commands)));
        remove_all(slot,&mut scheduler,&mut commands,targets,&mut mp_flags)?;
        writeln!(rows,"{before},{},{},{},{},{},{},{},{},{mp_flags}",scheduler.actors[slot].flags0,scheduler.actors[slot].gauge.counter,hex(&scheduler.queues.advance.bytes),hex(&commands.links),hex(&commands.normal),hex(&commands.retaliation),scheduler.queues.immediate.unwrap_or(255),hex(&payload_bytes(&commands))).unwrap();
    }
    fs::write(directory.join("removal.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn living_excludes_disabled_states_but_override_can_bypass_status() {
        let mut actors=[Actor::default();10];actors[0]=Actor {present:true,status1:128};actors[6]=Actor {present:true,status1:0};
        let mut s=Living {untargetable:0,override_targets:0,valid:0,alive:0,enemy_alive:0,revived:65535,enemy_characters:0,dead_monsters:0,allies:0,enemies:0};update_living(&actors,&mut s);assert_eq!((s.valid,s.alive,s.allies,s.enemies),(0x401,0x400,0,1));
        s.override_targets=1;update_living(&actors,&mut s);assert_eq!((s.allies,s.enemies),(1,1));
    }
    #[test]fn cleanup_preserves_retaliation_and_source_wrapped_scan() {
        let mut s=Scheduler::default();let mut pool=CommandLists::default();pool.normal(6,Payload::default()).unwrap();let retal=pool.retaliation(6,Payload::default(),&mut s.queues).unwrap();s.queues.advance.head=250;s.queues.advance.tail=3;s.queues.advance.bytes[250]=12;s.queues.advance.bytes[0]=12;
        remove_all(6,&mut s,&mut pool,0,&mut 0).unwrap();assert_eq!((pool.normal[6],pool.retaliation[6]),(255,retal));assert_eq!((s.queues.advance.bytes[250],s.queues.advance.bytes[0]),(255,12));
    }
    #[test]fn committed_guard_death_cleans_normal_actions_and_changes_living_count() {
        use crate::{damage_application as hp,battle_status_update as status};
        let mut target=hp::Target {hp:40,mp:15,max_hp:40,max_mp:15,status1:0,pending_status:0,update_flags:0,monster_flags:0,weapon_spell:0};hp::apply(&mut target,hp::Damage {taken:40,healed:65535,invincible:false,golem:255,dog:255,affects_mp:false});
        let mut actors=[status::Actor::default();10];
        for slot in [0,6] {actors[slot].flags=1;actors[slot].status.allowed12=65535;actors[slot].status.allowed34=65535;actors[slot].status.hp=if slot==0 {63}else{target.hp};actors[slot].status.max_hp=if slot==0 {63}else{40};}
        actors[6].status.pending.set12=target.pending_status;status::update(&mut actors,&mut status::Globals::default());assert_eq!(actors[6].status.current12,128);assert_ne!(actors[6].update&64,0);
        let mut scheduler=Scheduler::default();let mut pool=CommandLists::default();pool.normal(6,Payload {command:0,attack:238,targets:1,mp_cost:0}).unwrap();scheduler.enqueue_advance(6);let reaction=pool.retaliation(6,Payload::default(),&mut scheduler.queues).unwrap();
        scheduler.actors[6].flags0=actors[6].flags as u8;remove_all(6,&mut scheduler,&mut pool,0,&mut 0).unwrap();assert_eq!((pool.normal[6],pool.retaliation[6]),(255,reaction));
        let state=std::array::from_fn(|slot|Actor {present:actors[slot].flags&1!=0,status1:actors[slot].status.current12 as u8});let mut living=Living {untargetable:0,override_targets:0,valid:0,alive:0,enemy_alive:0,revived:65535,enemy_characters:0,dead_monsters:0,allies:0,enemies:0};update_living(&state,&mut living);assert_eq!((living.alive,living.allies,living.enemies),(1,1,0));
        // Full AfterAction2, retaliation completion and victory are not implied.
    }
    #[test]fn character_cleanup_appends_pose_action_and_preserves_other_immediates() {
        let mut scheduler=Scheduler::default();let mut pool=CommandLists::default();pool.normal(0,Payload::default()).unwrap();let first=pool.immediate(Payload {command:35,attack:7,targets:1,mp_cost:0},&mut scheduler.queues).unwrap();let mut flags=0x41;
        remove_all(0,&mut scheduler,&mut pool,0x1234,&mut flags).unwrap();assert_eq!(scheduler.queues.immediate,Some(first));let next=pool.links[usize::from(first)];let p=pool.payloads[usize::from(next)];assert_eq!(p,Payload {command:44,attack:0,targets:0x1234,mp_cost:0});assert_eq!(flags,1);
    }
}
