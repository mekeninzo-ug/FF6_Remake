//! Confirmed RTS target entries and Steal C2/399E..3A3B, including TakeGil.
use crate::{battle_rng::{BattleRng,RngTable},battle_targets::actor_mask};
use std::{fs,path::Path,collections::BTreeMap};
pub const NO_OPS:&[u8]=&[0,2,4,10,12,14,18,20,22,24,28,30,34,40,42,44,48,52,54,58,60,72,82,84,88,98,100,108,120,122,130,132,134,138,140,142,144,146,154,156,158,162];
pub fn supported(raw:u8)->bool {NO_OPS.contains(&raw)||raw==164}
#[derive(Clone,Copy,Debug)]pub struct Actor {pub items:[u8;2],pub obtained:u8,pub relic3:u8,pub monster_gil:u16}
impl Default for Actor {fn default()->Self {Self {items:[255;2],obtained:255,relic3:0,monster_gil:0}}}
#[derive(Clone,Debug,Default)]pub struct State {pub message:u8,pub item:u8,pub obtained_mask:u8,pub gil:u32,pub stolen_gil:u32,pub deliveries:Vec<(u8,u8)>}
pub fn dispatch(raw:u8,attacker:usize,target:usize,levels:&[u8;10],actors:&mut[Actor;10],last_command:&mut u8,s:&mut State,rng:&mut BattleRng,table:&RngTable)->Result<(),String> {
    if attacker>=10 || target>=10 {return Err("Target effect actor outside ten slots".into());}
    if NO_OPS.contains(&raw) {return Ok(());}if raw!=164 {return Err(format!("Target effect {raw:02X} is not implemented"));}
    s.message=1;
    if attacker<4 {
        if actors[target].items==[255;2] {*last_command=0;return Ok(());}
        s.message=s.message.wrapping_add(1);
        let sum=u16::from(levels[attacker])+51;
        if sum<256 {
            let chance=i16::from(levels[attacker])+50-i16::from(levels[target]);
            if chance<0 {*last_command=0;return Ok(());}
            if chance<128 {
                let chance=if actors[attacker].relic3&1!=0 {(chance as u8).wrapping_mul(2)}else{chance as u8};
                if rng.below(table,100)>=chance {*last_command=0;return Ok(());}
            }
        }
        let slot=usize::from(rng.next(table)>=32);let item=actors[target].items[slot];
        if item==255 {*last_command=0;return Ok(());}
        s.item=item;
        if actors[attacker].obtained!=255 {s.obtained_mask|=actor_mask(attacker) as u8;for slot in (0..4).rev() {let mask=actor_mask(slot) as u8;let pending=s.obtained_mask&mask!=0;s.obtained_mask&=!mask;if pending && actors[slot].obtained!=255 {s.deliveries.push((slot as u8,actors[slot].obtained));actors[slot].obtained=255;}}}
        actors[attacker].obtained=item;s.obtained_mask|=actor_mask(attacker) as u8;actors[target].items=[255;2];s.message=s.message.wrapping_add(1);
    }else {
        s.stolen_gil&=65535;s.message=s.message.wrapping_add(1);
        if rng.next(table)>=192 {*last_command=0;return Ok(());}
        s.message=s.message.wrapping_sub(1);
        // The source forms the word level<<8 | 0x14; no multiply occurs.
        let requested=(u16::from(levels[attacker])<<8)|20;let taken=s.gil.min(u32::from(requested));s.gil-=taken;
        if taken==0 {*last_command=0;return Ok(());}
        s.stolen_gil=taken;actors[attacker].monster_gil=actors[attacker].monster_gil.saturating_add(taken as u16);s.message=63;
    }
    Ok(())
}
pub(crate) fn snapshot(actors:&[Actor;10],s:&State,levels:&[u8;10],commands:&[u8;10],rng:&BattleRng)->String {
    let mut m=BTreeMap::<u32,u8>::new();let mut put=|addr:u32,val:u32,width:usize|{for i in 0..width {m.insert(addr+i as u32,(val>>(i*8)) as u8);}};
    for slot in 0..10 {let x=slot as u32*2;for (addr,val,width) in [(0x3308,u32::from(actors[slot].items[0])|(u32::from(actors[slot].items[1])<<8),2),(0x32f4,u32::from(actors[slot].obtained),1),(0x3c45,u32::from(actors[slot].relic3),1),(0x3d98,u32::from(actors[slot].monster_gil),2),(0x3b18,u32::from(levels[slot]),1),(0x3d48,u32::from(commands[slot]),1),(0x3018,u32::from(actor_mask(slot)),2)] {put(0x7e0000+addr+x,val,width);}}
    for (addr,val,width) in [(0x7e3401,u32::from(s.message),1),(0x7e2f35,u32::from(s.item),1),(0x7e3a8c,u32::from(s.obtained_mask),1),(0x7e1860,s.gil,3),(0x7e2f38,s.stolen_gil,3),(0xbe,u32::from(rng.index),1)] {put(addr,val,width);}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,attacker,target,before,after,deliveries\n");
    for case in 0..8192usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let attacker=case%10;let target=case/10%10;let mut a=std::array::from_fn(|_|Actor {items:[if case%3==0 {255}else{next()},if case%5==0 {255}else{next()}],obtained:next(),relic3:next(),monster_gil:u16::from_le_bytes([next(),next()])});let levels=std::array::from_fn(|_|next());let mut commands=std::array::from_fn(|_|next());let mut s=State {message:next(),item:next(),obtained_mask:next(),gil:if case%7==0 {0}else{u32::from(next())|(u32::from(next())<<8)|(u32::from(next())<<16)},stolen_gil:u32::from(next())|(u32::from(next())<<8)|(u32::from(next())<<16),deliveries:Vec::new()};let mut rng=BattleRng {index:next()};let before=snapshot(&a,&s,&levels,&commands,&rng);dispatch(164,attacker,target,&levels,&mut a,&mut commands[target],&mut s,&mut rng,&table)?;writeln!(rows,"{case},{attacker},{target},{before},{},{}",snapshot(&a,&s,&levels,&commands,&rng),s.deliveries.iter().map(|(slot,item)|format!("{slot}:{item}")).collect::<Vec<_>>().join("|")).unwrap();}
    fs::write(dir.join("steal.csv"),rows).map_err(|e|e.to_string())?;let supported=NO_OPS.iter().map(|v|format!("{v}\n")).collect::<String>();fs::write(dir.join("no-op-effects.csv"),format!("raw\n{supported}")).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn character_success_selects_one_slot_and_invalidates_both() {let mut a=[Actor::default();10];a[6].items=[7,8];let mut levels=[1;10];levels[0]=255;let mut command=29;let mut s=State::default();let mut rng=BattleRng {index:255};dispatch(164,0,6,&levels,&mut a,&mut command,&mut s,&mut rng,&RngTable::identity()).unwrap();assert_eq!((a[6].items,a[0].obtained,s.item,s.obtained_mask,s.message,rng.index,command),([255;2],7,7,1,3,0,29));}
    #[test]fn empty_slot_does_not_fall_back_and_failure_erases_history() {let mut a=[Actor::default();10];a[6].items=[7,255];let mut levels=[1;10];levels[0]=255;let mut command=29;let mut s=State::default();let mut rng=BattleRng {index:31};dispatch(164,0,6,&levels,&mut a,&mut command,&mut s,&mut rng,&RngTable::identity()).unwrap();assert_eq!((a[6].items,s.message,command,rng.index),([7,255],2,0,32));}
    #[test]fn monster_takes_source_word_amount_and_saturates_held_gil() {let mut a=[Actor::default();10];a[6].monster_gil=65530;let mut levels=[1;10];levels[6]=2;let mut command=1;let mut s=State {gil:1000,stolen_gil:0xffffff,..State::default()};let mut rng=BattleRng {index:255};dispatch(164,6,0,&levels,&mut a,&mut command,&mut s,&mut rng,&RngTable::identity()).unwrap();assert_eq!((s.gil,s.stolen_gil,a[6].monster_gil,s.message),(468,532,65535,63));}
    #[test]fn no_gil_does_not_award_any_and_clears_display_high_byte() {let mut a=[Actor::default();10];let levels=[1;10];let mut command=1;let mut s=State {stolen_gil:0xabcdef,..State::default()};dispatch(164,6,0,&levels,&mut a,&mut command,&mut s,&mut BattleRng {index:255},&RngTable::identity()).unwrap();assert_eq!((s.stolen_gil,s.message,command),(0xcdef,1,0));}
    #[test]fn patched_success_delivers_old_pending_items_before_replacing_the_slot() {let mut a=[Actor::default();10];a[0].obtained=4;a[1].obtained=9;a[6].items=[7,8];let mut levels=[1;10];levels[0]=255;let mut command=6;let mut s=State {obtained_mask:2,..State::default()};dispatch(164,0,6,&levels,&mut a,&mut command,&mut s,&mut BattleRng {index:255},&RngTable::identity()).unwrap();assert_eq!(s.deliveries,vec![(1,9),(0,4)]);assert_eq!((a[0].obtained,a[1].obtained,s.obtained_mask),(7,255,1));}
}
