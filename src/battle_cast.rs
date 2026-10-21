//! Target ExecAttack C2/3225..3235: interception, patched base damage and effects.
use crate::{battle_action_damage::{Meta,Prefix,Tables},battle_attack_effect::{self,ActorMeta,State as EffectState},battle_hit_resolution::{Context,Extra},battle_target_damage::Actor,battle_damage_exec::Graphics,battle_rng::{BattleRng,RngTable},battle_targets::actor_mask,damage::BaseDamage,equipment::ItemTable};
use std::{fs,path::Path};
#[derive(Clone,Debug)]pub struct State {pub stat:u8,pub level:u8,pub relic1:[u8;10]}

fn runic(a:&[Actor;10],m:&mut[Meta;10],am:&[ActorMeta;10],s:&mut EffectState,p:&mut Prefix,c:&mut Context,cast:&mut State) {
    if c.damage.magic[3]&8==0 {return;}
    let mut targets=0;
    for slot in (0..10).rev() {
        if am[slot].presence&1==0 || m[slot].retal_flags&6==0 {continue;}
        m[slot].retal_flags&=!4;
        if a[slot].status12&0x80c0==0 && a[slot].status34&0x2210==0 {targets|=actor_mask(slot);}
    }
    if targets==0 {return;}
    battle_attack_effect::setup_old_targets(s);s.modification_high=0;s.targets=targets;
    c.damage.magic[10..14].fill(0);c.damage.magic[3]=0x82;c.damage.magic[4]=0x21;c.damage.magic[2]=0x60;
    c.damage.magic[6]=c.damage.magic[5]/targets.count_ones() as u8;cast.stat=0;cast.level=0;c.damage.enabled=0;c.damage.flags_b2&=!64;p.ba=4;c.damage.magic[7]&=!3;c.damage.magic[9]=0;
}
fn air_anchor(attacker:usize,a:&mut[Actor;10],c:&mut Context) {if a[attacker].update&0x400==0 {a[attacker].update|=0x400;c.damage.magic[3]|=64;}}
pub fn run(a:&mut[Actor;10],e:&mut[Extra;10],m:&mut[Meta;10],am:&[ActorMeta;10],s:&mut EffectState,p:&mut Prefix,c:&mut Context,g:&mut Graphics,rng:&mut BattleRng,table:&RngTable,items:&ItemTable,t:&Tables,cast:&mut State)->Result<(),String> {
    let attacker=c.damage.execution.attacker;if attacker>=10 {return Err("Cast attacker outside ten actors".into());}
    runic(a,m,am,s,p,c,cast);air_anchor(attacker,a,c);
    c.damage.base=BaseDamage {level:cast.level,power:c.damage.magic[6],stat:cast.stat,physical:c.damage.magic[2]&1!=0,monster:attacker>=4,restorative:c.damage.magic[4]&1!=0,affects_mp:c.damage.magic[3]&128!=0,attack_flags:c.damage.flags_b2,command:c.damage.command,relic_damage:u16::from(cast.relic1[attacker])|(u16::from(e[attacker].effect.relic3)<<8),relic4:m[attacker].relic4}.calculate();
    c.damage.execution.attack_flags=u16::from(c.damage.magic[2])|(u16::from(c.damage.magic[3])<<8);
    battle_attack_effect::run(a,e,m,am,s,p,c,g,rng,table,items,t)
}
pub(crate) fn snapshot(a:&[Actor;10],e:&[Extra;10],m:&[Meta;10],am:&[ActorMeta;10],s:&EffectState,p:&Prefix,c:&Context,g:&Graphics,rng:&BattleRng,cast:&State)->String {
    use std::fmt::Write;let mut out=battle_attack_effect::snapshot(a,e,m,am,s,p,c,g,rng);
    write!(out,"7E11AE{:02X}7E11AF{:02X}",cast.stat,cast.level).unwrap();for (i,v) in cast.relic1.iter().enumerate() {write!(out,"{:06X}{v:02X}",0x7e3c44+i*2).unwrap();}out
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;let items=ItemTable::load(root)?;let t=Tables::load(root)?;let mut rows=String::from("case,attacker,before,after\n");
    for case in 0..4096 {let (mut a,mut e,mut m,am,mut s,mut p,mut c,mut g,mut rng)=battle_attack_effect::fixture(case);let mut cast=State {stat:(case*17) as u8,level:(case*31) as u8,relic1:std::array::from_fn(|i|[0,1,2,3,255][(case+i)%5])};
        if case%4==0 {c.damage.magic[3]|=8;}if case%3!=0 {a[c.damage.execution.attacker].update|=0x400;}
        let before=snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng,&cast);run(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&table,&items,&t,&mut cast)?;writeln!(rows,"{case},{},{before},{}",c.damage.execution.attacker,snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng,&cast)).unwrap();}
    fs::write(dir.join("cast.csv"),rows).map_err(|e|e.to_string())
}

