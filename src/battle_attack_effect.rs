//! Connected CalcAttackEffect C2/3292..3482, including selection, cover and MP.
use crate::{battle_action_damage::{self,Meta,Prefix,Tables},battle_hit_resolution::{Context,Extra},battle_target_damage::Actor,battle_damage_exec::Graphics,battle_rng::{BattleRng,RngTable},battle_targets::{self,actor_mask},battle_attack_prep,battle_resources,equipment::ItemTable};
use std::{fs,path::Path};

#[derive(Clone,Copy,Debug)]
pub struct ActorMeta {
    pub presence:u8,pub seize:u16,pub love:u8,pub charm:u8,
    pub special:u8,pub special_animation:u8,
}
impl Default for ActorMeta {fn default()->Self {Self {presence:1,seize:65535,love:255,charm:255,special:0,special_animation:0}}}
#[derive(Clone,Debug)]
pub struct State {
    pub targets:u16,pub mask_flags:u8,pub cursor:u8,pub targettable:u16,pub valid:u16,
    pub revived:u16,pub jump_seize:u16,pub remove_jump:bool,pub enemy_characters:u8,
    pub pincer_left:u8,pub pincer_right:u8,pub first_previous:u8,
    pub flags_b0:u8,pub remaining:u8,pub dragon_horn:u8,pub modification_high:u8,
    pub cached_cost:u16,pub old_targets:u16,pub backup:u16,pub special_index:u8,
}
impl Default for State {fn default()->Self {Self {targets:0,mask_flags:0,cursor:0x40,targettable:0x3f0f,valid:0x3f0f,revived:0x3f0f,jump_seize:0,remove_jump:false,enemy_characters:0,pincer_left:0,pincer_right:0,first_previous:0,flags_b0:0,remaining:0,dragon_horn:0,modification_high:255,cached_cost:0,old_targets:0,backup:0,special_index:0}}}

