//! InitParty C2/2F2F..30E7, with actual scripted-character table and AI pointers.
use crate::battle_ai::AiTable;
use std::{fs,path::Path,collections::BTreeMap};
#[derive(Clone)]pub struct Tables {pub character_ai:[[u8;24];24],ai:AiTable}
impl Tables {pub fn load(root:&Path)->Result<Self,String> {let bytes=fs::read(root.join("build/native_battle/character_ai.bin")).map_err(|e|format!("Character AI: {e}; run tools/extract_native_party.py"))?;if bytes.len()!=576 {return Err("Character AI requires 24 records of 24 bytes".into());}Ok(Self {character_ai:std::array::from_fn(|i|bytes[i*24..i*24+24].try_into().unwrap()),ai:AiTable::load(root)?})}}
#[derive(Clone,Copy,Debug)]pub struct Context {pub fields:[[u8;37];16],pub membership:[u8;16],pub active_party:u8,pub party_mode:u8,pub battle:u16,pub gau_switch:u8,pub monster6:u8}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct Slot {pub actor:u8,pub field_index:u8,pub flags:u8,pub row:u8,pub mask:u16,pub field_pointer:u16,pub graphics:[u8;32],pub ai_normal:u16,pub ai_retaliation:u16}
impl Default for Slot {fn default()->Self {Self {actor:255,field_index:255,flags:0,row:0,mask:0,field_pointer:65535,graphics:[255;32],ai_normal:65535,ai_retaliation:65535}}}
#[derive(Clone,Debug,PartialEq,Eq)]pub struct State {pub slots:[Slot;4],pub candidates:[u8;12],pub battle_flags:u8,pub ai_flags:u8,pub ai_index:u8,pub colosseum:u8,pub valid:u16,pub enemy:u16}
impl Default for State {fn default()->Self {Self {slots:[Slot::default();4],candidates:[255;12],battle_flags:0,ai_flags:0,ai_index:0,colosseum:0,valid:0,enemy:0}}}
pub fn initialize(c:&Context,s:&mut State,tables:&Tables)->Result<(),String> {
    s.valid&=0xff00;let mut count=0u8;
    if c.party_mode==0 {
        for slot in &mut s.slots {slot.field_index=255;if slot.actor==255 {if let Some(index)=s.candidates.iter().position(|v|*v!=255) {let field=usize::from(s.candidates[index]);if field>=16 {return Err("Final-battle candidate outside sixteen field records".into());}s.candidates[index]=255;slot.actor=c.fields[field][0];}}}
    }else if c.battle>=0x23e {s.slots[0].actor=s.candidates[3];s.valid|=1;s.colosseum=s.colosseum.wrapping_sub(1);}
    else {
        for field in (0..16).rev() {let member=c.membership[field];if member&7==c.active_party {count=count.wrapping_add(1);let slot=usize::from((member&24)>>3);s.slots[slot].actor=c.fields[field][0];}}
        if c.gau_switch&8!=0 || c.monster6!=255 || count>=4 {s.battle_flags&=!1;}
        if s.battle_flags&1!=0 {s.ai_index=10;s.ai_flags|=128;}
    }
    if s.ai_flags&128!=0 {
        let data=tables.character_ai.get(usize::from(s.ai_index)).ok_or("Character AI index outside 24 records")?;
        if data[0]&128!=0 {for slot in &mut s.slots {slot.actor=255;}}
        for entry in 0..4 {let at=4+entry*5;let raw=data[at];if raw==255 {continue;}let actor=raw&63;
            let selected=(0..4).rev().find(|i|s.slots[*i].actor==actor).or_else(||(0..4).find(|i|s.slots[*i].actor==255));
            let Some(index)=selected else{continue;};s.valid|=1<<index;let slot=&mut s.slots[index];slot.actor=raw;slot.field_index=data[at+1];let script=data[at+2];
            if script!=255 {let pointers=tables.ai.initialize(0x100|u16::from(script),slot.ai_retaliation)?;slot.ai_normal=pointers.normal;slot.ai_retaliation=pointers.retaliation;}
        }
    }
    for index in (0..4).rev() {let slot=&mut s.slots[index];let raw=slot.actor;if raw==255 {continue;}
        if raw&128==0 {slot.flags=slot.flags.wrapping_add(1);}if raw&64!=0 {s.enemy|=1<<index;}slot.actor=raw&63;
        // Source scans every matching field record; it does not stop at the first.
        for field in (0..16).rev() {let record=c.fields[field];if record[0]!=slot.actor {continue;}slot.row=c.membership[field]&32;let previous=slot.field_index;slot.field_index=field as u8;slot.graphics[0]=if previous==255 {record[1]}else{previous};slot.graphics[24]=slot.actor;slot.graphics[1..7].copy_from_slice(&record[2..8]);
            // The source's actor>=14 branch keeps the last name word in A.
            slot.mask=if slot.actor<14 {1u16<<slot.actor}else{u16::from_le_bytes([record[6],record[7]])};slot.field_pointer=field as u16*37;
        }
    }
    Ok(())
}
fn snapshot(c:&Context,s:&State)->String {
    let mut m=BTreeMap::<u32,u8>::new();let mut put=|addr:u32,value:u16,width:usize|{for i in 0..width {m.insert(addr+i as u32,(value>>(i*8)) as u8);}};
    for (i,v) in c.fields.concat().iter().enumerate() {put(0x7e1600+i as u32,u16::from(*v),1);}for (i,v) in c.membership.iter().enumerate() {put(0x7e1850+i as u32,u16::from(*v),1);}for (i,v) in s.candidates.iter().enumerate() {put(0x7e0205+i as u32,u16::from(*v),1);}
    for (addr,value,width) in [(0x1a6d,u16::from(c.active_party),1),(0x3ee0,u16::from(c.party_mode),1),(0x3ed4,c.battle,2),(0x1edf,u16::from(c.gau_switch),1),(0x3f4b,u16::from(c.monster6),1),(0x11e4,u16::from(s.battle_flags),1),(0x2f49,u16::from(s.ai_flags),1),(0x2f4a,u16::from(s.ai_index),1),(0x3a97,u16::from(s.colosseum),1),(0xb8,s.valid,2),(0x3a40,s.enemy,2)] {put(0x7e0000+addr,value,width);}
    for (i,slot) in s.slots.iter().enumerate() {let x=i as u32*2;for (addr,value,width) in [(0x3ed8,u16::from(slot.actor),1),(0x3ed9,u16::from(slot.field_index),1),(0x3aa0,u16::from(slot.flags),1),(0x3aa1,u16::from(slot.row),1),(0x3a20,slot.mask,2),(0x3010,slot.field_pointer,2),(0x3254,slot.ai_normal,2),(0x3268,slot.ai_retaliation,2),(0x3018,1<<i,2)] {put(0x7e0000+addr+x,value,width);}for (field,v) in slot.graphics.iter().enumerate() {put(0x7e2eae+i as u32*32+field as u32,u16::from(*v),1);}}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let tables=Tables::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,mode,before,after\n");
    for case in 0..1920usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let mode=case%5;let active=(case%3+1) as u8;
        let c=Context {fields:std::array::from_fn(|i|{let mut field=std::array::from_fn(|_|next());field[0]=if case%13==0 {255}else if case%11==0 {(i%4) as u8}else{i as u8};field}),membership:std::array::from_fn(|i|if i<if mode==4 {2}else{4} {active|((i as u8)<<3)|(next()&32)}else{0}),active_party:active,party_mode:if mode==1 {0}else{255},battle:if mode==2 {0x23e+(case%18) as u16}else{2},gau_switch:if mode==4 {0}else{next()},monster6:if mode==4 || case%2==0 {255}else{next()}};
        let mut s=State {slots:std::array::from_fn(|i|Slot {actor:if mode==1 && case%3==0 {i as u8}else{255},field_index:next(),flags:next(),row:next(),mask:u16::from_le_bytes([next(),next()]),field_pointer:u16::from_le_bytes([next(),next()]),graphics:std::array::from_fn(|_|next()),ai_normal:u16::from_le_bytes([next(),next()]),ai_retaliation:u16::from_le_bytes([next(),next()])}),candidates:std::array::from_fn(|_|if case%7==0 {255}else{next()%16}),battle_flags:if mode==4 {1}else{next()&!1},ai_flags:if mode==3 {128}else{0},ai_index:(case/5%24) as u8,colosseum:next(),valid:u16::from_le_bytes([next(),next()]),enemy:u16::from_le_bytes([next(),next()])};let before=snapshot(&c,&s);initialize(&c,&mut s,&tables)?;writeln!(rows,"{case},{mode},{before},{}",snapshot(&c,&s)).unwrap();
    }fs::write(dir.join("party.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn tables()->Tables {Tables::load(Path::new("extracted/twue-3.07")).unwrap()}
    fn context()->Context {Context {fields:std::array::from_fn(|i|{let mut r=[0;37];r[0]=i as u8;r[1]=i as u8;r}),membership:std::array::from_fn(|i|if i<3 {1|((i as u8)<<3)}else{0}),active_party:1,party_mode:255,battle:2,gau_switch:0,monster6:255}}
    #[test]fn field_party_maps_slots_rows_and_record_pointers() {let mut c=context();c.membership[1]|=32;let mut s=State::default();initialize(&c,&mut s,&tables()).unwrap();assert_eq!(s.slots.map(|a|a.actor),[0,1,2,255]);assert_eq!((s.slots[1].row,s.slots[1].field_pointer,s.slots[2].mask),(32,37,4));assert_eq!(s.slots[0].flags,1);}
    #[test]fn guest_equipment_mask_is_last_encoded_name_word() {let mut c=context();c.fields[0][0]=14;c.fields[0][6]=0x12;c.fields[0][7]=0x34;let mut s=State::default();initialize(&c,&mut s,&tables()).unwrap();assert_eq!(s.slots[0].mask,0x3412);}
    #[test]fn final_candidates_are_consumed_and_existing_slot_is_kept() {let mut c=context();c.party_mode=0;let mut s=State::default();s.slots[0].actor=3;s.candidates[..3].copy_from_slice(&[2,1,0]);initialize(&c,&mut s,&tables()).unwrap();assert_eq!(s.slots.map(|a|a.actor),[3,2,1,0]);assert!(s.candidates.iter().all(|v|*v==255));}
    #[test]fn colosseum_uses_candidate_byte_and_decrements_mode_flag() {let mut c=context();c.battle=0x23e;let mut s=State::default();s.candidates[3]=3;initialize(&c,&mut s,&tables()).unwrap();assert_eq!((s.slots[0].actor,s.valid,s.colosseum),(3,1,255));}
    #[test]fn full_party_skips_new_script_actor_instead_of_overwriting_last() {let mut c=context();c.membership[3]=25;let mut t=tables();t.character_ai[0]=[255;24];t.character_ai[0][0]=0;t.character_ai[0][4..7].copy_from_slice(&[41,21,255]);let mut s=State {ai_flags:128,..State::default()};initialize(&c,&mut s,&t).unwrap();assert_eq!(s.slots.map(|a|a.actor),[0,1,2,3]);assert_eq!(s.valid,0);}
    #[test]fn gau_request_uses_real_scripted_actor_and_enemy_presence_flags() {let c=context();let mut s=State {battle_flags:1,..State::default()};initialize(&c,&mut s,&tables()).unwrap();assert_eq!((s.ai_index,s.ai_flags),(10,128));let gau=s.slots.iter().position(|a|a.actor==11).unwrap();assert_eq!(s.slots[gau].flags,0);assert_ne!(s.enemy&(1<<gau),0);assert_ne!(s.valid&(1<<gau),0);}
    #[test]fn duplicate_actor_records_repeat_source_side_effects() {let mut c=context();c.fields[1][0]=0;c.fields[1][1]=9;let mut s=State::default();initialize(&c,&mut s,&tables()).unwrap();assert_eq!((s.slots[0].field_pointer,s.slots[0].graphics[0]),(0,1));}
}
