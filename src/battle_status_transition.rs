//! InitStatusVars C2/450D..4584, post-handler store C2/4585..4597 and
//! ResetStatusMod C2/44FF..450C. Individual status handlers are not bypassed.
use crate::battle_status_requests::Changes;
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct State {
    pub current12:u16,pub current34:u16,pub pending:Changes,pub allowed12:u16,pub allowed34:u16,
    pub hp:u16,pub max_hp:u16,pub extra_status:u16,pub retaliation_marker:u16,
    pub retaliation12:u16,pub retaliation34:u16,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Frame {pub changes:Changes,pub current12:u16,pub current34:u16}
pub fn prepare(s:&mut State)->Frame {
    let mut changes=Changes {set12:s.pending.set12&s.allowed12,set34:s.pending.set34&s.allowed34,clear12:s.pending.clear12,clear34:s.pending.clear34};
    changes.set12|=s.current12&0x40;
    let near=s.max_hp>>3>=s.hp;
    if s.current12&0x0200==0 && near {changes.set12|=0x0200;}
    if s.current12&0x0200!=0 && !near {changes.clear12|=0x0200;}
    if changes.set12&0x80!=0 && s.extra_status&2!=0 {changes.set12=(changes.set12|2)&!0x80;changes.clear12|=0x0100;}
    if s.retaliation_marker&0x8000!=0 {(s.retaliation12,s.retaliation34)=changes.resolved(s.current12,s.current34);}
    Frame {changes,current12:s.current12,current34:s.current34}
}
/// Caller must run the source's individual status handlers and CalcStatus
/// before supplying these resolved words. This is not a full UpdateStatus.
pub fn store_after_handlers(s:&mut State,mut resolved12:u16,resolved34:u16) {
    if resolved12&2!=0 {resolved12&=0x4dfa;}
    s.current12=resolved12;s.current34=resolved34;
}
pub fn reset_pending(s:&mut State) {s.pending=Changes::default();}
pub fn capture(directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let mut seed=0x6ff6u32;
    let mut rows=String::from("case,current12,current34,set12,set34,clear12,clear34,allowed12,allowed34,hp,max_hp,extra,marker,retal12,retal34,out_set12,out_set34,out_clear12,out_clear34,out_retal12,out_retal34,resolved12,resolved34,stored12,stored34,reset\n");
    for case in 0..16384usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u16};
        let max_hp=next();let hp=if case%5==0 {max_hp>>3}else if case%7==0 {(max_hp>>3)+1}else{next()};
        let mut s=State {current12:next(),current34:next(),pending:Changes {set12:next(),set34:next(),clear12:next(),clear34:next()},allowed12:next(),allowed34:next(),hp,max_hp,extra_status:next(),retaliation_marker:next(),retaliation12:next(),retaliation34:next()};let before=s;let frame=prepare(&mut s);let resolved=frame.changes.resolved(frame.current12,frame.current34);let retal=(s.retaliation12,s.retaliation34);store_after_handlers(&mut s,resolved.0,resolved.1);reset_pending(&mut s);
        writeln!(rows,"{case},{},{},{},{},{},{},{},{},{hp},{max_hp},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",before.current12,before.current34,before.pending.set12,before.pending.set34,before.pending.clear12,before.pending.clear34,before.allowed12,before.allowed34,before.extra_status,before.retaliation_marker,before.retaliation12,before.retaliation34,frame.changes.set12,frame.changes.set34,frame.changes.clear12,frame.changes.clear34,retal.0,retal.1,resolved.0,resolved.1,s.current12,s.current34,format!("{:04X}{:04X}{:04X}{:04X}",s.pending.set12,s.pending.set34,s.pending.clear12,s.pending.clear34)).unwrap();
    }
    fs::write(directory.join("preparation.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn state()->State {State {current12:0,current34:0,pending:Changes::default(),allowed12:65535,allowed34:65535,hp:10,max_hp:80,extra_status:0,retaliation_marker:0,retaliation12:0,retaliation34:0}}
    #[test]fn near_fatal_includes_exact_one_eighth_and_preserves_petrify() {
        let mut s=state();s.current12=64;s.allowed12=0;
        assert_eq!(prepare(&mut s).changes.set12,0x240);
        s.current12=0x240;s.hp=11;assert_eq!(prepare(&mut s).changes.clear12,0x200);
    }
    #[test]fn overcast_reads_pending_dead_instead_of_current_dead() {
        let mut s=state();s.extra_status=2;s.current12=128;assert_eq!(prepare(&mut s).changes.set12&2,0);
        s.pending.set12=128;let frame=prepare(&mut s);assert_eq!(frame.changes.set12&0x82,2);assert_eq!(frame.changes.clear12&0x100,0x100);
    }
    #[test]fn retaliation_snapshot_does_not_replace_pending_or_current_status() {
        let mut s=state();s.hp=80;s.pending.set12=4;s.pending.clear12=1;s.current12=1;s.retaliation_marker=0x8000;
        let frame=prepare(&mut s);assert_eq!(s.retaliation12,4);assert_eq!((s.current12,s.pending.set12),(1,4));assert_eq!(frame.changes.set12,4);
    }
    #[test]fn immune_poison_request_can_complete_a_no_change_transition() {
        use crate::battle_status_requests::{self,Target};
        let mut target=Target {current12:1,current34:0,pending:Changes::default(),allowed12:65531,allowed34:65535,monster_flags:0,mask:1,failed_mask:0,miss_targets:0};
        let mut magic=[0;14];magic[10]=4;battle_status_requests::queue(&magic,128,&mut target).unwrap();assert_eq!(target.pending.set12,4);
        let mut s=state();s.hp=80;s.current12=target.current12;s.pending=target.pending;s.allowed12=target.allowed12;
        let frame=prepare(&mut s);assert_eq!(frame.changes,Changes::default());
        // With no set/remove bits, UpdateStatus invokes no per-status handlers.
        let resolved=frame.changes.resolved(frame.current12,frame.current34);store_after_handlers(&mut s,resolved.0,resolved.1);reset_pending(&mut s);
        assert_eq!((s.current12,s.current34,s.pending),(1,0,Changes::default()));
    }
    #[test]fn lethal_damage_request_enters_overcast_preparation_without_fake_commit() {
        use crate::damage_application as hp;
        let mut target=hp::Target {hp:1,mp:0,max_hp:80,max_mp:0,status1:0,pending_status:0,update_flags:0,monster_flags:0,weapon_spell:7};
        hp::apply(&mut target,hp::Damage {taken:1,healed:65535,invincible:false,golem:255,dog:255,affects_mp:false});assert_eq!((target.hp,target.pending_status),(0,128));
        let mut s=state();s.hp=target.hp;s.pending.set12=target.pending_status;s.extra_status=2;
        let frame=prepare(&mut s);assert_eq!((frame.changes.set12,frame.changes.clear12),(0x202,0x100));assert_eq!(s.current12,0);
    }
}