#[cfg(test)]mod tests {
    use super::*;
    fn cast()->State {State {stat:100,level:30,relic1:[0;10]}}
    #[test]fn non_absorbable_attack_leaves_runic_flags_and_working_properties_alone() {
        let (a,_,mut m,am,mut s,mut p,mut c,_,_)=battle_attack_effect::fixture(1);c.damage.magic[3]&=!8;let flags=m.map(|v|v.retal_flags);let before=c.damage.magic;let mut cast=cast();runic(&a,&mut m,&am,&mut s,&mut p,&mut c,&mut cast);assert_eq!(m.map(|v|v.retal_flags),flags);assert_eq!(c.damage.magic,before);assert_eq!((cast.stat,cast.level),(100,30));
    }
    #[test]fn runic_consumes_character_flag_before_status_rejection_but_skips_absent_actors() {
        let (mut a,_,mut m,mut am,mut s,mut p,mut c,_,_)=battle_attack_effect::fixture(1);for i in 0..10 {am[i].presence=0;m[i].retal_flags=6;a[i].status12=0;a[i].status34=0;}am[0].presence=1;a[0].status12=128;am[1].presence=1;am[3].presence=1;a[3].status34=0x200;c.damage.magic[3]|=8;c.damage.magic[5]=7;let mut cast=cast();runic(&a,&mut m,&am,&mut s,&mut p,&mut c,&mut cast);
        assert_eq!((m[0].retal_flags,m[1].retal_flags,m[2].retal_flags,m[3].retal_flags),(2,2,6,2));assert_eq!((s.targets,c.damage.magic[6]),(2,7));
    }
    #[test]fn runic_splits_working_mp_cost_and_retains_original_target_history() {
        let (mut a,_,mut m,mut am,mut s,mut p,mut c,_,_)=battle_attack_effect::fixture(1);for i in 0..10 {am[i].presence=0;m[i].retal_flags=0;a[i].status12=0;a[i].status34=0;}for i in [0,4,9] {am[i].presence=1;m[i].retal_flags=6;}s.targets=256;s.backup=0;s.modification_high=255;c.damage.magic[3]|=8;c.damage.magic[5]=10;c.damage.magic[10..14].fill(255);let mut cast=cast();runic(&a,&mut m,&am,&mut s,&mut p,&mut c,&mut cast);
        assert_eq!((s.targets,s.old_targets,s.backup,s.modification_high),(0x2101,256,256,0));assert_eq!((c.damage.magic[2],c.damage.magic[3],c.damage.magic[4],c.damage.magic[6]),(0x60,0x82,0x21,3));assert_eq!((cast.stat,cast.level,c.damage.enabled,p.ba),(0,0,0,4));assert_eq!(&c.damage.magic[10..14],&[0;4]);
    }
    #[test]fn air_anchor_uses_shared_update_high_byte_and_only_requests_death_later() {
        let (mut a,_,_,_,_,_,mut c,_,_)=battle_attack_effect::fixture(1);a[1].update=0x8051;c.damage.magic[3]=8;air_anchor(1,&mut a,&mut c);assert_eq!((a[1].update,c.damage.magic[3],a[1].pending_set),(0x8451,0x48,0));c.damage.magic[3]=2;air_anchor(1,&mut a,&mut c);assert_eq!(c.damage.magic[3],2);
    }
    #[test]fn connected_runic_pays_caster_cost_and_restores_split_mp_with_two_parameter_records() {
        let root=Path::new("extracted/twue-3.07");let (mut a,mut e,mut m,mut am,mut s,mut p,mut c,mut g,mut rng)=battle_attack_effect::fixture(1);for i in 0..10 {m[i].retal_flags=0;am[i].presence=1;a[i].status12=0;a[i].status34=0;a[i].monster_flags=0;a[i].flags1=0;a[i].absorb=0;a[i].null=0;a[i].mp=0;a[i].max_mp=50;}for i in [0,4] {m[i].retal_flags=6;}a[1].mp=20;a[1].update|=0x400;s.special_index=0;s.cached_cost=7;s.cursor=4;s.modification_high=255;c.damage.magic[1]=0;c.damage.magic[3]=8;c.damage.magic[5]=7;c.damage.flags_b2=0x240;p.deferred=255;let mut cast=cast();run(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&RngTable::identity(),&ItemTable::load(root).unwrap(),&Tables::load(root).unwrap(),&mut cast).unwrap();
        assert_eq!((a[1].mp,a[0].mp,a[4].mp,c.damage.base,c.parameter_tail),(13,3,3,3,32));assert_eq!((m[0].retal_flags,m[4].retal_flags),(2,2));assert_eq!((cast.stat,cast.level),(0,0));
    }
    #[test]fn patched_base_caller_applies_relic_boost_once_using_shared_relic_three_byte() {
        let root=Path::new("extracted/twue-3.07");let (mut a,mut e,mut m,mut am,mut s,mut p,mut c,mut g,mut rng)=battle_attack_effect::fixture(1);a[1].update|=0x400;a[1].status12=0;a[1].status34=0;a[1].flags1=0;am[4].presence=1;s.special_index=0;s.cached_cost=0;s.targets=256;s.cursor=0x40;c.damage.magic[2]=0;c.damage.magic[3]=0;c.damage.magic[4]=0;c.damage.magic[6]=10;c.damage.magic[9]=0;c.damage.flags_b2=0x240;p.ba=0;p.deferred=0;let mut cast=State {stat:0,level:0,relic1:[0;10]};e[1].effect.relic3=2;
        run(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&RngTable::identity(),&ItemTable::load(root).unwrap(),&Tables::load(root).unwrap(),&mut cast).unwrap();assert_eq!((c.damage.base,e[1].effect.relic3),(12,2));
    }
}