fn check_cover(slot:usize,a:&[Actor;10],m:&[Meta;10],am:&[ActorMeta;10],c:&mut Context,choice:&mut(Option<usize>,u16)) {
    let d=am[slot];let control=u16::from(m[slot].control_target)|(u16::from(a[slot].control)<<8);
    if d.presence&1==0 || control&0x8000==0 || d.seize&0x8000==0 || a[slot].status12&0xa0d2!=0 || a[slot].status34&0x3210!=0 {return;}
    c.params[3]|=actor_mask(slot);if a[slot].hp>=choice.1 {*choice=(Some(slot),a[slot].hp);}
}
fn cover(a:&[Actor;10],m:&[Meta;10],am:&[ActorMeta;10],s:&mut State,c:&mut Context)->Result<(),String> {
    if c.damage.flags_b2&2!=0 || s.targets==0 {return Ok(());}
    let original=(0..10).rev().find(|&i|s.targets&actor_mask(i)!=0).ok_or("Cover requires a valid actor target bit")?;
    let attacker=c.damage.execution.attacker;let mut choice=(None,0u16);let mut recorded=original;
    let love=am[original].love;
    if love&128==0 {
        if love&1!=0 || love>=20 {return Err("Love Token relationship must identify an aligned actor offset".into());}
        check_cover(usize::from(love/2),a,m,am,c,&mut choice);
        if let Some(slot)=choice.0 {recorded=slot;s.targets=actor_mask(slot);c.params[4]=(c.params[4]&0xff00)|original as u16;}
    }
    if a[original].status12&0x200==0 || a[original].status12&16!=0 || am[original].seize&0x8000==0 {return Ok(());}
    let allowed=(if original<4 {15}else{0x3f00})&!(actor_mask(original)|actor_mask(attacker));
    for slot in (0..10).rev() {if m[slot].relic4&64!=0 && allowed&actor_mask(slot)!=0 {check_cover(slot,a,m,am,c,&mut choice);}}
    if choice.1!=0 && recorded==original {if let Some(slot)=choice.0 {s.targets=actor_mask(slot);c.params[4]=(c.params[4]&0xff00)|original as u16;}}
    Ok(())
}
fn init_graphics(attacker:usize,s:&mut State,p:&mut Prefix,c:&mut Context) {
    let mut flags:u8=if attacker>=4 {128}else{0};if s.targets&255==0 {flags|=64;}
    if (attacker<4 && s.targets&255==0) || (attacker>=4 && s.targets&255!=0) {p.ba|=2;}
    let prior=s.flags_b0;s.flags_b0&=!16;if prior&16==0 {flags|=16;}
    if s.remaining!=0 && s.dragon_horn!=0 {flags|=2;p.ba|=0x60;}
    c.params[0]=u16::from(flags)|((attacker as u16)<<8);c.params[1]=s.targets;c.params[2]=s.targets;
}
pub(crate) fn setup_old_targets(s:&mut State) {if s.modification_high&128==0 {s.targets=s.old_targets;}else{s.old_targets=s.targets;s.backup|=s.targets;}}
pub(crate) fn choose_targets(attacker:usize,a:&[Actor;10],am:&[ActorMeta;10],s:&mut State,p:&Prefix,c:&Context,rng:&mut BattleRng,table:&RngTable)->Result<(),String> {
    let previous=if c.parameter_tail==0 {s.first_previous}else{c.parameters[usize::from(c.parameter_tail)-15]};
    let target=battle_targets::Context {actor:attacker,restrictions:p.ba,cursor:s.cursor,actors:std::array::from_fn(|i|battle_targets::Actor {present:am[i].presence&1!=0,status12:a[i].status12,status34:a[i].status34,charm_attacker:am[i].charm}),targettable:s.targettable,valid:s.valid,revived:s.revived,jump_seize:s.jump_seize,remove_jump:s.remove_jump,enemy_characters:s.enemy_characters,battle_type:p.battle_type,pincer_left:s.pincer_left,pincer_right:s.pincer_right,previous_attacker:usize::from(previous),backup:s.backup};
    let mut selected=battle_targets::Selection {targets:s.targets,mask_flags:s.mask_flags};battle_targets::choose(&target,&mut selected,rng,table)?;s.targets=selected.targets;s.mask_flags=selected.mask_flags;    Ok(())
}
pub(crate) fn copy_parameters(c:&mut Context)->Result<(),String> {
    if c.parameter_tail&15!=0 {return Err("Graphics parameter cursor requires a sixteen-byte record".into());}
    let base=usize::from(c.parameter_tail);for (i,word) in c.params.iter().enumerate() {c.parameters[base+i*2..base+i*2+2].copy_from_slice(&word.to_le_bytes());}c.parameter_tail=c.parameter_tail.wrapping_add(16);Ok(())
}
fn append_attack(c:&mut Context,g:&mut Graphics) {c.damage.insertion=g.tail;g.command=u16::from(c.damage.gfx_kind)|(u16::from(c.damage.command)<<8);g.parameter=u16::from(c.damage.animation)|(u16::from(c.damage.weapon_length)<<8);g.append();}
pub fn run(a:&mut[Actor;10],e:&mut[Extra;10],m:&[Meta;10],am:&[ActorMeta;10],s:&mut State,p:&mut Prefix,c:&mut Context,g:&mut Graphics,rng:&mut BattleRng,table:&RngTable,items:&ItemTable,t:&Tables)->Result<(),String> {
    let attacker=c.damage.execution.attacker;if attacker>=10 {return Err("Attack effect attacker outside ten actors".into());}
    c.damage.execution.misses=0;c.facing=0;c.params=[0;8];
    choose_targets(attacker,a,am,s,p,c,rng,table)?;p.target_count=s.targets.count_ones() as u8;
    cover(a,m,am,s,c)?;init_graphics(attacker,s,p,c);
    setup_old_targets(s);
    let mut payment=battle_resources::State {mp:a[attacker].mp,cached_cost:s.cached_cost,update_flags:a[attacker].update,command:c.damage.command,message_flags:u16::from(c.damage.magic[7])|(u16::from(c.damage.magic[8])<<8),gfx_a2:c.params[1],gfx_a4:c.params[2]};
    let paid=battle_resources::pay(&mut payment,(attacker*2) as u8,&mut Vec::new());a[attacker].mp=payment.mp;a[attacker].update=payment.update_flags;s.cached_cost=payment.cached_cost;c.damage.command=payment.command;c.damage.magic[7]=payment.message_flags as u8;c.damage.magic[8]=(payment.message_flags>>8) as u8;c.params[1]=payment.gfx_a2;c.params[2]=payment.gfx_a4;
    if paid==battle_resources::Payment::Insufficient {append_attack(c,g);g.command=0x2802;g.append();return copy_parameters(c);}
    let before_flags=c.params[0] as u8;
    let mut prep=battle_attack_prep::State {magic:c.damage.magic,flags_b2:c.damage.flags_b2 as u8,flags_b3:(c.damage.flags_b2>>8) as u8,graphics_flags:before_flags,animation:c.damage.weapon_length,multiplier:c.damage.multiplier,base:c.damage.base};
    let mut effects=Vec::new();battle_attack_prep::prepare(battle_attack_prep::Context {special_index:s.special_index,special_property:am[attacker].special,special_animation:am[attacker].special_animation,status2:(a[attacker].status12>>8) as u8,status4:(a[attacker].status34>>8) as u8,targets:p.target_count,row_flags:a[attacker].flags1},&mut prep,&mut effects);
    if !effects.is_empty() {g.command=0x2506;g.append();c.params[0]=(c.params[0]&0xff00)|u16::from(prep.graphics_flags|(before_flags&16));copy_parameters(c)?;}
    c.params[0]=(c.params[0]&0xff00)|u16::from(prep.graphics_flags);c.damage.magic=prep.magic;c.damage.flags_b2=(c.damage.flags_b2&0xff00)|u16::from(prep.flags_b2);c.damage.weapon_length=prep.animation;c.damage.multiplier=prep.multiplier;c.damage.base=prep.base;c.damage.execution.attack_flags=u16::from(prep.magic[2])|(u16::from(prep.magic[3])<<8);
    battle_action_damage::run(a,e,m,p,c,g,rng,table,items,t)
}

