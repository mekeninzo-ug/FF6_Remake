//! CalcItemEffect C2/2A37..2B68, including all eight ToolsEffectTbl entries.
use crate::{battle_target_damage::{Actor,Context as Damage},battle_cast,battle_rng::{BattleRng,RngTable},equipment::ItemTable};
use std::{fs,path::Path};
#[derive(Clone,Debug)]pub struct Context {pub tools_or_throw:bool,pub strength:[u8;10],pub sketch:u8}
pub fn prepare(item:u8,context:&Context,a:&[Actor;10],damage:&mut Damage,cast:&mut battle_cast::State,items:&ItemTable,rng:&mut BattleRng,table:&RngTable)->Result<(),String> {
    damage.magic=[0;14];cast.stat=0;cast.level=0;damage.magic[2]=0x21;damage.magic[3]=0x22;damage.magic[4]=0x20;
    if context.tools_or_throw {
        let actor=damage.execution.attacker;if actor>=10 {return Err("Item setup attacker outside ten actors".into());}cast.stat=context.strength[actor];
        let level_actor=if context.sketch&128==0 {if context.sketch&1!=0 || context.sketch>=20 {return Err("Sketch level source requires an aligned actor offset".into());}usize::from(context.sketch/2)}else{actor};cast.level=a[level_actor].level;
    }
    let r=items.record(item);damage.magic[6]=r[20];damage.magic[1]=r[15];
    if !context.tools_or_throw {
        damage.magic[2]&=!1;if r[27]&128==0 {damage.magic[9]=r[27].wrapping_mul(2).wrapping_add(0x90);}
        damage.magic[10..14].copy_from_slice(&r[21..25]);let flags=r[19];
        if flags&128!=0 {damage.magic[4]|=128;}if flags&32!=0 {damage.magic[4]|=4;}if flags&16!=0 {damage.magic[3]|=128;}if flags&24!=0 {damage.magic[4]|=1;}if flags&2!=0 {damage.magic[2]|=8;}if damage.magic[10]&128!=0 {damage.magic[2]|=12;}
    }
    if item==174 {damage.magic[10]|=16;}else if item==175 {damage.magic[11]|=4;}
    if r[0]&7==0 {
        damage.magic[2]&=!32;damage.magic[4]&=!32;damage.magic[8]=r[21];
        match item.wrapping_sub(163)&7 {
            0=>damage.magic[11]=32,
            1|2=>{},
            3=>{if rng.next(table)&3==0 {damage.animation=8;damage.magic[6]=0;damage.magic[10]|=128;damage.magic[4]=16;damage.magic[2]|=2;}damage.magic[2]|=32;},
            4=>damage.magic[9]=172,
            5=>damage.magic[2]|=32,
            6=>damage.magic[9]=174,
            7=>damage.magic[2]|=64,
            _=>unreachable!(),
        }
    }
    damage.execution.attack_flags=u16::from(damage.magic[2])|(u16::from(damage.magic[3])<<8);Ok(())
}
fn snapshot(a:&[Actor;10],e:&[crate::battle_hit_resolution::Extra;10],m:&[crate::battle_action_damage::Meta;10],am:&[crate::battle_attack_effect::ActorMeta;10],s:&crate::battle_attack_effect::State,p:&crate::battle_action_damage::Prefix,c:&crate::battle_hit_resolution::Context,g:&crate::battle_damage_exec::Graphics,rng:&BattleRng,cast:&battle_cast::State,context:&Context)->String {
    use std::fmt::Write;let mut out=battle_cast::snapshot(a,e,m,am,s,p,c,g,rng,cast);for (i,v) in context.strength.iter().enumerate() {write!(out,"{:06X}{v:02X}",0x7e3b2c+i*2).unwrap();}write!(out,"7E3417{:02X}",context.sketch).unwrap();out
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let items=ItemTable::load(root)?;let table=RngTable::load(root)?;let mut rows=String::from("case,item,tools,attacker,combined,before,after\n");
    let tables=crate::battle_action_damage::Tables::load(root)?;
    for case in 0..6144 {
        let combined=case>=5120;let (mut a,mut e,mut m,am,mut s,mut p,mut c,mut g,mut rng)=crate::battle_attack_effect::fixture(case);
        c.damage.execution.attacker=if combined {case%10}else{case/512};
        let context=Context {tools_or_throw:!combined && case/256%2!=0,strength:std::array::from_fn(|i|(case*31+i*17) as u8),sketch:if case%11==0 {255}else{(case%10*2) as u8}};
        let mut cast=battle_cast::State {stat:(case*17) as u8,level:(case*31) as u8,relic1:std::array::from_fn(|i|(case+i) as u8)};
        let item=if combined {[232,233,234,235,236,237,240,241,242,243,244,245,246,247,248,250][case%16]}else{case as u8};
        if combined {c.damage.command=1;c.original_command=1;c.damage.animation=item;c.damage.gfx_kind=6;c.last_item=item;c.last_attack=255;c.damage.enabled=0;c.damage.flags_b2=0x242;s.cached_cost=0;s.special_index=1;s.cursor=items.record(item)[14];p.ba=0;p.deferred=255;c.secondary_effect=0;}
        let before=snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng,&cast,&context);
        prepare(item,&context,&a,&mut c.damage,&mut cast,&items,&mut rng,&table)?;
        if combined {crate::battle_reflect::cast_and_reflect(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&table,&items,&tables,&mut cast)?;}
        writeln!(rows,"{case},{item},{},{},{},{before},{}",u8::from(context.tools_or_throw),c.damage.execution.attacker,u8::from(combined),snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng,&cast,&context)).unwrap();
    }
    fs::write(dir.join("item-effect.csv"),rows).map_err(|e|e.to_string())
}

