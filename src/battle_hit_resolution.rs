//! CalcDmg's tail C2/343C..3482 for confirmed RTS entries and patched Steal.
use crate::{battle_target_damage::{self,Actor,Context as DamageContext},battle_attack_history::{self,Action,History,Lore},battle_status_requests::{self,Changes},battle_damage_exec::Graphics,battle_rng::{BattleRng,RngTable},battle_targets::actor_mask};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug)]pub struct Extra {pub effect:crate::battle_target_effects::Actor,pub history:History,pub allowed12:u16,pub allowed34:u16,pub pending_set34:u16,pub pending_clear34:u16}
#[derive(Clone,Debug)]pub struct Context {
    pub damage:DamageContext,pub effects:crate::battle_target_effects::State,pub secondary_effect:u8,pub multiplier_high:u8,pub params:[u16;8],pub facing:u16,pub zinger_attacker:u8,pub failed:u16,
    pub original_command:u8,pub last_attack:u8,pub last_item:u8,pub lore:Lore,pub parameter_tail:u8,pub parameters:[u8;256],
}
pub fn run(a:&mut[Actor;10],extra:&mut[Extra;10],c:&mut Context,g:&mut Graphics,rng:&mut BattleRng,table:&RngTable)->Result<(),String> {
    if !crate::battle_target_effects::supported(c.damage.magic[9]) {return Err(format!("Target special-effect entry {:02X} needs its native handler",c.damage.magic[9]));}
    if !crate::battle_target_effects::supported(c.secondary_effect) {return Err(format!("Secondary target special-effect entry {:02X} needs its native handler",c.secondary_effect));}
    if c.parameter_tail&15!=0 {return Err("Graphics parameter cursor must identify an aligned sixteen-byte record".into());}
    if c.damage.magic[4]&12==12 {return Err("Status mode 0C has no source StatusModTbl entry".into());}
    let attacker=c.damage.execution.attacker;if attacker>=10 {return Err("Hit resolution attacker outside ten slots".into());}
    for slot in (0..10).rev() {
        let mask=actor_mask(slot);let selected=c.params[2]&mask!=0;c.params[2]&=!mask;if !selected {continue;}
        if c.facing&mask!=0 {let value=(u16::from(c.damage.multiplier)|(u16::from(c.multiplier_high)<<8)).wrapping_add(1);c.damage.multiplier=value as u8;c.multiplier_high=(value>>8) as u8;}
        battle_attack_history::record(attacker,&mut a[slot].previous,&mut extra[slot].history,Action {command:c.original_command,attack:c.last_attack,item:c.last_item,element:c.damage.magic[1]},rng,table)?;
        if c.zinger_attacker==(slot*2) as u8 {continue;}
        c.failed=0;
        let mut t=battle_status_requests::Target {current12:a[slot].status12,current34:a[slot].status34,pending:Changes {set12:a[slot].pending_set,set34:extra[slot].pending_set34,clear12:a[slot].pending_clear,clear34:extra[slot].pending_clear34},allowed12:extra[slot].allowed12,allowed34:extra[slot].allowed34,monster_flags:a[slot].monster_flags as u8,mask,failed_mask:c.failed,miss_targets:c.damage.execution.misses};
        battle_status_requests::queue(&c.damage.magic,(c.damage.flags_b2>>8) as u8,&mut t)?;
        a[slot].pending_set=t.pending.set12;a[slot].pending_clear=t.pending.clear12;extra[slot].pending_set34=t.pending.set34;extra[slot].pending_clear34=t.pending.clear34;c.failed=t.failed_mask;c.damage.execution.misses=t.miss_targets;
        let levels=std::array::from_fn(|i|a[i].level);let mut effect_actors=std::array::from_fn(|i|extra[i].effect);
        let primary=c.damage.magic[9];
        crate::battle_target_effects::dispatch(primary,attacker,slot,&levels,&mut effect_actors,&mut extra[slot].history.command,&mut c.effects,rng,table)?;
        if c.secondary_effect!=primary {crate::battle_target_effects::dispatch(c.secondary_effect,attacker,slot,&levels,&mut effect_actors,&mut extra[slot].history.command,&mut c.effects,rng,table)?;}
        for i in 0..10 {extra[i].effect=effect_actors[i];}
        if c.failed!=0 {continue;}
        c.params[2]|=mask;battle_target_damage::calculate(a,slot,&mut c.damage,g,rng,table)?;
    }
    battle_target_damage::apply(a,&mut c.damage,g)?;
    c.lore.attack_flags=c.damage.magic[3];c.lore.attack=c.damage.animation;
    battle_attack_history::learn(&mut c.lore,&std::array::from_fn(|slot|a[slot].status12),&std::array::from_fn(|slot|a[slot].status34))?;
    if c.params[2]==0 {c.damage.magic[7]&=253;}
    let base=usize::from(c.parameter_tail);for (index,word) in c.params.iter().enumerate() {c.parameters[base+index*2..base+index*2+2].copy_from_slice(&word.to_le_bytes());}c.parameter_tail=c.parameter_tail.wrapping_add(16);Ok(())
}
fn snapshot(a:&[Actor;10],e:&[Extra;10],c:&Context,g:&Graphics,rng:&BattleRng)->String {
    use std::fmt::Write;let mut result=battle_target_damage::snapshot(a,&c.damage,g,rng);let mut put=|addr:u32,val:u16,width:usize|{for i in 0..width {write!(result,"{:06X}{:02X}",addr+i as u32,(val>>(i*8)) as u8).unwrap();}};
    for slot in 0..10 {let x=slot as u32*2;let h=e[slot].history;for (addr,v,width) in [(0x3290,u16::from(h.command_source),1),(0x3291,u16::from(h.attack_source),1),(0x32a4,u16::from(h.item_source),1),(0x32a5,u16::from(h.element_source),1),(0x3d48,u16::from(h.command),1),(0x3d49,u16::from(h.attack),1),(0x3d5c,u16::from(h.item),1),(0x3d5d,u16::from(h.element),1),(0x331c,e[slot].allowed12,2),(0x3330,e[slot].allowed34,2),(0x3de8,e[slot].pending_set34,2),(0x3e10,e[slot].pending_clear34,2)] {put(0x7e0000+addr+x,v,width);}}
    for (addr,val,width) in [(0x7e2f3d,u16::from(c.secondary_effect),1),(0xbd,u16::from(c.multiplier_high),1),(0x7e3a54,c.facing,2),(0x7e33f8,u16::from(c.zinger_attacker),1),(0x7e3a48,c.failed,2),(0x7e3a7c,u16::from(c.original_command),1),(0x7e3410,u16::from(c.last_attack),1),(0x7e3411,u16::from(c.last_item),1),(0x7e3007,u16::from(c.lore.strago_offset),1),(0x7e3a32,u16::from(c.parameter_tail),1)] {put(addr,val,width);}
    for i in 0..3 {put(0x7e1d29+i as u32,u16::from(c.lore.known[i]),1);put(0x7e3a84+i as u32,u16::from(c.lore.learned[i]),1);}for (i,v) in c.params.iter().enumerate() {put(0xa0+i as u32*2,*v,2);}for (i,v) in c.parameters.iter().enumerate() {put(0x7e2c6e+i as u32,u16::from(*v),1);}
    result.push_str(&crate::battle_target_effects::snapshot(&std::array::from_fn(|i|e[i].effect),&c.effects,&std::array::from_fn(|i|a[i].level),&std::array::from_fn(|i|e[i].history.command),rng));
    result
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,attacker,before,after,deliveries\n");
    for case in 0..4096usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let attacker=case%10;
        let mut magic=std::array::from_fn(|_|next());magic[6]=if case%13==0 {0}else{next()};magic[9]=if case%7==0 {164}else{crate::battle_target_effects::NO_OPS[case%crate::battle_target_effects::NO_OPS.len()]};magic[4]=(magic[4]&!12)|[0,4,8][case/3%3];
        let damage=DamageContext {execution:battle_damage_exec_context(attacker,magic,&mut next),magic,base:u16::from_le_bytes([next(),next()]),enabled:[0,1,128,255][case/3%4],flags_b2:u16::from_le_bytes([next(),next()]),multiplier:next(),b1:next(),command:next()%32,blocked_elements:next(),gfx_kind:next(),animation:if case%3==0 {139+(case%24) as u8}else{next()},weapon_length:next(),insertion:next()};
        let mut c=Context {damage,effects:crate::battle_target_effects::State {message:next(),item:next(),obtained_mask:next(),gil:u32::from(next())|(u32::from(next())<<8)|(u32::from(next())<<16),stolen_gil:u32::from(next())|(u32::from(next())<<8)|(u32::from(next())<<16),deliveries:Vec::new()},secondary_effect:if case%3==0 {164}else{crate::battle_target_effects::NO_OPS[case/3%crate::battle_target_effects::NO_OPS.len()]},multiplier_high:next(),params:std::array::from_fn(|_|u16::from_le_bytes([next(),next()])),facing:u16::from_le_bytes([next(),next()]),zinger_attacker:if case%5==0 {(case%10*2) as u8}else{255},failed:u16::from_le_bytes([next(),next()]),original_command:next(),last_attack:if case%3==0 {255}else{next()},last_item:if case%5==0 {255}else{next()},lore:Lore {strago_offset:if case%7==0 {255}else{(case/7%4*2) as u8},attack_flags:magic[3],attack:damage.animation,known:[next(),next(),next()],learned:[next(),next(),next()]},parameter_tail:(case%16*16) as u8,parameters:std::array::from_fn(|_|next())};
        if case%11==0 {c.params[2]=0;}else if case%7==0 {c.params[2]=actor_mask((case+4)%10);}
        let mut a=std::array::from_fn(|slot|Actor {hp:u16::from_le_bytes([next(),next()]),mp:u16::from_le_bytes([next(),next()]),max_hp:u16::from_le_bytes([next(),next()]),max_mp:u16::from_le_bytes([next(),next()]),level:next(),status12:if case%5==0 {0}else{u16::from_le_bytes([next(),next()])},status34:if case%5==0 {0}else{u16::from_le_bytes([next(),next()])},monster_flags:u16::from_le_bytes([next(),next()]),defense:next(),magic_defense:next(),flags1:next(),absorb:next(),null:next(),half:next(),weak:next(),pending_set:u16::from_le_bytes([next(),next()]),pending_clear:u16::from_le_bytes([next(),next()]),update:u16::from_le_bytes([next(),next()]),control:255,previous:if case%3==0 {128|slot as u8}else{127},retaliation_target:next(),taken:65535,healed:65535});
        let mut extra=std::array::from_fn(|_|Extra {effect:crate::battle_target_effects::Actor {items:[if case%5==0 {255}else{next()},if case%7==0 {255}else{next()}],obtained:next(),relic3:next(),monster_gil:u16::from_le_bytes([next(),next()])},history:History {command_source:next(),attack_source:next(),item_source:next(),element_source:next(),command:next(),attack:next(),item:next(),element:next()},allowed12:u16::from_le_bytes([next(),next()]),allowed34:u16::from_le_bytes([next(),next()]),pending_set34:u16::from_le_bytes([next(),next()]),pending_clear34:u16::from_le_bytes([next(),next()])});let mut g=Graphics::default();g.tail=next();g.command=u16::from_le_bytes([next(),next()]);g.parameter=u16::from_le_bytes([next(),next()]);g.frame_count=(case%7) as u8;g.script=std::array::from_fn(|_|next());let mut rng=BattleRng {index:next()};
        let before=snapshot(&a,&extra,&c,&g,&rng);run(&mut a,&mut extra,&mut c,&mut g,&mut rng,&table)?;writeln!(rows,"{case},{attacker},{before},{},{}",snapshot(&a,&extra,&c,&g,&rng),c.effects.deliveries.iter().map(|(slot,item)|format!("{slot}:{item}")).collect::<Vec<_>>().join("|")).unwrap();
    }
    fs::write(dir.join("resolution.csv"),rows).map_err(|e|e.to_string())
}
fn battle_damage_exec_context(attacker:usize,magic:[u8;14],next:&mut impl FnMut()->u8)->crate::battle_damage_exec::Context {crate::battle_damage_exec::Context {attacker,attack_flags:u16::from(magic[2])|(u16::from(magic[3])<<8),invincible:u16::from_le_bytes([next(),next()]),golem:255,dog:255,golem_hp:0,weapon_spell:next(),blackbelt_blocked:next(),misses:u16::from_le_bytes([next(),next()])}}
#[cfg(test)]mod tests {
    use super::*;
    fn actor()->Actor {Actor {hp:100,mp:30,max_hp:100,max_mp:30,level:10,status12:0,status34:0,monster_flags:0,defense:0,magic_defense:0,flags1:0,absorb:0,null:0,half:0,weak:0,pending_set:0,pending_clear:0,update:0,control:255,previous:127,retaliation_target:255,taken:65535,healed:65535}}
    fn extra()->Extra {Extra {effect:crate::battle_target_effects::Actor::default(),history:History::default(),allowed12:65535,allowed34:65535,pending_set34:0,pending_clear34:0}}
    fn context()->Context {
        let mut magic=[0;14];magic[6]=16;magic[7]=2;let mut next=||0;
        Context {effects:crate::battle_target_effects::State::default(),damage:DamageContext {execution:battle_damage_exec_context(0,magic,&mut next),magic,base:150,enabled:0,flags_b2:0,multiplier:0,b1:0,command:2,blocked_elements:0,gfx_kind:6,animation:139,weapon_length:0,insertion:0},secondary_effect:0,multiplier_high:0,params:[0,0,2,0,0,0,0,0],facing:0,zinger_attacker:255,failed:123,original_command:29,last_attack:131,last_item:255,lore:Lore {strago_offset:2,attack_flags:0,attack:139,known:[0;3],learned:[0;3]},parameter_tail:0,parameters:[0;256]}
    }
    #[test]fn lethal_hit_records_history_applies_damage_and_copies_graphics_masks() {
        let mut a=[actor();10];let mut e=[extra();10];let mut c=context();let mut g=Graphics::default();run(&mut a,&mut e,&mut c,&mut g,&mut BattleRng::default(),&RngTable::identity()).unwrap();assert_eq!((a[1].hp,a[1].pending_set,a[1].previous),(0,128,128));assert_eq!((e[1].history.command,e[1].history.attack,e[1].history.command_source),(29,131,0));assert_eq!((c.parameter_tail,c.params[2],c.damage.magic[7]),(16,2,2));assert_eq!(&c.parameters[4..6],&[2,0]);assert_eq!(&g.script[..4],&[11,1,150,0]);
    }
    #[test]fn zinger_skip_still_records_target_history_but_does_not_apply_status_or_damage() {
        let mut a=[actor();10];let mut e=[extra();10];let mut c=context();let mut g=Graphics::default();c.zinger_attacker=2;run(&mut a,&mut e,&mut c,&mut g,&mut BattleRng::default(),&RngTable::identity()).unwrap();assert_eq!((a[1].hp,a[1].previous,e[1].history.attack,c.params[2],c.failed,g.tail,c.damage.magic[7]),(100,128,131,0,123,0,0));
    }
    #[test]fn observer_learning_checks_current_status_even_after_lethal_hp_change() {
        let mut a=[actor();10];let mut e=[extra();10];let mut c=context();let mut g=Graphics::default();c.damage.magic[3]=4;run(&mut a,&mut e,&mut c,&mut g,&mut BattleRng::default(),&RngTable::identity()).unwrap();assert_eq!((a[1].hp,a[1].status12,a[1].pending_set,c.lore.learned[0]),(0,0,128,1));
    }
    #[test]fn facing_increment_is_a_word_and_unsupported_secondary_effect_is_explicit() {
        let mut a=[actor();10];let mut e=[extra();10];let mut c=context();let mut g=Graphics::default();c.damage.multiplier=255;c.multiplier_high=7;c.damage.magic[6]=0;c.facing=2;run(&mut a,&mut e,&mut c,&mut g,&mut BattleRng::default(),&RngTable::identity()).unwrap();assert_eq!((c.damage.multiplier,c.multiplier_high),(0,8));c.secondary_effect=6;assert!(run(&mut a,&mut e,&mut c,&mut g,&mut BattleRng::default(),&RngTable::identity()).unwrap_err().contains("Secondary"));
    }
}
