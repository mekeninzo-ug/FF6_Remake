//! Patched CalcReflectDmg C2/3483..352A; original labels move within this span.
use crate::{battle_action_damage::{self,Meta,Prefix,Tables},battle_attack_effect::{self,ActorMeta,State as EffectState},battle_hit_resolution::{self,Context,Extra},battle_target_damage::{self,Actor},battle_damage_exec::Graphics,battle_rng::{BattleRng,RngTable},battle_targets::actor_mask,battle_attack_history::{self,Action},battle_hit::Outcome,equipment::ItemTable,battle_cast};
use std::{fs,path::Path};
fn map_target(c:&mut Context,index:usize,target:usize)->Result<(),String> {
    if index>=10 {return Err("Reflected-hit mapping needs one of ten actor/count positions".into());}
    let shift=(index%2)*8;c.params[index/2]=(c.params[index/2]&!(255<<shift))|((target as u16)<<shift);Ok(())
}
pub fn run(mask:u16,a:&mut[Actor;10],e:&mut[Extra;10],m:&[Meta;10],am:&[ActorMeta;10],s:&mut EffectState,p:&mut Prefix,c:&mut Context,g:&mut Graphics,rng:&mut BattleRng,table:&RngTable,items:&ItemTable)->Result<(),String> {
    let original=c.damage.execution.attacker;if original>=10 {return Err("Reflection caster outside ten actors".into());}
    if !crate::battle_target_effects::supported(c.damage.magic[9]) {return Err("Reflected primary effect needs its native target handler".into());}
    c.params=[0;8];c.damage.execution.misses=0;c.damage.magic[3]|=0x22;c.damage.execution.attack_flags=u16::from(c.damage.magic[2])|(u16::from(c.damage.magic[3])<<8);s.cursor=0x40;p.ba|=0x40;
    let animation=if p.deferred&128!=0 {p.ba|=16;9}else{21};g.parameter=(g.parameter&0xff00)|u16::from(c.damage.animation);g.command=(g.command&0xff00)|animation;g.append();
    c.params[..5].fill(65535);c.damage.base>>=1;
    for reflector in (0..10).rev() {
        if mask&actor_mask(reflector)==0 {continue;}
        loop {
            c.damage.execution.attacker=reflector;battle_attack_effect::choose_targets(reflector,a,am,s,p,c,rng,table)?;
            let Some(target)=(0..10).rev().find(|&slot|s.targets&actor_mask(slot)!=0) else{break;};
            let index=if p.deferred&128!=0 {reflector}else {let count=usize::from(p.deferred);p.deferred=p.deferred.wrapping_sub(1);count};map_target(c,index,target)?;
            if battle_action_damage::check_target(target,a,m,p,c,rng,table)?==Outcome::Hit {
                c.failed=0;battle_attack_history::record(reflector,&mut a[target].previous,&mut e[target].history,Action {command:c.original_command,attack:c.last_attack,item:c.last_item,element:c.damage.magic[1]},rng,table)?;
                // Unlike the ordinary hit path, reflection calls the primary
                // dispatcher directly and does not enter TWUE's dual wrapper.
                battle_hit_resolution::queue_effects(reflector,target,a,e,c,rng,table,items,false)?;
                if c.failed==0 {c.params[7]|=actor_mask(reflector);battle_target_damage::calculate(a,target,&mut c.damage,g,rng,table)?;}
            }
            if p.deferred&128!=0 {break;}
        }
    }
    // The target adds INY/STY at C2/350F: reset the counted-hit source byte.
    p.deferred=255;c.damage.execution.attacker=original;
    if p.ba&16==0 {c.params[7]=actor_mask(original);c.params[5]=0;c.params[6]=0;}
    battle_target_damage::apply(a,&mut c.damage,g)?;battle_attack_effect::copy_parameters(c)
}
pub fn cast_and_reflect(a:&mut[Actor;10],e:&mut[Extra;10],m:&mut[Meta;10],am:&[ActorMeta;10],s:&mut EffectState,p:&mut Prefix,c:&mut Context,g:&mut Graphics,rng:&mut BattleRng,table:&RngTable,items:&ItemTable,t:&Tables,cast:&mut battle_cast::State)->Result<(),String> {
    battle_cast::run(a,e,m,am,s,p,c,g,rng,table,items,t,cast)?;
    if c.damage.magic[2]&1==0 && c.params[3]!=0 {run(c.params[3],a,e,m,am,s,p,c,g,rng,table,items)?;}
    Ok(())
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;let items=ItemTable::load(root)?;let t=Tables::load(root)?;let mut rows=String::from("case,combined,attacker,mask,before,after\n");
    for case in 0..6144 {let combined=case>=4096;let (mut a,mut e,mut m,mut am,mut s,mut p,mut c,mut g,mut rng)=battle_attack_effect::fixture(case);let mut cast=battle_cast::State {stat:(case*17) as u8,level:(case*31) as u8,relic1:std::array::from_fn(|i|[0,1,2,3,255][(case+i)%5])};
        if !combined {p.deferred=if case%3==0 {(case%10) as u8}else{255};c.damage.magic[9]=if case%7==0 {164}else{crate::battle_target_effects::NO_OPS[case%42]};c.secondary_effect=if case%11==0 {164}else{0};}
        else {if case%4==0 {c.damage.magic[3]|=8;}if case%3!=0 {a[c.damage.execution.attacker].update|=0x400;}if case%5==0 {c.damage.magic[2]=0;c.damage.magic[3]=0;c.damage.magic[4]=32;s.cached_cost=0;s.special_index=0;p.deferred=255;for i in 0..10 {a[i].status12=0;a[i].status34=128;am[i].presence=1;}}}
        let mask=if case%9==0 {0}else if case%5==0 {actor_mask((case+4)%10)}else{c.params[3]&0x3f0f};let before=battle_cast::snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng,&cast);
        if combined {cast_and_reflect(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&table,&items,&t,&mut cast)?;}else{run(mask,&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&table,&items)?;}
        writeln!(rows,"{case},{},{},{mask},{before},{}",u8::from(combined),c.damage.execution.attacker,battle_cast::snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng,&cast)).unwrap();
    }
    fs::write(dir.join("reflect.csv"),rows).map_err(|e|e.to_string())
}

