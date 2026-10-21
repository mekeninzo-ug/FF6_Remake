//! Target attack history C2/35E3..362E and Lore learning C2/36D6..370A.
use crate::battle_rng::{BattleRng,RngTable};
use std::{fs,path::Path,collections::BTreeMap};
#[derive(Clone,Copy,Debug)]pub struct History {pub command_source:u8,pub attack_source:u8,pub item_source:u8,pub element_source:u8,pub command:u8,pub attack:u8,pub item:u8,pub element:u8}
impl Default for History {fn default()->Self {Self {command_source:255,attack_source:255,item_source:255,element_source:255,command:255,attack:255,item:255,element:0}}}
#[derive(Clone,Copy,Debug)]pub struct Action {pub command:u8,pub attack:u8,pub item:u8,pub element:u8}
/// `previous` is the same byte used by target damage and retaliation; the
/// caller owns it once and passes a mutable reference into this stage.
pub fn record(attacker:usize,previous:&mut u8,h:&mut History,action:Action,rng:&mut BattleRng,table:&RngTable)->Result<(),String> {
    if attacker>=10 {return Err("Attack history attacker outside ten slots".into());}
    if *previous&128==0 || rng.next(table)&1!=0 {*previous=128|attacker as u8;}
    let offset=(attacker*2) as u8;h.command_source=offset;h.command=action.command;
    if action.attack!=255 {h.attack=action.attack;h.attack_source=offset;}
    if action.item!=255 {h.item=action.item;h.item_source=offset;}
    h.element=action.element;h.element_source=offset;Ok(())
}
#[derive(Clone,Copy,Debug)]pub struct Lore {pub strago_offset:u8,pub attack_flags:u8,pub attack:u8,pub known:[u8;3],pub learned:[u8;3]}
pub fn learn(l:&mut Lore,status12:&[u16;10],status34:&[u16;10])->Result<(),String> {
    if l.attack_flags&4==0 || l.strago_offset&128!=0 {return Ok(());}
    if l.strago_offset>=20 || l.strago_offset&1!=0 {return Err("Lore observer is outside ten aligned actor offsets".into());}
    let slot=usize::from(l.strago_offset/2);
    if status12[slot]&0xb0c3!=0 || status34[slot]&0x2310!=0 {return Ok(());}
    let bit=l.attack.wrapping_sub(139);let byte=usize::from(bit>>3);if byte>=3 {return Ok(());}
    let mask=1<<(bit&7);if l.known[byte]&mask==0 {l.learned[byte]|=mask;}Ok(())
}
fn history_snapshot(previous:&[u8;10],h:&[History;10],action:Action,rng:&BattleRng)->String {
    let mut m=BTreeMap::<u32,u8>::new();m.insert(0xbe,rng.index);
    for (slot,h) in h.iter().enumerate() {let x=slot as u32*2;for (addr,v) in [(0x32e0,previous[slot]),(0x3290,h.command_source),(0x3291,h.attack_source),(0x32a4,h.item_source),(0x32a5,h.element_source),(0x3d48,h.command),(0x3d49,h.attack),(0x3d5c,h.item),(0x3d5d,h.element)] {m.insert(0x7e0000+addr+x,v);}}
    for (addr,v) in [(0x3a7c,action.command),(0x3410,action.attack),(0x3411,action.item),(0x11a1,action.element)] {m.insert(0x7e0000+addr,v);}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
fn lore_snapshot(l:&Lore,s12:&[u16;10],s34:&[u16;10])->String {
    let mut m=BTreeMap::<u32,u8>::new();m.insert(0x7e3007,l.strago_offset);m.insert(0x7e11a3,l.attack_flags);m.insert(0xb6,l.attack);
    for slot in 0..10 {for (addr,v) in [(0x3ee4,s12[slot]),(0x3ef8,s34[slot])] {for i in 0..2 {m.insert(0x7e0000+addr+slot as u32*2+i,(v>>(i*8)) as u8);}}}
    for i in 0..3 {m.insert(0x7e1d29+i as u32,l.known[i]);m.insert(0x7e3a84+i as u32,l.learned[i]);}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,attacker,target,mode,targets,before,after\n");
    for case in 0..8192usize {
        let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let attacker=case%10;let target=case/10%10;let mode=case%2;
        let mut previous=std::array::from_fn(|_|next());let mut histories=std::array::from_fn(|_|History {command_source:next(),attack_source:next(),item_source:next(),element_source:next(),command:next(),attack:next(),item:next(),element:next()});
        let action=Action {command:next(),attack:if case%3==0 {255}else{next()},item:if case%5==0 {255}else{next()},element:next()};let mut rng=BattleRng {index:next()};let targets=u16::from_le_bytes([next(),next()]);let before=history_snapshot(&previous,&histories,action,&rng);
        if mode==0 {record(attacker,&mut previous[target],&mut histories[target],action,&mut rng,&table)?;}else{for slot in (0..10).rev() {if crate::battle_targets::actor_mask(slot)&targets!=0 {record(attacker,&mut previous[slot],&mut histories[slot],action,&mut rng,&table)?;}}}
        writeln!(rows,"{case},{attacker},{target},{mode},{targets},{before},{}",history_snapshot(&previous,&histories,action,&rng)).unwrap();
    }
    fs::write(dir.join("history.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,before,after\n");
    for case in 0..4096usize {
        let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let mut l=Lore {strago_offset:if case%7==0 {255}else{(case/7%10*2) as u8},attack_flags:if case%3==0 {0}else{4},attack:case as u8,known:[next(),next(),next()],learned:[next(),next(),next()]};
        let mut s12=std::array::from_fn(|_|if case%5==0 {0}else{u16::from_le_bytes([next(),next()])});let mut s34=std::array::from_fn(|_|if case%5==0 {0}else{u16::from_le_bytes([next(),next()])});
        if case<768 {l.strago_offset=(case/24%4*2) as u8;l.attack_flags=4;l.attack=139+(case%24) as u8;l.known=[0;3];l.learned=[0;3];s12=[0;10];s34=[0;10];}
        let before=lore_snapshot(&l,&s12,&s34);learn(&mut l,&s12,&s34)?;writeln!(rows,"{case},{before},{}",lore_snapshot(&l,&s12,&s34)).unwrap();
    }
    fs::write(dir.join("lore.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn first_attacker_is_always_recorded_and_marked_attackers_draw_once() {
        let mut previous=127;let mut h=History::default();let action=Action {command:2,attack:131,item:255,element:1};let table=RngTable::identity();let mut rng=BattleRng {index:255};
        record(1,&mut previous,&mut h,action,&mut rng,&table).unwrap();assert_eq!((previous,rng.index),(129,255));record(2,&mut previous,&mut h,action,&mut rng,&table).unwrap();assert_eq!((previous,rng.index),(129,0));record(2,&mut previous,&mut h,action,&mut rng,&table).unwrap();assert_eq!((previous,rng.index),(130,1));
    }
    #[test]fn absent_attack_and_item_preserve_their_old_history_and_sources() {
        let mut previous=127;let mut h=History {attack:9,attack_source:4,item:13,item_source:6,..History::default()};let mut rng=BattleRng::default();record(1,&mut previous,&mut h,Action {command:29,attack:255,item:255,element:0},&mut rng,&RngTable::identity()).unwrap();assert_eq!((h.command,h.command_source,h.attack,h.attack_source,h.item,h.item_source,h.element,h.element_source),(29,2,9,4,13,6,0,2));
    }
    #[test]fn learning_only_sets_battle_bits_and_observer_need_not_be_targeted() {
        let mut l=Lore {strago_offset:2,attack_flags:4,attack:162,known:[0;3],learned:[0;3]};learn(&mut l,&[0;10],&[0;10]).unwrap();assert_eq!((l.learned,l.known),([0,0,128],[0;3]));l.attack=163;learn(&mut l,&[0;10],&[0;10]).unwrap();assert_eq!(l.learned,[0,0,128]);
    }
    #[test]fn lore_status_known_and_absent_gates_preserve_pending_bits() {
        let mut l=Lore {strago_offset:0,attack_flags:4,attack:139,known:[0;3],learned:[0;3]};let mut s=[0;10];s[0]=1;learn(&mut l,&s,&[0;10]).unwrap();assert_eq!(l.learned,[0;3]);l.known[0]=1;learn(&mut l,&[0;10],&[0;10]).unwrap();assert_eq!(l.learned,[0;3]);l.strago_offset=255;l.known=[0;3];learn(&mut l,&[0;10],&[0;10]).unwrap();assert_eq!(l.learned,[0;3]);
    }
}
