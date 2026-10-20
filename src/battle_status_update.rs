//! Complete target UpdateStatus C2/4391..4405 and its patched status handlers.
use crate::{battle_status_requests::Changes,battle_status_transition::{self,Frame,State},battle_targets::actor_mask};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct Actor {pub status:State,pub flags:u16,pub update:u16,pub sleep:u8,pub stop:u8,pub reflect:u8,pub freeze:u8}
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct Globals {pub confused:u16,pub changed_characters:u16,pub vanish_monsters:u16,pub alive_monsters:u8,pub dead_monsters:u16,pub newly_dead:u16,pub equipment:[u8;5]}
fn clear_existing(f:&mut Frame,mask:u16) {f.changes.clear12|=(f.current12|f.changes.set12)&mask;}
fn changed(slot:usize,g:&mut Globals) {g.changed_characters|=actor_mask(slot)&255;}
fn revive(slot:usize,g:&mut Globals) {let mask=(actor_mask(slot)>>8) as u8;g.alive_monsters|=mask;g.dead_monsters&=!u16::from(mask);}
fn rage_clear(slot:usize,f:&mut Frame,g:&mut Globals) {
    if slot<4 {
        // TWUE adds Dance/Rage removal, then increments the source word at a
        // byte-indexed equipment flag. Preserve its carry into the next byte.
        f.changes.clear34|=0x0101;
        let value=u16::from_le_bytes([g.equipment[slot],g.equipment[slot+1]]).wrapping_add(1).to_le_bytes();g.equipment[slot..slot+2].copy_from_slice(&value);
    }
}
fn stop_menu(a:&mut Actor) {a.flags&=0xff7f;a.update|=0x40;}
fn petrify(slot:usize,a:&mut Actor,f:&mut Frame,g:&mut Globals) {
    g.newly_dead|=actor_mask(slot);clear_existing(f,0xfe15);
    f.changes.clear34|=(f.current34|f.changes.set34)&0x9bff;
    a.status.extra_status&=!0x40;stop_menu(a);
}
fn handler(set:bool,id:usize,slot:usize,a:&mut Actor,f:&mut Frame,g:&mut Globals) {
    match (set,id) {
        (true,1)=>{clear_existing(f,0x80);g.newly_dead|=actor_mask(slot);changed(slot,g);},
        (false,1)=>{revive(slot,g);changed(slot,g);},
        (true,13)=>{g.confused|=actor_mask(slot);changed(slot,g);},
        (false,13)=>{g.confused&=!actor_mask(slot);changed(slot,g);},
        (true,4)=>g.vanish_monsters|=actor_mask(slot)>>8,
        (false,4)=>g.vanish_monsters&=!(actor_mask(slot)>>8),
        (_,5)=>{a.update|=0x88;rage_clear(slot,f,g);},
        (false,24)=>rage_clear(slot,f,g),
        (false,6)|(false,7)=>{revive(slot,g);a.flags|=0x4000;a.update|=0x40;},
        (true,7)=>{clear_existing(f,0x0140);f.changes.clear12&=!0x80;petrify(slot,a,f,g);},
        (true,6)=>petrify(slot,a,f,g),
        (true,15)=>{a.sleep=18;stop_menu(a);},
        (false,15)=>a.flags|=0x4000,
        (true,8)=>a.update|=0x20,
        (false,8)=>a.update|=0x10,
        (_,11)=>a.update|=8,
        (true,14)=>f.changes.clear34|=2,
        (true,17)=>f.changes.clear12|=0x4000,
        (true,18)=>{f.changes.clear34|=8;a.update|=4;},
        (true,19)=>{f.changes.clear34|=4;a.update|=4;},
        (false,18)|(false,19)=>a.update|=4,
        (_,27)=>a.update|=2,
        (true,20)=>a.stop=18,
        (true,23)=>a.reflect=26,
        (true,25)=>a.freeze=34,
        _=>{}, // Remaining entries point to the target's RTS no-effect handler.
    }
}
pub fn update(actors:&mut [Actor;10],globals:&mut Globals) {
    for slot in (0..10).rev() {
        let a=&mut actors[slot];if a.flags&1==0 {continue;}
        let mut frame=battle_status_transition::prepare(&mut a.status);
        let mask=frame.changes.set12;for bit in (0..16).rev() {if mask&(1<<bit)!=0 {handler(true,bit,slot,a,&mut frame,globals);}}
        let mask=frame.changes.set34;for bit in (0..16).rev() {if mask&(1<<bit)!=0 {handler(true,bit+16,slot,a,&mut frame,globals);}}
        frame.changes.clear12&=a.status.allowed12;let mask=frame.changes.clear12;
        for bit in (0..16).rev() {if mask&(1<<bit)!=0 {handler(false,bit,slot,a,&mut frame,globals);}}
        frame.changes.clear34&=a.status.allowed34;let mask=frame.changes.clear34;
        for bit in (0..16).rev() {if mask&(1<<bit)!=0 {handler(false,bit+16,slot,a,&mut frame,globals);}}
        let resolved=frame.changes.resolved(frame.current12,frame.current34);
        battle_status_transition::store_after_handlers(&mut a.status,resolved.0,resolved.1);battle_status_transition::reset_pending(&mut a.status);
    }
}
fn actor_bytes(a:&Actor)->Vec<u8> {
    let s=&a.status;let mut b=Vec::new();
    for v in [s.current12,s.current34,s.pending.set12,s.pending.set34,s.pending.clear12,s.pending.clear34,s.allowed12,s.allowed34,s.hp,s.max_hp,s.extra_status,s.retaliation_marker,s.retaliation12,s.retaliation34,a.flags,a.update] {b.extend(v.to_le_bytes());}
    b.extend([a.sleep,a.stop,a.reflect,a.freeze]);b
}
fn global_bytes(g:&Globals)->Vec<u8> {
    let mut b=Vec::new();for v in [g.confused,g.changed_characters,g.vanish_monsters,u16::from(g.alive_monsters),g.dead_monsters,g.newly_dead] {b.extend(v.to_le_bytes());}b.extend(g.equipment);b
}
pub fn capture(directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let mut seed=0x6ff6u32;let mut rows=String::from("case,actors,globals,actors_out,globals_out\n");
    for case in 0..2048usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u16};
        let mut actors=std::array::from_fn(|_|Actor {status:State {current12:next(),current34:next(),pending:Changes {set12:next(),set34:next(),clear12:next(),clear34:next()},allowed12:next(),allowed34:next(),hp:next(),max_hp:next(),extra_status:u16::from(next() as u8),retaliation_marker:next(),retaliation12:next(),retaliation34:next()},flags:next(),update:next(),sleep:next() as u8,stop:next() as u8,reflect:next() as u8,freeze:next() as u8});
        let mut globals=Globals {confused:next(),changed_characters:next(),vanish_monsters:next(),alive_monsters:next() as u8,dead_monsters:next(),newly_dead:next(),equipment:std::array::from_fn(|_|next() as u8)};
        let hex=|b:&[u8]|b.iter().map(|v|format!("{v:02X}")).collect::<String>();let before=hex(&actors.iter().flat_map(actor_bytes).collect::<Vec<_>>());let before_globals=hex(&global_bytes(&globals));update(&mut actors,&mut globals);
        writeln!(rows,"{case},{before},{before_globals},{},{}",hex(&actors.iter().flat_map(actor_bytes).collect::<Vec<_>>()),hex(&global_bytes(&globals))).unwrap();
    }
    for slot in 0..10 {for id in 0..32 {for set in [false,true] {
        let case=2048+slot*64+id*2+usize::from(set);let mut actors=[Actor::default();10];let a=&mut actors[slot];a.flags=0x81;a.status.hp=40;a.status.max_hp=40;a.status.allowed12=65535;a.status.allowed34=65535;
        if id<16 {if set {a.status.pending.set12=1<<id;}else{a.status.current12=1<<id;a.status.pending.clear12=1<<id;}}
        else if set {a.status.pending.set34=1<<(id-16);}else{a.status.current34=1<<(id-16);a.status.pending.clear34=1<<(id-16);}
        if id==5 || id==24 {a.status.current34|=0x0101;}
        let mut globals=Globals {equipment:[255,7,0,0,0],..Globals::default()};
        let hex=|b:&[u8]|b.iter().map(|v|format!("{v:02X}")).collect::<String>();let before=hex(&actors.iter().flat_map(actor_bytes).collect::<Vec<_>>());let before_globals=hex(&global_bytes(&globals));update(&mut actors,&mut globals);
        writeln!(rows,"{case},{before},{before_globals},{},{}",hex(&actors.iter().flat_map(actor_bytes).collect::<Vec<_>>()),hex(&global_bytes(&globals))).unwrap();
    }}}
    fs::write(directory.join("update.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn dead_handler_clears_incompatible_status_and_marks_death() {
        let mut actors=[Actor::default();10];let a=&mut actors[6];a.flags=0x81;a.status.allowed12=65535;a.status.allowed34=65535;a.status.current12=0xffff;a.status.current34=0xffff;a.status.pending.set12=128;a.status.hp=0;a.status.max_hp=40;
        let mut globals=Globals::default();update(&mut actors,&mut globals);assert_ne!(globals.newly_dead&0x400,0);assert_eq!(actors[6].flags&0x80,0);assert_eq!(actors[6].status.pending,Changes::default());
    }
    #[test]fn absent_actor_keeps_pending_changes() {
        let mut actors=[Actor::default();10];actors[0].status.pending.set12=4;let before=actors[0];update(&mut actors,&mut Globals::default());assert_eq!(actors[0],before);
    }
    #[test]fn patched_imp_cancels_dance_and_rage_and_keeps_word_increment_carry() {
        let mut actors=[Actor::default();10];let a=&mut actors[0];a.flags=1;a.status.allowed12=65535;a.status.allowed34=65535;a.status.hp=80;a.status.max_hp=80;a.status.current34=0x0101;a.status.pending.set12=32;
        let mut globals=Globals::default();globals.equipment=[255,7,0,0,0];update(&mut actors,&mut globals);
        assert_eq!(actors[0].status.current12,32);assert_eq!(actors[0].status.current34,0);assert_eq!(globals.equipment[..2],[1,8]);assert_eq!(actors[0].update&0x88,0x88);
        // Imp clears Rage; its remove handler increments the flag a second time.
    }
    #[test]fn native_lethal_damage_commits_dead_and_revive_updates_monster_masks() {
        use crate::damage_application as hp;
        let mut target=hp::Target {hp:40,mp:15,max_hp:40,max_mp:15,status1:0,pending_status:0,update_flags:0,monster_flags:0,weapon_spell:0};
        hp::apply(&mut target,hp::Damage {taken:40,healed:65535,invincible:false,golem:255,dog:255,affects_mp:false});
        let mut actors=[Actor::default();10];let a=&mut actors[6];a.flags=0x81;a.status.hp=target.hp;a.status.max_hp=40;a.status.allowed12=65535;a.status.allowed34=65535;a.status.pending.set12=target.pending_status;
        let mut globals=Globals::default();update(&mut actors,&mut globals);assert_eq!((actors[6].status.current12,globals.newly_dead,actors[6].update),(128,0x400,64));assert_eq!(actors[6].flags&0x80,0);
        actors[6].status.hp=1;actors[6].status.pending.clear12=128;globals.dead_monsters=4;
        update(&mut actors,&mut globals);assert_eq!(actors[6].status.current12&128,0);assert_eq!((globals.alive_monsters,globals.dead_monsters),(4,0));assert_ne!(actors[6].flags&0x4000,0);
    }
    #[test]fn status_request_runs_through_handlers_and_commit() {
        use crate::battle_status_requests::{self,Target};
        let mut target=Target {current12:0,current34:0,pending:Changes::default(),allowed12:65535,allowed34:65535,monster_flags:0,mask:1,failed_mask:0,miss_targets:0};let mut magic=[0;14];magic[10]=4;magic[7]=1;battle_status_requests::queue(&magic,128,&mut target).unwrap();assert_eq!(target.failed_mask,0);
        let mut actors=[Actor::default();10];let a=&mut actors[0];a.flags=1;a.status.hp=40;a.status.max_hp=40;a.status.allowed12=65535;a.status.allowed34=65535;a.status.pending=target.pending;
        update(&mut actors,&mut Globals::default());assert_eq!((actors[0].status.current12,actors[0].status.pending),(4,Changes::default()));
    }
    #[test]fn fire_beam_thaw_commits_without_inventing_counter_reset() {
        use crate::battle_status_requests::{self,Target};
        let root=Path::new("extracted/twue-3.07");let mut working=crate::battle_target_init::State {restrictions:0,cursor:0,flags_b1:0,last_spell:0,last_item:0,magic:[0;14]};crate::battle_target_init::Initializer::load(root).unwrap().initialize(29,131,&mut working);
        let mut target=Target {current12:0,current34:512,pending:Changes::default(),allowed12:65535,allowed34:65535,monster_flags:0,mask:1,failed_mask:0,miss_targets:0};battle_status_requests::queue(&working.magic,128,&mut target).unwrap();
        let mut actors=[Actor::default();10];let a=&mut actors[0];a.flags=1;a.freeze=34;a.status.current34=512;a.status.pending=target.pending;a.status.allowed12=65535;a.status.allowed34=65535;a.status.hp=40;a.status.max_hp=40;
        update(&mut actors,&mut Globals::default());assert_eq!((actors[0].status.current34,actors[0].freeze),(0,34));
    }
}