pub(crate) fn snapshot(a:&[Actor;10],e:&[Extra;10],m:&[Meta;10],am:&[ActorMeta;10],s:&State,p:&Prefix,c:&Context,g:&Graphics,rng:&BattleRng)->String {
    use std::fmt::Write;let mut out=battle_action_damage::snapshot(a,e,m,p,c,g,rng);
    let mut put=|addr:u32,value:u16,width:usize| {for i in 0..width {write!(out,"{:06X}{:02X}",addr+i as u32,(value>>(i*8)) as u8).unwrap();}};
    for (slot,d) in am.iter().enumerate() {let x=slot as u32*2;for (base,v,width) in [(0x3aa0,u16::from(d.presence),1),(0x3358,d.seize,2),(0x336c,u16::from(d.love),1),(0x3395,u16::from(d.charm),1),(0x322d,u16::from(d.special),1),(0x3c81,u16::from(d.special_animation),1)] {put(0x7e0000+base+x,v,width);}}
    for (addr,v,width) in [(0xb8,s.targets,2),(0xbb,u16::from(s.cursor),1),(0xb0,u16::from(s.flags_b0),1),(0x7e3a46,u16::from(s.mask_flags),1),(0x7e3403,s.targettable&255,1),(0x7e2f46,s.targettable>>8,1),(0x7e3a78,s.valid,2),(0x7e3408,s.revived,2),(0x7e3f2c,s.jump_seize,2),(0x7e341b,if s.remove_jump {128}else{0},1),(0x7e3a40,u16::from(s.enemy_characters),1),(0x7e2eac,u16::from(s.pincer_left),1),(0x7e2ead,u16::from(s.pincer_right),1),(0x7e2c5f,u16::from(s.first_previous),1),(0x7e3a70,u16::from(s.remaining),1),(0x7e3a8e,u16::from(s.dragon_horn),1),(0x7e3415,u16::from(s.modification_high),1),(0x7e3a4c,s.cached_cost,2),(0x7e3a30,s.old_targets,2),(0x7e3a4e,s.backup,2),(0x7e3412,u16::from(s.special_index),1)] {put(addr,v,width);}
    out
}
pub(crate) fn fixture(case:usize)->([Actor;10],[Extra;10],[Meta;10],[ActorMeta;10],State,Prefix,Context,Graphics,BattleRng) {
    let (mut a,e,mut m,p,mut c,g,rng)=battle_action_damage::fixture(case);let attacker=c.damage.execution.attacker;
    let mut am=std::array::from_fn(|slot|ActorMeta {presence:if (case+slot)%11==0 {0}else{1},seize:if (case+slot)%13==0 {0}else{65535},love:255,charm:if (case+slot)%17==0 {0}else{255},special:[0,1,0x17,0x20,0x2f,0x30,0x31,0x32][(case+slot)%8],special_animation:(case+slot) as u8});
    let mut s=State {targets:c.params[2],mask_flags:(case%4) as u8,cursor:[0,2,4,0x40,0x2c,0x60][case%6],targettable:0x3f0f,valid:0x3f0f,revived:0x3f0f,jump_seize:if case%17==0 {0x0101}else{0},remove_jump:case%2==0,enemy_characters:if case%19==0 {1}else{0},pincer_left:7,pincer_right:56,first_previous:(case%10) as u8,flags_b0:if case%2==0 {16}else{0},remaining:(case%3) as u8,dragon_horn:if case%7==0 {1}else{0},modification_high:if case%11==0 {0}else{255},cached_cost:match case%6 {0=>65534,1=>5,_=>0},old_targets:actor_mask((case+3)%10),backup:actor_mask((case+7)%10),special_index:if case%3==0 {6}else{0}};
    let mut p=p;
    if case%8==0 {for i in 0..4 {a[i].status12=0;a[i].status34=0;a[i].hp=100;am[i].presence=1;a[i].control=255;am[i].seize=65535;m[i].relic4|=64;}let original=if attacker==0 {1}else{0};a[original].status12=0x200;if case%16==0 {am[original].love=if attacker==2 {6}else{4};}s.targets=actor_mask(original);s.cursor=0x40;p.ba=0;c.damage.flags_b2&=!2;}
    // Special Reflect removal sets mode 04; avoid the nonexistent mode 0C.
    if s.special_index==6 {c.damage.magic[4]&=!12;}
    (a,e,m,am,s,p,c,g,rng)
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;let items=ItemTable::load(root)?;let t=Tables::load(root)?;let mut rows=String::from("case,attacker,before,after\n");
    for case in 0..4096 {let (mut a,mut e,m,am,mut s,mut p,mut c,mut g,mut rng)=fixture(case);let before=snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng);run(&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng,&table,&items,&t)?;writeln!(rows,"{case},{},{before},{}",c.damage.execution.attacker,snapshot(&a,&e,&m,&am,&s,&p,&c,&g,&rng)).unwrap();}
    fs::write(dir.join("attack-effect.csv"),rows).map_err(|e|e.to_string())
}

