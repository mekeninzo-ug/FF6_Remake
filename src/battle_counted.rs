//! Record-derived Launcher and Super Ball diagnostics for connected execution.
use crate::{battle_action_damage::{Meta,Prefix,Tables},battle_attack_effect::{self,ActorMeta,State},battle_hit_resolution::{Context,Extra},battle_target_damage::Actor,battle_damage_exec::Graphics,battle_rng::{BattleRng,RngTable},battle_cast,battle_reflect,equipment::ItemTable,damage::AttackTable};
use std::{fs,path::Path};
fn fixture(case:usize,attacks:&AttackTable,items:&ItemTable,table:&RngTable)->([Actor;10],[Extra;10],[Meta;10],[ActorMeta;10],State,Prefix,Context,Graphics,BattleRng,battle_cast::State) {
    let (mut a,e,m,am,mut s,mut p,mut c,g,mut rng)=battle_attack_effect::fixture(case);let super_ball=case%2!=0;
    if super_ball {
        let r=items.record(250);
        c.damage.command=1;c.original_command=1;c.damage.animation=250;c.last_item=250;c.last_attack=255;c.damage.enabled=0;c.damage.flags_b2&=!0x8000;s.cursor=r[14];p.ba=0;s.cached_cost=0;
    }else {
        c.damage.magic.copy_from_slice(attacks.record(205));c.damage.magic[9]*=2;c.damage.command=2;c.original_command=2;c.damage.animation=205;c.last_attack=205;c.last_item=255;c.damage.enabled=255;s.cursor=c.damage.magic[0];p.ba=0;s.cached_cost=if case%11==0 {20}else{0};
    }
    if !super_ball && case%22==0 {a[c.damage.execution.attacker].mp=0;}
    s.special_index=if super_ball {1}else{0};p.deferred=255;s.modification_high=255;c.secondary_effect=0;c.damage.execution.attack_flags=u16::from(c.damage.magic[2])|(u16::from(c.damage.magic[3])<<8);
    if case%3!=0 {a[c.damage.execution.attacker].update|=0x400;}
    let mut cast=battle_cast::State {stat:if super_ball {0}else{(case*17) as u8},level:if super_ball {0}else{(case*31) as u8},relic1:std::array::from_fn(|i|[0,1,2,3,255][(case+i)%5])};
    if super_ball {crate::battle_item_effect::prepare(250,&crate::battle_item_effect::Context {tools_or_throw:false,strength:[0;10],sketch:255},&a,&mut c.damage,&mut cast,items,&mut rng,table).expect("normal Super Ball setup has no actor/Sketch lookup");}
    (a,e,m,am,s,p,c,g,rng,cast)
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;let items=ItemTable::load(root)?;let attacks=AttackTable::load(root)?;let t=Tables::load(root)?;let mut rows=String::from("case,combined,attacker,mask,effect,before,after\n");
    for case in 0..4096 {let (mut a,mut e,mut m,am,mut s,mut p,mut c,mut g,mut rng,mut cast)=fixture(case,&attacks,&items,&table);let effect=c.damage.magic[9];let before=battle_cast::snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng,&cast);battle_reflect::cast_and_reflect(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&table,&items,&t,&mut cast)?;writeln!(rows,"{case},1,{},0,{effect},{before},{}",c.damage.execution.attacker,battle_cast::snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng,&cast)).unwrap();}
    fs::write(dir.join("counted.csv"),rows).map_err(|e|e.to_string())
}

