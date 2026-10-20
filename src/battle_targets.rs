//! ChooseTarget C2/587E..5A82. Command/attack target initialization is separate.
use crate::battle_rng::{BattleRng,RngTable};
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default)]
pub struct Actor {pub present:bool,pub status12:u16,pub status34:u16,pub charm_attacker:u8}
#[derive(Clone,Debug)]
pub struct Context {
    pub actor:usize,pub restrictions:u8,pub cursor:u8,pub actors:[Actor;10],
    pub targettable:u16,pub valid:u16,pub revived:u16,pub jump_seize:u16,
    pub remove_jump:bool,pub enemy_characters:u8,pub battle_type:u8,
    pub pincer_left:u8,pub pincer_right:u8,pub previous_attacker:usize,pub backup:u16,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Selection {pub targets:u16,pub mask_flags:u8}
pub fn actor_mask(slot:usize)->u16 {1u16<<if slot<4 {slot}else{slot+4}}
fn mask(c:&Context,s:&mut Selection) {
    let bypass=s.mask_flags&2!=0;s.mask_flags&=!2;if bypass {return;}
    s.targets&=c.targettable&c.valid&c.revived;
    if c.remove_jump {s.targets&=!c.jump_seize;}
    if c.restrictions&0x80!=0 {s.targets&=0xff00;}
    if c.restrictions&1!=0 {s.targets&=!actor_mask(c.actor);}
}
fn present(c:&Context,s:&mut Selection) {
    for (slot,a) in c.actors.iter().enumerate() {
        if !a.present || a.status12&(if slot<4 {0x80}else{0xc2})!=0 || a.status34&0x2000!=0 {s.targets&=!actor_mask(slot);}
    }
}
fn retarget(c:&Context)->u16 {
    let a=c.actors[c.actor];let mut side=if c.actor>=4 {0x80}else{0};
    if c.restrictions&0x10==0 {
        if a.charm_attacker&0x80==0 {side^=0x80;}
        if actor_mask(c.actor) as u8&c.enemy_characters!=0 {side^=0x80;}
        if c.cursor&0x0c==4 {side|=0x40;}
        side^=(c.cursor&0x40)<<1;
        if a.status12&2!=0 {side|=0x40;}
        if a.status12&0x2000!=0 {side^=0x80;}
    } else {side^=0x80;}
    let mut targets=0;
    if side&0xc0!=0 {targets|=0x3f00|u16::from(c.enemy_characters);}
    if side&0x40!=0 || side&0x80==0 {targets|=u16::from(15^c.enemy_characters);}
    targets
}
fn battle_type(c:&Context,s:&mut Selection,rng:&mut BattleRng,table:&RngTable) {
    if c.restrictions&0x10!=0 {
        if c.restrictions&2!=0 && c.actors[c.actor].status12&0x2000==0 && rng.next(table)&1==0 {
            let previous=actor_mask(c.previous_attacker);if s.targets&previous!=0 {s.targets=previous;}
        }
        return;
    }
    let init=c.cursor&0x0c;
    if init&4==0 {
        if c.battle_type==2 {
            let high=(s.targets>>8) as u8;
            let clear=if c.actor>=4 {Some(if c.pincer_left&(actor_mask(c.actor)>>8) as u8!=0 {c.pincer_right}else{c.pincer_left})}
            else if high&c.pincer_left!=0 && high&c.pincer_right!=0 {Some(if rng.next(table)&1!=0 {c.pincer_left}else{c.pincer_right})}else{None};
            if let Some(clear)=clear {s.targets&=!(u16::from(clear)<<8);}
        }
        if c.battle_type==3 {
            let low=s.targets as u8;
            let clear:Option<u8>=if c.actor<4 {Some(if actor_mask(c.actor)&3!=0 {12}else{3})}
            else if low&3!=0 && low&12!=0 {Some(if rng.next(table)&1!=0 {3}else{12})}else{None};
            if let Some(clear)=clear {s.targets&=!u16::from(clear);}
        }
    }
    if init!=4 && s.targets&255!=0 && s.targets&0xff00!=0 {
        if rng.next(table)&1==0 {s.targets&=0xff00;}else{s.targets&=255;}
    }
}
pub fn choose(c:&Context,s:&mut Selection,rng:&mut BattleRng,table:&RngTable)->Result<(),String> {
    if c.actor>=10 || c.previous_attacker>=10 || c.battle_type>3 {return Err("Target selection context has invalid actor or battle type".into());}
    if c.cursor==2 {s.targets=actor_mask(c.actor);return Ok(());}
    mask(c,s);
    let mut new_target=c.restrictions&0x40!=0;
    if !new_target {
        if c.restrictions&8==0 {present(c,s);}
        if s.targets!=0 {
            if c.cursor&0x2c==0 {s.targets=rng.bit(table,s.targets);}
            return Ok(());
        }
        new_target=c.restrictions&4==0;
    }
    if new_target {s.targets=retarget(c);mask(c,s);if c.restrictions&8==0 {present(c,s);}}
    battle_type(c,s,rng,table);
    if c.restrictions&0x20!=0 && s.targets==0 {s.targets=c.backup;}
    if c.cursor&0x0c!=0 {return Ok(());}
    if c.cursor&0x20!=0 && rng.next(table)&1!=0 {return Ok(());}
    s.targets=rng.bit(table,s.targets);Ok(())
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;let mut seed=0x6ff6u32;
    let mut rows=String::from("case,seed,actor,restrictions,cursor,targets,mask_flags,targettable,valid,revived,jump,remove_jump,enemy,battle_type,left,right,previous,backup,actors,result,after_flags,index\n");
    for case in 0..16384usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let start=next();let restrictions=next();let cursor=if case%31==0 {2}else{next()};
        let actors=std::array::from_fn(|_|Actor {present:next()&3!=0,status12:u16::from_le_bytes([next(),next()]),status34:u16::from_le_bytes([next(),next()]),charm_attacker:if next()&1!=0 {255}else{0}});
        let mut word=||u16::from_le_bytes([next(),next()]);let targets=if case%3==0 {0}else{word()};
        let c=Context {actor:case%10,restrictions,cursor,actors,targettable:word(),valid:word(),revived:word(),jump_seize:word(),remove_jump:case%2==0,enemy_characters:(case%16) as u8,battle_type:(case/3%4) as u8,pincer_left:0x15,pincer_right:0x2a,previous_attacker:case/7%10,backup:word()};
        let flags=if case%7==0 {3}else{1};let mut selection=Selection {targets,mask_flags:flags};let mut rng=BattleRng {index:start};choose(&c,&mut selection,&mut rng,&table)?;
        let actors=c.actors.iter().map(|a|format!("{:02X}{:04X}{:04X}{:02X}",a.present as u8,a.status12,a.status34,a.charm_attacker)).collect::<String>();
        writeln!(rows,"{case},{start},{},{restrictions},{cursor},{targets},{flags},{},{},{},{},{},{},{},{},{},{},{},{actors},{},{},{}",c.actor,c.targettable,c.valid,c.revived,c.jump_seize,c.remove_jump as u8,c.enemy_characters,c.battle_type,c.pincer_left,c.pincer_right,c.previous_attacker,c.backup,selection.targets,selection.mask_flags,rng.index).unwrap();
    }
    fs::write(directory.join("selection.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn context()->Context {Context {actor:6,restrictions:0,cursor:0x40,actors:[Actor {present:true,status12:0,status34:0,charm_attacker:255};10],targettable:0x3f0f,valid:65535,revived:65535,jump_seize:0,remove_jump:false,enemy_characters:0,battle_type:0,pincer_left:0x15,pincer_right:0x2a,previous_attacker:0,backup:0}}
    #[test]fn guard_chooses_a_living_character_and_consumes_one_rng_byte() {
        let mut c=context();c.actors[1].present=false;c.actors[2].status12=128;
        let mut s=Selection {targets:0,mask_flags:0};let mut rng=BattleRng::default();choose(&c,&mut s,&mut rng,&RngTable::identity()).unwrap();
        assert_eq!(s.targets,1);assert_eq!(rng.index,1);
    }
    #[test]fn self_target_bypasses_masks_and_rng() {
        let mut c=context();c.cursor=2;c.targettable=0;
        let mut s=Selection {targets:0,mask_flags:2};let mut rng=BattleRng::default();choose(&c,&mut s,&mut rng,&RngTable::identity()).unwrap();assert_eq!((s.targets,s.mask_flags,rng.index),(0x400,2,0));
    }
    #[test]fn side_and_pincer_coin_branches_use_source_side_order() {
        let table=RngTable::identity();let mut c=context();c.battle_type=3;
        let mut s=Selection {targets:0,mask_flags:0};let mut rng=BattleRng::default();choose(&c,&mut s,&mut rng,&table).unwrap();assert_eq!((s.targets,rng.index),(4,2));
        c.actor=0;c.battle_type=2;s.targets=0;rng.index=0;choose(&c,&mut s,&mut rng,&table).unwrap();assert_eq!((s.targets,rng.index),(0x200,2));
    }
    #[test]fn guard_ai_chooses_party_target_before_native_command_storage() {
        use crate::{battle_ai::{AiTable,AiLoop},battle_commands::{CommandLists,MpCosts,Payload},battle_scheduler::CommandDelays};
        let root=Path::new("extracted/twue-3.07");let ai=AiTable::load(root).unwrap();let table=RngTable::load(root).unwrap();
        let costs=MpCosts::from_attacks(&crate::damage::AttackTable::load(root).unwrap());let delays=CommandDelays::load(root).unwrap();
        let initializer=crate::battle_target_init::Initializer::load(root).unwrap();
        let mut target_state=crate::battle_target_init::State {restrictions:0,cursor:0,flags_b1:0,last_spell:0,last_item:0,magic:[0;14]};
        let mut c=context();
        for slot in 0..10 {c.actors[slot].present=[0,1,2,6,7].contains(&slot);}
        let mut state=AiLoop {resume:0,block:255};let mut rng=BattleRng::default();let mut pool=CommandLists::default();let mut duration=0;let mut flags=0;
        ai.run_normal(0,&mut state,0,false,&mut rng,&table,|a,rng| {
            initializer.initialize(a.command,a.attack,&mut target_state);c.restrictions=target_state.restrictions&!4;c.cursor=target_state.cursor;
            let mut selection=Selection {targets:a.targets,mask_flags:0};choose(&c,&mut selection,rng,&table)?;
            duration=delays.add(duration,a.command);let mp_cost=costs.get(a.command,a.attack,12,&mut flags)?;
            pool.normal(6,Payload {command:a.command,attack:a.attack,targets:selection.targets,mp_cost})?;Ok(())
        }).unwrap();
        let p=pool.payloads[usize::from(pool.normal[6])];assert_eq!((p.command,p.mp_cost,duration),(0,0,16));assert!([1,2,4].contains(&p.targets));assert_eq!(rng.index,2);
    }
}