#[cfg(test)]mod tests {
    use super::*;
    fn fixture()->([Actor;10],[Extra;10],[Meta;10],[ActorMeta;10],EffectState,Prefix,Context,Graphics,BattleRng) {
        let (mut a,e,mut m,mut am,mut s,mut p,mut c,_,_)=battle_attack_effect::fixture(1);
        for i in 0..10 {a[i].hp=100;a[i].max_hp=100;a[i].status12=0;a[i].status34=0;a[i].monster_flags=0;a[i].flags1=0;a[i].absorb=0;a[i].null=0;a[i].half=0;a[i].weak=0;a[i].previous=127;a[i].control=255;m[i].physical_block=0;m[i].magical_block=0;am[i].presence=1;am[i].charm=255;}
        s.targettable=1;s.valid=1;s.revived=0x3f0f;s.jump_seize=0;s.remove_jump=false;s.enemy_characters=0;p.battle_type=0;p.ba=0;p.deferred=255;p.target_count=1;c.damage.magic[1]=0;c.damage.magic[2]=0;c.damage.magic[3]=0;c.damage.magic[4]=32;c.damage.magic[6]=10;c.damage.magic[7]=0;c.damage.magic[8]=255;c.damage.magic[9]=0;c.damage.magic[10..14].fill(0);c.damage.enabled=0;c.damage.base=100;c.damage.multiplier=0;c.damage.flags_b2=0x200;c.secondary_effect=0;
        (a,e,m,am,s,p,c,Graphics::default(),BattleRng::default())
    }
    fn execute(mask:u16,a:&mut[Actor;10],e:&mut[Extra;10],m:&[Meta;10],am:&[ActorMeta;10],s:&mut EffectState,p:&mut Prefix,c:&mut Context,g:&mut Graphics,rng:&mut BattleRng) {run(mask,a,e,m,am,s,p,c,g,rng,&RngTable::identity(),&ItemTable::load(Path::new("extracted/twue-3.07")).unwrap()).unwrap();}
    #[test]fn empty_mask_still_queues_counted_animation_halves_base_and_resets_counter() {
        let (mut a,mut e,m,am,mut s,mut p,mut c,mut g,mut rng)=fixture();p.deferred=0;g.command=0xabcd;g.parameter=0xef34;c.damage.animation=77;
        execute(0,&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng);
        assert_eq!((c.damage.base,p.deferred,c.parameter_tail,g.command,g.parameter),(50,255,16,0xab15,0xef4d));assert_eq!(&g.script[..4],&[21,171,77,239]);assert_eq!(&c.params[..5],&[65535;5]);assert_eq!((c.params[7],a[0].hp),(2,100));
    }
    #[test]fn reflector_records_history_while_final_application_restores_original_caster() {
        let (mut a,mut e,m,am,mut s,mut p,mut c,mut g,mut rng)=fixture();execute(256,&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng);
        assert_eq!((a[0].hp,a[0].previous,a[0].retaliation_target,e[0].history.command_source),(50,132,2,8));assert_eq!((c.damage.execution.attacker,c.params[2],c.params[7],p.deferred),(1,0xff00,256,255));assert_eq!(c.parameter_tail,16);
    }
    #[test]fn reflected_primary_dispatch_does_not_execute_secondary_steal() {
        let (mut a,mut e,m,am,mut s,mut p,mut c,mut g,mut rng)=fixture();s.valid=256;s.targettable=256;c.secondary_effect=164;e[4].effect.items=[4,5];e[1].effect.obtained=255;a[1].level=255;
        execute(2,&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng);assert_eq!(e[4].effect.items,[4,5]);assert_eq!(e[1].effect.obtained,255);assert_eq!(a[4].hp,50);
    }
    #[test]fn counted_hits_fill_reverse_mapping_and_accumulate_before_single_application() {
        let (mut a,mut e,m,am,mut s,mut p,mut c,mut g,mut rng)=fixture();p.deferred=2;execute(256,&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng);
        assert_eq!((a[0].hp,a[0].pending_set&128,p.deferred,c.params[0],c.params[1],c.params[7]),(0,128,255,0,0xff00,2));assert_eq!((c.params[5],c.params[6]),(0,0));assert_eq!(g.script[0],21);
    }
    #[test]fn patched_counted_path_preserves_existing_reflection_flag() {
        let (mut a,mut e,m,am,mut s,mut p,mut c,mut g,mut rng)=fixture();p.deferred=0;p.ba=16;execute(256,&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng);assert_eq!(p.ba&16,16);assert_eq!(c.params[7],256);assert_eq!(g.script[0],21);
    }
    #[test]fn no_eligible_rebound_target_still_resets_positive_counter() {
        let (mut a,mut e,m,am,mut s,mut p,mut c,mut g,mut rng)=fixture();p.deferred=3;s.valid=0;s.targettable=0;execute(256,&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng);assert_eq!((p.deferred,c.parameter_tail,a[0].hp),(255,16,100));assert_eq!(&c.params[..5],&[65535;5]);
    }
}