#[cfg(test)]mod tests {
    use super::*;
    fn controlled(case:usize)->([Actor;10],[Extra;10],[Meta;10],[ActorMeta;10],State,Prefix,Context,Graphics,BattleRng,battle_cast::State) {
        let root=Path::new("extracted/twue-3.07");let (mut a,e,mut m,mut am,mut s,mut p,mut c,_,_,cast)=fixture(case,&AttackTable::load(root).unwrap(),&ItemTable::load(root).unwrap(),&RngTable::identity());
        for i in 0..10 {a[i].hp=10000;a[i].max_hp=10000;a[i].status12=0;a[i].status34=0;a[i].monster_flags=0;a[i].flags1=0;a[i].absorb=0;a[i].null=0;a[i].half=0;a[i].weak=0;a[i].previous=127;a[i].control=255;a[i].update|=0x400;m[i].physical_block=0;m[i].magical_block=0;am[i].presence=1;am[i].charm=255;}
        s.targets=256;s.targettable=256;s.valid=256;s.revived=0x3f0f;s.jump_seize=0;s.remove_jump=false;s.enemy_characters=0;s.cached_cost=0;p.battle_type=0;p.ba=0;c.damage.flags_b2=0x240;
        (a,e,m,am,s,p,c,Graphics::default(),BattleRng::default(),cast)
    }
    fn execute(a:&mut[Actor;10],e:&mut[Extra;10],m:&mut[Meta;10],am:&[ActorMeta;10],s:&mut State,p:&mut Prefix,c:&mut Context,g:&mut Graphics,rng:&mut BattleRng,cast:&mut battle_cast::State) {
        let root=Path::new("extracted/twue-3.07");battle_reflect::cast_and_reflect(a,e,m,am,s,p,c,g,rng,&RngTable::identity(),&ItemTable::load(root).unwrap(),&Tables::load(root).unwrap(),cast).unwrap();
    }
    #[test]fn launcher_setup_defers_seven_and_replaces_reflection_mask_without_rng() {
        let root=Path::new("extracted/twue-3.07");let (mut a,mut e,m,_,_,mut p,mut c,mut g,mut rng,_)=controlled(0);c.params[3]=65535;
        crate::battle_action_damage::run(&mut a,&mut e,&m,&mut p,&mut c,&mut g,&mut rng,&RngTable::identity(),&ItemTable::load(root).unwrap(),&Tables::load(root).unwrap()).unwrap();assert_eq!((p.deferred,c.params[3],g.tail,c.parameter_tail,rng.index),(7,1,0,0,0));
    }
    #[test]fn super_ball_setup_draws_one_byte_for_one_through_four_hits() {
        let root=Path::new("extracted/twue-3.07");for count in 0..4u8 {let (mut a,mut e,m,_,_,mut p,mut c,mut g,mut rng,_)=controlled(1);rng.index=count.wrapping_sub(1);crate::battle_action_damage::run(&mut a,&mut e,&m,&mut p,&mut c,&mut g,&mut rng,&RngTable::identity(),&ItemTable::load(root).unwrap(),&Tables::load(root).unwrap()).unwrap();assert_eq!((p.deferred,c.params[3],c.damage.animation,rng.index,g.tail,c.parameter_tail),(count,2,125,count,0,0));}
    }
    #[test]fn super_ball_target_effect_returns_each_new_base_without_mutating_steal_state() {
        use crate::battle_target_effects as fx;for value in 0..8u8 {let mut a=[fx::Actor::default();10];let mut s=fx::State::default();let mut command=29;let mut rng=BattleRng {index:value.wrapping_sub(1)};let out=fx::dispatch(148,0,4,&[1;10],&mut a,&mut command,&mut s,&mut rng,&RngTable::identity()).unwrap();assert_eq!(out,fx::Output::SetBase(u16::from(value+1)*256));assert_eq!((command,s.message,s.gil,rng.index),(29,0,0,value));}
    }
    #[test]fn launcher_record_produces_eight_sequential_fraction_hits() {
        let (mut a,mut e,mut m,am,mut s,mut p,mut c,mut g,mut rng,mut cast)=controlled(0);a[4].hp=1000;a[4].max_hp=1000;execute(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&mut cast);assert_eq!((a[4].hp,p.deferred,c.parameter_tail,c.damage.animation),(4,255,16,205));assert_eq!(&c.params[..4],&[0x0404;4]);assert_eq!(g.script[0],21);
    }
    #[test]fn super_ball_target_override_occurs_after_initial_base_halving() {
        let (mut a,mut e,mut m,am,mut s,mut p,mut c,mut g,mut rng,mut cast)=controlled(1);rng.index=255;execute(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&mut cast);assert_eq!((a[4].hp,c.damage.base,c.damage.animation,rng.index,p.deferred),(9232,768,125,2,255));assert_eq!(c.params[0]&255,4);
    }
    #[test]fn super_ball_four_hits_draw_separate_bases_and_repeat_history_rng() {
        let (mut a,mut e,mut m,am,mut s,mut p,mut c,mut g,mut rng,mut cast)=controlled(1);rng.index=2;execute(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&mut cast);assert_eq!((a[4].hp,c.damage.base,rng.index,p.deferred),(5392,1792,14,255));assert_eq!(&c.params[..2],&[0x0404;2]);assert_eq!(c.parameter_tail,16);
    }
    #[test]fn insufficient_launcher_mp_does_not_schedule_counted_damage() {
        let (mut a,mut e,mut m,am,mut s,mut p,mut c,mut g,mut rng,mut cast)=controlled(0);a[0].mp=0;s.cached_cost=20;execute(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&mut cast);assert_eq!((a[4].hp,p.deferred,c.params[3],c.parameter_tail,g.tail),(10000,255,0,16,8));assert_eq!(&g.script[4..6],&[2,40]);
    }
}
