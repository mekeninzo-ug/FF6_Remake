//! Casting gate C2/3201..3224 and cached MP payment C2/32B1..32EB.
//! Working spell cost controls Silence/Imp; queued cached cost controls payment.
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Payment {Free,Paid,Insufficient}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Effect {ResetAction(u8),SaveAttackMessage,NotEnoughMp,CopyGraphics}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct State {pub mp:u16,pub cached_cost:u16,pub update_flags:u16,pub command:u8,pub message_flags:u16,pub gfx_a2:u16,pub gfx_a4:u16}
pub fn casting_allowed(working_cost:u8,status1:u8,status2:u8,last_spell:u8,actor_offset:u8,effects:&mut Vec<Effect>)->bool {
    if working_cost==0 || (status2&8==0 && (status1&0x20==0 || last_spell==35)) {return true;}
    effects.push(Effect::ResetAction(actor_offset>>1));false
}
pub fn pay(state:&mut State,actor_offset:u8,effects:&mut Vec<Effect>)->Payment {
    if state.cached_cost==0 {return Payment::Free;}
    let cost=state.cached_cost;state.cached_cost=0;
    if state.mp>=cost {state.mp-=cost;state.update_flags|=0x80;Payment::Paid}
    else {
        if actor_offset>=8 {state.command=18;}
        state.gfx_a2=0;state.gfx_a4=0;state.message_flags&=!2;
        effects.extend([Effect::SaveAttackMessage,Effect::NotEnoughMp,Effect::CopyGraphics]);Payment::Insufficient
    }
}
pub fn capture(directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,offset,cost,status1,status2,spell,allowed,reset\n");let mut case=0;
    for offset in (0..20).step_by(2) {for cost in [0,1,255] {for status1 in [0,0x20,0xff] {for status2 in [0,8,0xff] {for spell in [0,35,255] {
        let mut effects=Vec::new();let allowed=casting_allowed(cost,status1,status2,spell,offset,&mut effects);
        let reset=effects.first().map(|e|match e {Effect::ResetAction(id)=>id.to_string(),_=>unreachable!()}).unwrap_or_default();
        writeln!(rows,"{case},{offset},{cost},{status1},{status2},{spell},{},{reset}",allowed as u8).unwrap();case+=1;
    }}}}}
    fs::write(directory.join("casting.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,offset,mp,cost,update,command,message,gfx_a2,gfx_a4,result,mp_out,cost_out,update_out,command_out,message_out,gfx_a2_out,gfx_a4_out,effects\n");let mut seed=0x6ff6u32;
    for case in 0..16384usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u16};
        let mp=if case<256 {case as u16}else{next()};let cost=if case%7==0 {0}else if case%5==0 {mp}else{next()};
        let mut state=State {mp,cached_cost:cost,update_flags:next(),command:(case%32) as u8,message_flags:next(),gfx_a2:next(),gfx_a4:next()};let before=state;let offset=(case%10*2) as u8;let mut effects=Vec::new();
        let result=match pay(&mut state,offset,&mut effects) {Payment::Free=>0,Payment::Paid=>1,Payment::Insufficient=>2};
        writeln!(rows,"{case},{offset},{mp},{cost},{},{},{},{},{},{result},{},{},{},{},{},{},{},{}",before.update_flags,before.command,before.message_flags,before.gfx_a2,before.gfx_a4,state.mp,state.cached_cost,state.update_flags,state.command,state.message_flags,state.gfx_a2,state.gfx_a4,if effects.is_empty() {""}else{"save|mp|copy"}).unwrap();
    }
    fs::write(directory.join("payment.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn casting_gate_uses_working_cost_and_imp_spell_exception() {
        let mut effects=Vec::new();assert!(casting_allowed(0,32,8,0,0,&mut effects));assert!(effects.is_empty());
        assert!(casting_allowed(1,32,0,35,0,&mut effects));assert!(!casting_allowed(1,32,8,35,6,&mut effects));assert_eq!(effects,vec![Effect::ResetAction(3)]);
    }
    #[test]fn exact_payment_spends_last_mp_and_failure_preserves_mp() {
        let mut s=State {mp:7,cached_cost:7,update_flags:0,command:2,message_flags:65535,gfx_a2:123,gfx_a4:456};let mut effects=Vec::new();
        assert_eq!(pay(&mut s,0,&mut effects),Payment::Paid);assert_eq!((s.mp,s.cached_cost,s.update_flags),(0,0,128));
        s.cached_cost=1;assert_eq!(pay(&mut s,12,&mut effects),Payment::Insufficient);assert_eq!((s.mp,s.command,s.message_flags,s.gfx_a2,s.gfx_a4),(0,18,65533,0,0));assert_eq!(effects,vec![Effect::SaveAttackMessage,Effect::NotEnoughMp,Effect::CopyGraphics]);
    }
    #[test]fn queued_magic_uses_working_cost_for_casting_and_cached_cost_for_payment() {
        use crate::{battle_commands::{CommandLists,MpCosts,Payload},battle_action_init,battle_rng::{BattleRng,RngTable},battle_target_init};
        let root=Path::new("extracted/twue-3.07");let attacks=crate::damage::AttackTable::load(root).unwrap();let costs=MpCosts::from_attacks(&attacks);let mut flags=0;
        let cost=costs.get(2,0,12,&mut flags).unwrap();assert_eq!(cost,4);
        let mut pool=CommandLists::default();pool.normal(6,Payload {command:2,attack:0,targets:1,mp_cost:cost}).unwrap();
        let mut ctx=battle_action_init::Context::default();let mut rng=BattleRng::default();
        let a=battle_action_init::initialize_normal(&pool,6,false,0,&mut ctx,&mut rng,&RngTable::identity(),&battle_action_init::CommandProperties::load(root).unwrap()).unwrap().unwrap();
        let initializer=battle_target_init::Initializer::load(root).unwrap();let mut working=battle_target_init::State {restrictions:0,cursor:0,flags_b1:0,last_spell:0,last_item:0,magic:[0;14]};initializer.initialize(a.command,a.attack,&mut working);
        let mut effects=Vec::new();assert!(!casting_allowed(working.magic[5],0,8,working.last_spell,12,&mut effects));assert_eq!(effects,vec![Effect::ResetAction(6)]);
        effects.clear();assert!(casting_allowed(working.magic[5],0,0,working.last_spell,12,&mut effects));
        let mut state=State {mp:15,cached_cost:u16::from(a.mp_cost),update_flags:0,command:a.command,message_flags:65535,gfx_a2:0,gfx_a4:0};
        assert_eq!(pay(&mut state,12,&mut effects),Payment::Paid);assert_eq!((state.mp,state.cached_cost,state.update_flags),(11,0,128));assert_eq!(rng.index,0);
    }
}
