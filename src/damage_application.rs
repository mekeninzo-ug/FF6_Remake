//! Native target C2/1323..13D2 HP/MP application. Pending status changes are
//! retained separately; this does not commit the whole battle status engine.
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug)]
pub struct Target {
    pub hp:u16,pub mp:u16,pub max_hp:u16,pub max_mp:u16,
    pub status1:u16,pub pending_status:u16,pub update_flags:u16,
    pub monster_flags:u16,pub weapon_spell:u8,
}
#[derive(Clone,Copy,Debug)]
pub struct Damage {
    pub taken:u16,pub healed:u16,pub invincible:bool,
    /// The high bits of both global block-target bytes must be set to allow damage.
    pub golem:u8,pub dog:u8,pub affects_mp:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Outcome {
    pub taken:u16,pub net:u16,pub healing:bool,
    /// Raw helper return carry; nonlethal subtraction can also leave it set.
    pub carry:bool,
}
pub fn apply(target:&mut Target,mut d:Damage)->Outcome {
    let mut taken=if d.taken==65535 {0}else{d.taken};
    if d.taken!=65535 && d.invincible {taken=0;d.taken=0;}
    if d.golem&d.dog&0x80==0 {taken=0;}
    let healed=if d.healed==65535 {0}else{d.healed};
    let net=healed.wrapping_sub(taken);let healing=healed>=taken;
    let mut carry=false;
    if net!=0 {
        let current=if d.affects_mp {target.mp}else{target.hp};
        let maximum=if d.affects_mp {target.max_mp}else{target.max_hp};
        let value=if healing {(u32::from(current)+u32::from(net)).min(u32::from(maximum)) as u16}
            else {current.saturating_sub(net.wrapping_neg())};
        if d.affects_mp {target.mp=value;} else {target.hp=value;}
        carry=!healing;
        if !healing && value==0 {
            if !d.affects_mp || target.monster_flags&1!=0 {
                target.hp=0;target.weapon_spell=0;
                if target.status1&2==0 {target.pending_status|=0x80;}
            }
        }
        if d.affects_mp {target.update_flags|=0x80;}
    }
    Outcome {taken:d.taken,net,healing,carry}
}
pub fn capture(directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,hp,mp,max_hp,max_mp,status,pending,update,monster,weapon,taken,healed,invincible,golem,dog,affects_mp,hp_out,mp_out,pending_out,update_out,weapon_out,taken_out,net,healing,carry\n");
    let mut seed=0x6ff6u32;
    for case in 0..16384usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u16};
        let edges=[0,1,39,40,9998,9999,32768,65534,65535];
        let hp=if case<729 {edges[case%9]}else{next()};
        let mp=next();let max_hp=next();let max_mp=next();
        let taken=if case<729 {edges[case/9%9]}else{next()};
        let healed=if case<729 {edges[case/81%9]}else{next()};
        let mut t=Target {hp,mp,max_hp,max_mp,status1:if case%3==0 {2}else{0},pending_status:0x100,update_flags:0x400,monster_flags:(case/3%2) as u16,weapon_spell:19};
        let d=Damage {taken,healed,invincible:case/7%2!=0,golem:[255,0,127,128][case/11%4],dog:[255,0,127,128][case/17%4],affects_mp:case%2!=0};
        let before=t;let out=apply(&mut t,d);
        writeln!(rows,"{case},{hp},{mp},{max_hp},{max_mp},{},{},{},{},{},{taken},{healed},{},{},{},{},{},{},{},{},{},{},{},{},{}",before.status1,before.pending_status,before.update_flags,before.monster_flags,before.weapon_spell,d.invincible as u8,d.golem,d.dog,d.affects_mp as u8,t.hp,t.mp,t.pending_status,t.update_flags,t.weapon_spell,out.taken,out.net,out.healing as u8,out.carry as u8).unwrap();
    }
    fs::write(directory.join("application.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn target()->Target {Target {hp:40,mp:15,max_hp:40,max_mp:15,status1:0,pending_status:0,update_flags:0,monster_flags:0,weapon_spell:7}}
    fn damage()->Damage {Damage {taken:40,healed:65535,invincible:false,golem:255,dog:255,affects_mp:false}}
    #[test]fn lethal_hp_requests_death_without_committing_status() {
        let mut t=target();let out=apply(&mut t,damage());
        assert_eq!(t.hp,0);assert_eq!(t.pending_status,0x80);assert_eq!(t.status1,0);assert_eq!(t.weapon_spell,0);assert!(out.carry);
        let mut t=target();t.status1=2;apply(&mut t,damage());assert_eq!(t.pending_status,0);
    }
    #[test]fn mp_depletion_carry_is_not_itself_death() {
        let mut t=target();assert!(apply(&mut t,Damage {affects_mp:true,..damage()}).carry);
        assert_eq!((t.hp,t.mp,t.update_flags,t.pending_status),(40,0,128,0));
        let mut t=target();t.monster_flags=1;apply(&mut t,Damage {affects_mp:true,..damage()});assert_eq!((t.hp,t.pending_status),(0,128));
        let mut t=target();assert!(apply(&mut t,Damage {taken:1,..damage()}).carry);assert_eq!(t.hp,39);
    }
    #[test]fn connected_damage_stages_reduce_guard_hp_and_queue_dead() {
        use crate::{damage::{BaseDamage,Modifiers},elemental_damage as elements};
        let base=BaseDamage {level:3,power:60,stat:39,command:29,..BaseDamage::default()}.calculate();
        let modified=Modifiers {magic_defense:140,..Modifiers::default()}.calculate(base);
        let contribution=elements::resolve(modified,elements::Context {power:60,element:1,..elements::Context::default()}).unwrap().unwrap();
        assert_eq!((base,modified),(459,181));assert!(!contribution.healing());
        let accumulated=elements::accumulate(65535,contribution.amount);
        let mut guard=target();apply(&mut guard,Damage {taken:accumulated,..damage()});
        assert_eq!((guard.hp,guard.pending_status),(0,128));
    }
    #[test]fn sentinel_invincibility_blocking_and_net_healing() {
        let mut t=target();t.hp=10;let out=apply(&mut t,Damage {taken:65535,healed:65535,affects_mp:true,..damage()});assert_eq!(out.net,0);assert_eq!(t.update_flags,0);
        let out=apply(&mut t,Damage {healed:5,invincible:true,..damage()});assert_eq!((out.taken,t.hp),(0,15));
        apply(&mut t,Damage {golem:0,..damage()});assert_eq!(t.hp,15);
        apply(&mut t,Damage {taken:1,healed:9999,..damage()});assert_eq!(t.hp,40);
    }
}