#[cfg(test)]mod tests {
    use super::*;
    fn tables()->ItemTable {ItemTable::load(Path::new("extracted/twue-3.07")).unwrap()}
    fn context(tools_or_throw:bool)->Context {Context {tools_or_throw,strength:[0;10],sketch:255}}
    fn cast()->battle_cast::State {battle_cast::State {stat:255,level:255,relic1:[7;10]}}
    #[test]fn normal_item_clears_all_working_properties_but_preserves_base_and_animation() {
        let (a,_,_,_,_,_,mut c,_,mut rng)=crate::battle_attack_effect::fixture(1);c.damage.magic=[255;14];c.damage.base=1234;c.damage.animation=42;let mut cast=cast();let seed=rng.index;
        prepare(250,&context(false),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((c.damage.magic[0],c.damage.magic[2],c.damage.magic[3],c.damage.magic[4],c.damage.magic[6],c.damage.magic[9]),(0,32,34,32,1,148));assert_eq!((cast.stat,cast.level,c.damage.base,c.damage.animation,rng.index),(0,0,1234,42,seed));assert_eq!(cast.relic1,[7;10]);
    }
    #[test]fn throwing_uses_current_strength_and_sketch_source_level() {
        let (mut a,_,_,_,_,_,mut c,_,mut rng)=crate::battle_attack_effect::fixture(1);a[1].level=13;a[3].level=77;let mut ctx=context(true);ctx.strength[1]=201;ctx.sketch=6;let mut cast=cast();
        prepare(0,&ctx,&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((cast.stat,cast.level,c.damage.magic[2],c.damage.magic[3],c.damage.magic[4]),(201,77,33,34,32));assert_eq!(&c.damage.magic[9..14],&[0;5]);
    }
    #[test]fn unused_sketch_offset_is_not_read_for_normal_items() {
        let (a,_,_,_,_,_,mut c,_,mut rng)=crate::battle_attack_effect::fixture(1);let mut ctx=context(false);ctx.sketch=1;let mut cast=cast();prepare(250,&ctx,&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((cast.stat,cast.level),(0,0));ctx.tools_or_throw=true;assert!(prepare(0,&ctx,&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).is_err());
    }
    #[test]fn mp_restoration_sets_shared_restore_flag_and_phoenix_sets_resurrection_bits() {
        let (a,_,_,_,_,_,mut c,_,mut rng)=crate::battle_attack_effect::fixture(1);let mut cast=cast();prepare(235,&context(false),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((c.damage.magic[3],c.damage.magic[4]),(0xa2,0x21));prepare(237,&context(false),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((c.damage.magic[3],c.damage.magic[4]),(0xa2,0xa1));prepare(240,&context(false),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((c.damage.magic[2],c.damage.magic[4],c.damage.magic[10]),(0x2c,0xa5,128));
    }
    #[test]fn invis_and_shadow_edges_add_status_after_throw_branch() {
        let (a,_,_,_,_,_,mut c,_,mut rng)=crate::battle_attack_effect::fixture(1);let mut cast=cast();prepare(174,&context(true),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((c.damage.magic[10],c.damage.magic[11]),(16,0));prepare(175,&context(true),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((c.damage.magic[10],c.damage.magic[11]),(0,4));
    }
    #[test]fn all_eight_raw_tool_dispatch_entries_preserve_their_source_setup() {
        let (a,_,_,_,_,_,mut c,_,mut rng)=crate::battle_attack_effect::fixture(1);let mut cast=cast();for item in 163..=170 {rng.index=1;prepare(item,&context(true),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!(c.damage.magic[8],tables().record(item)[21]);match item {163=>assert_eq!((c.damage.magic[2],c.damage.magic[11]),(1,32)),164|165=>assert_eq!((c.damage.magic[2],c.damage.magic[4]),(1,0)),166|168=>assert_eq!(c.damage.magic[2],33),167=>assert_eq!(c.damage.magic[9],172),169=>assert_eq!(c.damage.magic[9],174),170=>assert_eq!(c.damage.magic[2],65),_=>unreachable!()}}
    }
    #[test]fn chainsaw_death_branch_consumes_one_byte_and_changes_animation() {
        let (a,_,_,_,_,_,mut c,_,mut rng)=crate::battle_attack_effect::fixture(1);let mut cast=cast();c.damage.animation=42;rng.index=255;prepare(166,&context(true),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((rng.index,c.damage.animation,c.damage.magic[6],c.damage.magic[10],c.damage.magic[2],c.damage.magic[4]),(0,8,0,128,35,16));c.damage.animation=42;rng.index=0;prepare(166,&context(true),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();assert_eq!((rng.index,c.damage.animation,c.damage.magic[6],c.damage.magic[10]),(1,42,tables().record(166)[20],0));
    }
    #[test]fn prepared_potion_and_ether_reach_connected_hp_and_mp_application() {
        let root=Path::new("extracted/twue-3.07");for item in [232,235] {let (mut a,mut e,mut m,mut am,mut s,mut p,mut c,mut g,mut rng)=crate::battle_attack_effect::fixture(1);for i in 0..10 {a[i].status12=0;a[i].status34=0;a[i].monster_flags=0;a[i].flags1=0;a[i].absorb=0;a[i].null=0;a[i].half=0;a[i].weak=0;a[i].control=255;a[i].update|=0x400;am[i].presence=1;}a[1].hp=900;a[1].max_hp=1000;a[1].mp=10;a[1].max_mp=100;c.damage.execution.attacker=0;c.damage.command=1;c.original_command=1;c.damage.enabled=0;c.damage.multiplier=0;c.damage.flags_b2=0x242;c.secondary_effect=0;s.targets=2;s.targettable=2;s.valid=2;s.revived=0x3f0f;s.jump_seize=0;s.cached_cost=0;s.cursor=tables().record(item)[14];s.special_index=1;p.deferred=255;p.ba=0;let mut cast=cast();prepare(item,&context(false),&a,&mut c.damage,&mut cast,&tables(),&mut rng,&RngTable::identity()).unwrap();crate::battle_reflect::cast_and_reflect(&mut a,&mut e,&mut m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&RngTable::identity(),&tables(),&crate::battle_action_damage::Tables::load(root).unwrap(),&mut cast).unwrap();assert_eq!((a[1].hp,a[1].mp),if item==232 {(950,10)}else{(900,60)});}
    }
}
