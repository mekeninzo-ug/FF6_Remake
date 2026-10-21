//! Complete CalcTargetDmg C2/0B83..0E76 and its source effects.
use crate::{battle_damage_exec::{self,Graphics},damage_application::Target,battle_rng::{BattleRng,RngTable},damage::Modifiers,elemental_damage::{self,Contribution},battle_targets::actor_mask};
use std::{fs,path::Path,collections::BTreeMap};
#[derive(Clone,Copy,Debug)]pub struct Actor {
    pub hp:u16,pub mp:u16,pub max_hp:u16,pub max_mp:u16,pub level:u8,pub status12:u16,pub status34:u16,
    pub monster_flags:u16,pub defense:u8,pub magic_defense:u8,pub flags1:u8,pub absorb:u8,pub null:u8,pub half:u8,pub weak:u8,
    pub pending_set:u16,pub pending_clear:u16,pub update:u16,pub control:u8,pub previous:u8,pub retaliation_target:u8,pub taken:u16,pub healed:u16,
}
#[derive(Clone,Copy,Debug)]pub struct Context {
    pub execution:battle_damage_exec::Context,pub magic:[u8;14],pub base:u16,pub enabled:u8,pub flags_b2:u16,pub multiplier:u8,pub b1:u8,pub command:u8,
    pub blocked_elements:u8,pub gfx_kind:u8,pub animation:u8,pub weapon_length:u8,pub insertion:u8,
}
fn quantity(a:&Actor,mp:bool,maximum:bool)->u16 {match (mp,maximum) {(false,false)=>a.hp,(false,true)=>a.max_hp,(true,false)=>a.mp,(true,true)=>a.max_mp}}
fn store(a:&mut Actor,healing:bool,amount:u16) {if healing {a.healed=elemental_damage::accumulate(a.healed,amount);}else{a.taken=elemental_damage::accumulate(a.taken,amount);}}
pub fn calculate(actors:&mut[Actor;10],target:usize,c:&mut Context,g:&mut Graphics,rng:&mut BattleRng,table:&RngTable)->Result<(),String> {
    let attacker=c.execution.attacker;if attacker>=10 || target>=10 {return Err("Target damage actor outside ten slots".into());}
    let m=c.magic;if m[6]==0 {return Ok(());}
    let mp=m[3]&128!=0;
    let mut amount=if m[4]&128!=0 {
        let used=if actors[target].taken==65535 {0}else{actors[target].taken};
        let value=if c.command==1 || m[2]&4!=0 {quantity(&actors[target],mp,true)}else{quantity(&actors[target],mp,false).saturating_sub(used)};
        (((u32::from(value)*u32::from(m[6]))>>4) as u16).max(1)
    }else {
        let value=Modifiers {rng:if c.enabled!=0 {rng.next(table)}else{0},flags:[m[2],m[3],m[4]],element:m[1],attacker_monster:attacker>=4,target_monster:target>=4,defense:actors[target].defense,magic_defense:actors[target].magic_defense,status3:actors[target].status34 as u8,status4:(actors[target].status34>>8) as u8,target_flags:actors[target].flags1,normal_defense:c.execution.golem&c.execution.dog&128!=0,enabled:c.enabled!=0,attack_flags:c.flags_b2,multiplier:c.multiplier}.calculate(c.base);
        if c.enabled!=0 && ((u32::from(c.flags_b2)<<1)&u32::from(u16::from(m[1])|(u16::from(m[2])<<8))&0x2000)==0 {c.multiplier=0;}
        value
    };
    let Some(Contribution {amount:resolved,flags})=elemental_damage::resolve(amount,elemental_damage::Context {power:m[6],element:m[1],flags0:m[2],flags2:m[4],status1:actors[target].status12 as u8,monster_flags:actors[target].monster_flags as u8,attack_status1:m[10],blocked_elements:c.blocked_elements,absorb:actors[target].absorb,null:actors[target].null,half:actors[target].half,weak:actors[target].weak,special:0})? else{return Ok(());};
    amount=resolved;
    if m[9]==4 {
        let a=actors[attacker];let hp_high=(a.hp>>8) as u8;let max_high=(a.max_hp>>8) as u8;let product=u16::from(hp_high.wrapping_add(1))*u16::from(a.level);let divisor=max_high.wrapping_add(1);
        let factor=if divisor==0 {255}else{(product/u16::from(divisor)) as u8};amount=(((u32::from(amount)*u32::from(factor))>>6) as u16).wrapping_add(1);
        if amount>=501 {c.weapon_length=if amount<1001 {91}else{92};g.command=u16::from(c.gfx_kind)|(u16::from(c.command)<<8);g.parameter=u16::from(c.animation)|(u16::from(c.weapon_length)<<8);let tail=g.tail;g.tail=c.insertion;g.append();g.tail=tail;}
    }
    // Carry entering the drain attribution helper follows the real physical/
    // Zombie branch, rather than being an inferred death or drain predicate.
    let mut carry=m[2]&1!=0;
    if carry && c.execution.golem&c.execution.dog&128!=0 {
        if actors[attacker].status12&2!=0 {let roll=rng.next(table);if roll<16 {actors[target].pending_set|=4;}else if roll<32 {actors[target].pending_set|=1;}carry=roll>=32;}
        actors[target].pending_clear|=u16::from(((m[11]^0xa0)&0xa0)&((actors[target].status12>>8) as u8))<<8;actors[target].control|=128;
    }
    if m[4]&2!=0 {
        if c.enabled&128!=0 {
            let (victim,receiver)=if flags&1!=0 {(attacker,target)}else{(target,attacker)};amount=amount.min(quantity(&actors[victim],mp,false));
            // LDA $B1 is a word here, so its sign is B2 bit 7.
            if c.flags_b2&128!=0 {amount=amount.min(quantity(&actors[receiver],mp,true).wrapping_sub(quantity(&actors[receiver],mp,false)));}
        }
        if actors[attacker].previous&128==0 {actors[attacker].previous=target as u8|if carry {128}else{0};}
        store(&mut actors[attacker],flags&1==0,amount);
    }
    store(&mut actors[target],flags&1!=0,amount);Ok(())
}
pub fn apply(actors:&mut[Actor;10],c:&mut Context,g:&mut Graphics)->Result<(),String> {
    c.execution.attack_flags=u16::from(c.magic[2])|(u16::from(c.magic[3])<<8);
    let mut projected=std::array::from_fn(|slot|{let a=actors[slot];battle_damage_exec::Actor {target:Target {hp:a.hp,mp:a.mp,max_hp:a.max_hp,max_mp:a.max_mp,status1:a.status12,pending_status:a.pending_set,update_flags:a.update,monster_flags:a.monster_flags,weapon_spell:c.execution.weapon_spell},taken:a.taken,healed:a.healed,previous_attacker:a.previous,retaliation_target:a.retaliation_target}});
    battle_damage_exec::run(&mut projected,&mut c.execution,g)?;
    for slot in 0..10 {let p=projected[slot];let a=&mut actors[slot];a.hp=p.target.hp;a.mp=p.target.mp;a.pending_set=p.target.pending_status;a.update=p.target.update_flags;a.taken=p.taken;a.healed=p.healed;a.previous=p.previous_attacker;a.retaliation_target=p.retaliation_target;}
    Ok(())
}
pub(crate) fn snapshot(a:&[Actor;10],c:&Context,g:&Graphics,rng:&BattleRng)->String {
    let mut m=BTreeMap::<u32,u8>::new();let mut put=|addr:u32,val:u16,width:usize|{for i in 0..width {m.insert(addr+i as u32,(val>>(i*8)) as u8);}};
    for (slot,a) in a.iter().enumerate() {let x=slot as u32*2;
        for (addr,val,width) in [(0x3bf4,a.hp,2),(0x3c08,a.mp,2),(0x3c1c,a.max_hp,2),(0x3c30,a.max_mp,2),(0x3b18,u16::from(a.level),1),(0x3ee4,a.status12,2),(0x3ef8,a.status34,2),(0x3c95,a.monster_flags,2),(0x3bb8,u16::from(a.defense),1),(0x3bb9,u16::from(a.magic_defense),1),(0x3aa1,u16::from(a.flags1),1),(0x3bcc,u16::from(a.absorb),1),(0x3bcd,u16::from(a.null),1),(0x3be1,u16::from(a.half),1),(0x3be0,u16::from(a.weak),1),(0x3dd4,a.pending_set,2),(0x3dfc,a.pending_clear,2),(0x3204,a.update,2),(0x32b9,u16::from(a.control),1),(0x32e0,u16::from(a.previous),1),(0x327c,u16::from(a.retaliation_target),1),(0x33d0,a.taken,2),(0x33e4,a.healed,2),(0x3018,actor_mask(slot),2)] {put(0x7e0000+addr+x,val,width);}}
    for (i,v) in c.magic.iter().enumerate() {put(0x7e11a0+i as u32,u16::from(*v),1);}
    let e=c.execution;for (addr,val,width) in [(0x7e11b0,c.base,2),(0x7e3414,u16::from(c.enabled),1),(0xb2,c.flags_b2,2),(0xbc,u16::from(c.multiplier),1),(0xb1,u16::from(c.b1),1),(0xb5,u16::from(c.command),1),(0xb4,u16::from(c.gfx_kind),1),(0xb6,u16::from(c.animation),1),(0xb7,u16::from(c.weapon_length),1),(0x7e3ec8,u16::from(c.blocked_elements),1),(0x7e3a71,u16::from(c.insertion),1),(0x7e3a3c,e.invincible,2),(0x7e3a82,u16::from(e.golem),1),(0x7e3a83,u16::from(e.dog),1),(0x7e3a36,e.golem_hp,2),(0x7e3a89,u16::from(e.weapon_spell),1),(0x7e3419,u16::from(e.blackbelt_blocked),1),(0x7e3a5a,e.misses,2),(0xbe,u16::from(rng.index),1),(0x7e3a28,g.command,2),(0x7e3a2a,g.parameter,2),(0x7e3a72,u16::from(g.tail),1),(0x7e3a34,u16::from(g.frame_count),1)] {put(addr,val,width);}
    for (i,v) in g.script.iter().enumerate() {put(0x7e2d6e+i as u32,u16::from(*v),1);}for (frame,values) in g.frames.iter().take(8).enumerate() {for (slot,v) in values.iter().enumerate() {put(0x7e2bce+frame as u32*20+slot as u32*2,*v,2);}}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,attacker,target,combined,before,after\n");
    for case in 0..8192usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let attacker=case%10;let target=case/10%10;
        let mut magic=std::array::from_fn(|_|next());magic[6]=if case%13==0 {0}else{next()};magic[9]=if case%5==0 {4}else{0};
        let mut c=Context {execution:battle_damage_exec::Context {attacker,attack_flags:u16::from(magic[2])|(u16::from(magic[3])<<8),invincible:u16::from_le_bytes([next(),next()]),golem:if case%17==0 {(target*2) as u8}else{255},dog:255,golem_hp:u16::from_le_bytes([next(),next()]),weapon_spell:next(),blackbelt_blocked:next(),misses:u16::from_le_bytes([next(),next()])},magic,base:u16::from_le_bytes([next(),next()]),enabled:[0,1,128,255][case/3%4],flags_b2:u16::from_le_bytes([next(),next()]),multiplier:next()%8,b1:next(),command:if case%7==0 {1}else{next()%32},blocked_elements:next(),gfx_kind:next(),animation:next(),weapon_length:next(),insertion:next()};
        let mut a=std::array::from_fn(|slot|Actor {hp:u16::from_le_bytes([next(),next()]),mp:u16::from_le_bytes([next(),next()]),max_hp:u16::from_le_bytes([next(),next()]),max_mp:u16::from_le_bytes([next(),next()]),level:next(),status12:u16::from_le_bytes([next(),next()]),status34:u16::from_le_bytes([next(),next()]),monster_flags:u16::from_le_bytes([next(),next()]),defense:next(),magic_defense:next(),flags1:next(),absorb:next(),null:next(),half:next(),weak:next(),pending_set:u16::from_le_bytes([next(),next()]),pending_clear:u16::from_le_bytes([next(),next()]),update:u16::from_le_bytes([next(),next()]),control:255,previous:if case%3==0 {128|slot as u8}else{127},retaliation_target:next(),taken:if case%3==0 {65535}else{u16::from(next())*37},healed:if case%5==0 {65535}else{u16::from(next())*29}});
        let mut g=Graphics::default();g.tail=next();g.command=u16::from_le_bytes([next(),next()]);g.parameter=u16::from_le_bytes([next(),next()]);g.frame_count=(case%7) as u8;g.script=std::array::from_fn(|_|next());let mut rng=BattleRng {index:next()};let combined=case%2==1;
        let before=snapshot(&a,&c,&g,&rng);calculate(&mut a,target,&mut c,&mut g,&mut rng,&table)?;if combined {apply(&mut a,&mut c,&mut g)?;}
        writeln!(rows,"{case},{attacker},{target},{},{before},{}",u8::from(combined),snapshot(&a,&c,&g,&rng)).unwrap();
    }
    fs::write(dir.join("target-damage.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn actor()->Actor {Actor {hp:100,mp:50,max_hp:200,max_mp:100,level:32,status12:0,status34:0,monster_flags:0,defense:0,magic_defense:0,flags1:0,absorb:0,null:0,half:0,weak:0,pending_set:0,pending_clear:0,update:0,control:255,previous:127,retaliation_target:255,taken:65535,healed:65535}}
    fn context()->Context {let mut magic=[0;14];magic[6]=16;Context {execution:battle_damage_exec::Context {attacker:0,attack_flags:0,invincible:0,golem:255,dog:255,golem_hp:0,weapon_spell:9,blackbelt_blocked:255,misses:0},magic,base:40,enabled:0,flags_b2:0,multiplier:0,b1:0,command:2,blocked_elements:0,gfx_kind:6,animation:17,weapon_length:0,insertion:0}}
    #[test]fn fractional_damage_uses_remaining_or_item_maximum_and_accumulates() {
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();let mut rng=BattleRng::default();c.magic[4]=128;c.magic[6]=8;a[1].taken=20;
        calculate(&mut a,1,&mut c,&mut g,&mut rng,&RngTable::identity()).unwrap();assert_eq!((a[1].taken,a[1].hp,rng.index),(60,100,0));a[1].taken=20;c.command=1;
        calculate(&mut a,1,&mut c,&mut g,&mut rng,&RngTable::identity()).unwrap();assert_eq!(a[1].taken,120);
    }
    #[test]fn drain_caps_to_victim_and_recipient_and_uses_b2_sign_in_word_read() {
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();let mut rng=BattleRng::default();c.magic[2]=33;c.magic[4]=2;c.enabled=128;c.flags_b2=128;c.base=200;a[0].hp=80;a[0].max_hp=100;a[1].hp=50;
        calculate(&mut a,1,&mut c,&mut g,&mut rng,&RngTable::identity()).unwrap();assert_eq!((a[0].healed,a[1].taken,a[0].previous),(20,20,129));
        a=[actor();10];a[0].mp=45;a[0].max_mp=50;a[1].mp=10;c.magic[3]=128;
        calculate(&mut a,1,&mut c,&mut g,&mut rng,&RngTable::identity()).unwrap();assert_eq!((a[0].healed,a[1].taken),(5,5));
    }
    #[test]fn zombie_proc_changes_pending_status_and_drain_attribution_carry() {
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();let mut rng=BattleRng {index:255};c.magic[2]=1;c.magic[4]=2;a[0].status12=2;a[1].status12=0xa000;
        calculate(&mut a,1,&mut c,&mut g,&mut rng,&RngTable::identity()).unwrap();assert_eq!((a[1].pending_set,a[1].pending_clear,a[0].previous,rng.index),(4,0xa000,1,0));a[0].previous=127;rng.index=31;
        calculate(&mut a,1,&mut c,&mut g,&mut rng,&RngTable::identity()).unwrap();assert_eq!(a[0].previous,129);
    }
    #[test]fn ultima_uses_hp_high_bytes_and_rewrites_inserted_animation_without_moving_tail() {
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();let mut rng=BattleRng::default();a[0].hp=0x300;a[0].max_hp=0x700;c.magic[9]=4;c.base=4096;c.insertion=255;g.tail=12;
        calculate(&mut a,1,&mut c,&mut g,&mut rng,&RngTable::identity()).unwrap();assert_eq!((a[1].taken,c.weapon_length,g.tail),(1025,92,12));assert_eq!(&g.script[255..259],&[6,2,17,92]);a[1].taken=65535;a[1].status12=64;
        calculate(&mut a,1,&mut c,&mut g,&mut rng,&RngTable::identity()).unwrap();assert_eq!(a[1].taken,1);
    }
    #[test]fn calculated_hit_flows_through_application_into_pending_death_and_numerals() {
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();let mut rng=BattleRng::default();a[1].hp=40;c.base=100;
        calculate(&mut a,1,&mut c,&mut g,&mut rng,&RngTable::identity()).unwrap();assert_eq!(a[1].taken,100);apply(&mut a,&mut c,&mut g).unwrap();assert_eq!((a[1].hp,a[1].pending_set,a[1].previous,c.execution.weapon_spell,a[1].taken),(0,128,128,0,65535));assert_eq!(&g.script[..4],&[11,1,100,0]);
    }
}