#[cfg(test)]mod tests {
    use super::*;
    fn execute(a:&mut[Actor;10],e:&mut[Extra;10],m:&[Meta;10],am:&[ActorMeta;10],s:&mut State,p:&mut Prefix,c:&mut Context,g:&mut Graphics,rng:&mut BattleRng) {
        let root=Path::new("extracted/twue-3.07");run(a,e,m,am,s,p,c,g,rng,&RngTable::identity(),&ItemTable::load(root).unwrap(),&Tables::load(root).unwrap()).unwrap();
    }
    #[test]fn love_token_retarget_precedes_true_knight_but_records_other_eligible_coverers() {
        let (mut a,_,m,am,mut s,_,mut c,_,_)=fixture(0);c.params=[0;8];a[3].hp=900;
        cover(&a,&m,&am,&mut s,&mut c).unwrap();assert_eq!((s.targets,c.params[3],c.params[4]),(4,12,1));
    }
    #[test]fn cover_reads_shared_control_attacker_and_seize_high_bytes() {
        let (mut a,_,m,mut am,mut s,_,mut c,_,_)=fixture(0);c.params=[0;8];am[3].presence=0;a[2].control=0;
        cover(&a,&m,&am,&mut s,&mut c).unwrap();assert_eq!(s.targets,2);
        a[2].control=255;am[2].seize=255;cover(&a,&m,&am,&mut s,&mut c).unwrap();assert_eq!(s.targets,2);
        am[2].seize=65535;cover(&a,&m,&am,&mut s,&mut c).unwrap();assert_eq!(s.targets,4);
    }
    #[test]fn graphics_init_sets_attacker_friendliness_swirl_and_dragon_horn_flags() {
        let (_,_,_,_,mut s,mut p,mut c,_,_)=fixture(1);s.targets=1;s.flags_b0=0;s.remaining=1;s.dragon_horn=1;p.ba=0;
        init_graphics(4,&mut s,&mut p,&mut c);assert_eq!((c.params[0],c.params[1],c.params[2],p.ba),(0x0492,1,1,0x62));
        s.targets=256;s.flags_b0=16;s.remaining=0;p.ba=0;init_graphics(0,&mut s,&mut p,&mut c);assert_eq!((c.params[0],p.ba,s.flags_b0),(64,2,0));
    }
    #[test]fn insufficient_mp_queues_animation_message_and_empty_target_parameters() {
        let (mut a,mut e,m,am,mut s,mut p,mut c,mut g,mut rng)=fixture(1);c.damage.execution.attacker=4;a[4].mp=0;s.cached_cost=1;let tail=g.tail;
        execute(&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng);
        assert_eq!((a[4].mp,s.cached_cost,c.damage.command,c.params[1],c.params[2],c.parameter_tail,g.tail),(0,0,18,0,0,16,tail+8));
        assert_eq!(&g.script[usize::from(tail)+4..usize::from(tail)+6],&[2,40]);assert_eq!(c.damage.magic[7]&2,0);
    }
    #[test]fn runic_copies_parameters_before_clearing_swirl_even_when_damage_is_deferred() {
        let (mut a,mut e,m,am,mut s,mut p,mut c,mut g,mut rng)=fixture(1);s.cached_cost=0;s.special_index=0;s.flags_b0=0;s.remaining=0;c.damage.magic[9]=0;c.damage.flags_b2&=!64;p.deferred=0;let tail=g.tail;
        execute(&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng);
        assert_eq!((c.parameter_tail,g.tail),(16,tail+4));assert_ne!(c.parameters[0]&16,0);assert_eq!(c.params[0]&16,0);assert_eq!(&g.script[usize::from(tail)..usize::from(tail)+2],&[6,37]);
    }
    #[test]fn connected_payment_and_damage_preserve_attacker_and_target_ownership() {
        let (mut a,mut e,mut m,am,mut s,mut p,mut c,mut g,mut rng)=fixture(1);s.cached_cost=5;s.special_index=0;s.targets=256;s.cursor=0x40;a[1].mp=30;a[1].status12=0;a[1].status34=0;a[1].flags1=0;a[4].status12=0;a[4].status34=0;a[4].monster_flags=0;a[4].flags1=0;a[4].absorb=0;a[4].null=0;a[4].half=0;a[4].weak=0;m[4].physical_block=0;m[4].magical_block=0;m[4].relic4=0;c.damage.magic[1]=0;c.damage.magic[7]=0;c.damage.magic[2]=1;c.damage.magic[3]=0;c.damage.magic[4]=32;c.damage.magic[6]=10;c.damage.magic[8]=255;c.damage.magic[9]=0;c.damage.magic[10..14].fill(0);c.damage.flags_b2=64;c.damage.base=20;c.damage.multiplier=0;p.deferred=255;p.ba=0;
        execute(&mut a,&mut e,&m,&am,&mut s,&mut p,&mut c,&mut g,&mut rng);
        assert_eq!((a[1].mp,a[1].update&128,s.cached_cost,c.parameter_tail),(25,128,0,16));assert!(a[4].hp<300);assert_eq!(&c.parameters[2..4],&[0,1]);assert_eq!(e[4].history.command,c.original_command);
    }
}
