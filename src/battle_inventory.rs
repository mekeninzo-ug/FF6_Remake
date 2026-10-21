//! C1/01CC item receiver, C1/4445 inventory merge and C2/54DC item profile.
use crate::equipment::ItemTable;
use std::{fs,path::Path,collections::BTreeMap};
pub type Record=[u8;5];
#[derive(Clone,Debug)]pub struct Inventory {pub slots:[Record;264],pub pending:[Record;16],pub producer:u8,pub consumer:u8,pub profile:Record}
impl Default for Inventory {fn default()->Self {Self {slots:[[255,128,0,0,255];264],pending:[[255,128,0,0,255];16],producer:0,consumer:0,profile:[255,128,0,0,255]}}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct Update {pub consumed:usize,pub blocked:bool}
pub fn load_profile(item:u8,table:&ItemTable,masks:&[u16;4],buffer:&mut Record) {
    buffer[0]=item;buffer[1]=128;buffer[4]=255;if item==255 {return;}
    let r=table.record(item);buffer[2]=r[14];if r[0]&32!=0 {buffer[1]&=127;}if r[0]&16!=0 {buffer[1]|=32;}
    let flags=[160u8,8,128,4,128,128,0,0][usize::from(r[0]&7)];buffer[1]|=flags.wrapping_shl(1);
    if flags&128==0 {let allowed=u16::from_le_bytes([r[1],r[2]]);buffer[4]=0;for slot in 0..4 {if allowed&masks[slot]==0 {buffer[4]|=1<<slot;}}}
}
impl Inventory {
    /// C2/546E: reverse-copy equipped hands, then all 256 field slots.
    /// The shared profile keeps stale targeting for empty items, exactly as LoadItemProp.
    pub fn import_field(&mut self,ids:&[u8;256],quantities:&[u8;256],hands:&[[u8;2];4],owned_tools:&mut u8,table:&ItemTable,masks:&[u16;4]) {
        self.profile[3]=1;
        for hand in (0..2).rev() {for actor in (0..4).rev() {
            load_profile(hands[actor][hand],table,masks,&mut self.profile);
            self.slots[256+hand*4+actor]=self.profile;
        }}
        for slot in (0..256).rev() {self.profile[3]=quantities[slot];load_profile(ids[slot],table,masks,&mut self.profile);self.slots[slot]=self.profile;}
        // Source ORs tools without checking quantities and without clearing old bits.
        for &id in ids {if (163..171).contains(&id) {*owned_tools|=1<<(id-163);}}
    }
    /// C2/4987..499D only: ordinary inventory writeback. Character persistence,
    /// Coliseum wager subtraction and other UpdateSRAM duties are separate.
    pub fn export_field(&self,ids:&mut[u8;256],quantities:&mut[u8;256]) {
        for slot in (0..256).rev() {ids[slot]=self.slots[slot][0];quantities[slot]=if ids[slot]==255 {0}else{self.slots[slot][3]};}
    }
    /// The target retains the source's 64-byte clear, not all 80 queue bytes.
    pub fn initialize_pending_source_span(&mut self) {for byte in 0..64 {self.pending[byte/5][byte%5]=255;}}
    pub fn receive(&mut self) {self.pending[usize::from(self.producer&15)]=self.profile;self.producer=self.producer.wrapping_add(1);}
    pub fn update(&mut self)->Update {
        let mut consumed=0;
        loop {let index=usize::from(self.consumer&15);let record=self.pending[index];let id=record[0];if id==255 {return Update {consumed,blocked:false};}
            if let Some(slot)=self.slots[..256].iter().position(|r|r[0]==id) {self.slots[slot][3]=self.slots[slot][3].wrapping_add(record[3]).min(99);}
            else if let Some(slot)=self.slots[..256].iter().position(|r|r[0]==255) {self.slots[slot]=record;}
            else {return Update {consumed,blocked:true};}
            self.pending[index][0]=255;self.consumer=self.consumer.wrapping_add(1);consumed+=1;
        }
    }
    /// GIVE_ITEM receives only; the later UPDATE_INVENTORY/menu stage merges.
    pub fn receive_deliveries(&mut self,deliveries:&mut Vec<(u8,u8)>,table:&ItemTable,masks:&[u16;4])->usize {
        let count=deliveries.len();for (_,item) in deliveries.drain(..) {load_profile(item,table,masks,&mut self.profile);self.profile[3]=1;self.receive();}count
    }
    pub fn flush_pending(&mut self,actors:&mut[crate::battle_target_effects::Actor;10],effects:&mut crate::battle_target_effects::State,table:&ItemTable,masks:&[u16;4]) {
        self.receive_deliveries(&mut effects.deliveries,table,masks);
        for slot in (0..4).rev() {let mask=1<<slot;let ready=effects.obtained_mask&mask!=0;effects.obtained_mask&=!mask;if ready && actors[slot].obtained!=255 {load_profile(actors[slot].obtained,table,masks,&mut self.profile);self.profile[3]=1;self.receive();actors[slot].obtained=255;}}
    }
}
pub(crate) fn snapshot(s:&Inventory,masks:&[u16;4])->String {
    let mut m=BTreeMap::<u32,u8>::new();for (base,records) in [(0x7e2686,s.slots.as_slice()),(0x7e602d,s.pending.as_slice())] {for (index,record) in records.iter().enumerate() {for (field,value) in record.iter().enumerate() {m.insert(base+index as u32*5+field as u32,*value);}}}
    for (i,v) in s.profile.iter().enumerate() {m.insert(0x7e2e72+i as u32,*v);}m.insert(0x7e64da,s.producer);m.insert(0x7e64db,s.consumer);
    for (slot,v) in masks.iter().enumerate() {m.insert(0x7e3a20+slot as u32*2,*v as u8);m.insert(0x7e3a21+slot as u32*2,(*v>>8) as u8);}
    m.iter().map(|(addr,value)|format!("{addr:06X}{value:02X}")).collect()
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=ItemTable::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,mode,item,quantity,before,after\n");
    for case in 0..5120usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};let mut s=Inventory::default();s.slots=std::array::from_fn(|i|[if case%3==0 || i>=256 {next()%200}else if i%7==0 {255}else{next()},next(),next(),next(),next()]);s.pending=std::array::from_fn(|i|[if i%5==case%5 {255}else{next()},next(),next(),next(),next()]);s.producer=next();s.consumer=next();s.profile=std::array::from_fn(|_|next());let masks=std::array::from_fn(|_|u16::from_le_bytes([next(),next()]));let item=case as u8;let quantity=next();let mode=case/256%5;let before=snapshot(&s,&masks);
        match mode {0=>load_profile(item,&table,&masks,&mut s.profile),1=>s.receive(),2=>{s.update();},3=>{load_profile(item,&table,&masks,&mut s.profile);s.profile[3]=quantity;s.receive();s.update();},4=>s.initialize_pending_source_span(),_=>unreachable!()}
        writeln!(rows,"{case},{mode},{item},{quantity},{before},{}",snapshot(&s,&masks)).unwrap();
    }
    fs::write(dir.join("inventory.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn existing_stacks_cap_and_new_slots_copy_all_properties() {let mut s=Inventory::default();s.slots[4]=[7,8,9,98,10];s.profile=[7,99,88,3,77];s.receive();s.profile=[9,11,12,1,13];s.receive();assert_eq!(s.update(),Update {consumed:2,blocked:false});assert_eq!(s.slots[4],[7,8,9,99,10]);assert_eq!(s.slots[0],[9,11,12,1,13]);}
    #[test]fn full_inventory_retains_front_and_equipped_slots_are_not_capacity() {let mut s=Inventory::default();for r in &mut s.slots[..256] {r[0]=1;}s.profile=[2,0,0,1,0];s.receive();assert_eq!(s.update(),Update {consumed:0,blocked:true});assert_eq!((s.consumer,s.pending[0][0]),(0,2));s.slots[10][0]=255;assert_eq!(s.update().consumed,1);assert_eq!(s.slots[10][0],2);}
    #[test]fn producer_wrap_is_source_overwrite_and_consumer_uses_sentinel() {let mut s=Inventory::default();s.producer=255;s.profile=[3,0,0,1,0];s.receive();assert_eq!((s.producer,s.pending[15][0]),(0,3));assert_eq!(s.update().consumed,0);s.consumer=255;assert_eq!(s.update().consumed,1);assert_eq!(s.consumer,0);}
    #[test]fn source_initialization_keeps_the_last_sixteen_bytes_untouched() {let mut s=Inventory::default();s.pending=[[1,2,3,4,5];16];s.initialize_pending_source_span();let bytes=s.pending.concat();assert!(bytes[..64].iter().all(|v|*v==255));assert_eq!(&bytes[64..],&[5,1,2,3,4,5,1,2,3,4,5,1,2,3,4,5]);}
    #[test]fn item_profile_preserves_quantity_and_empty_targeting() {let table=ItemTable::load(Path::new("extracted/twue-3.07")).unwrap();let mut b=[0,0,55,7,0];load_profile(255,&table,&[1,2,4,8],&mut b);assert_eq!(b,[255,128,55,7,255]);load_profile(0,&table,&[1,2,4,8],&mut b);assert_eq!(b[3],7);}
    #[test]fn steal_deliveries_and_final_pending_items_reach_inventory_in_order() {use crate::{battle_target_effects as fx,battle_rng::{BattleRng,RngTable}};let table=ItemTable::load(Path::new("extracted/twue-3.07")).unwrap();let mut a=[fx::Actor::default();10];a[0].obtained=4;a[1].obtained=9;a[6].items=[7,8];let mut levels=[1;10];levels[0]=255;let mut effects=fx::State {obtained_mask:2,..fx::State::default()};fx::dispatch(164,0,6,&levels,&mut a,&mut 6,&mut effects,&mut BattleRng {index:255},&RngTable::identity()).unwrap();let mut s=Inventory::default();s.flush_pending(&mut a,&mut effects,&table,&[1,2,4,8]);assert_eq!((s.pending[0][0],s.pending[1][0],s.pending[2][0]),(9,4,7));assert!(effects.deliveries.is_empty());assert_eq!((effects.obtained_mask,a[0].obtained),(0,255));assert_eq!(s.update().consumed,3);assert_eq!((s.slots[0][0],s.slots[1][0],s.slots[2][0]),(9,4,7));}
}

pub fn capture_field(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let table=ItemTable::load(root)?;
    let mut seed=0x6ff6u32;let mut rows=String::from("case,mode,before,after\n");
    for case in 0..512usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let mut s=Inventory::default();s.profile=std::array::from_fn(|_|next());s.slots=std::array::from_fn(|_|std::array::from_fn(|_|next()));
        let mut ids=std::array::from_fn(|i|if case%4==0 {255}else{i as u8});let mut quantities=std::array::from_fn(|_|next());
        let hands=std::array::from_fn(|_|std::array::from_fn(|_|if case%7==0 {255}else{next()}));let masks=std::array::from_fn(|_|u16::from_le_bytes([next(),next()]));let mut tools=next();
        let snap=|s:&Inventory,ids:&[u8;256],qty:&[u8;256],tools:u8| {let mut out=snapshot(s,&masks);for (base,values) in [(0x7e1869,ids),(0x7e1969,qty)] {for (i,v) in values.iter().enumerate() {write!(out,"{:06X}{v:02X}",base+i).unwrap();}}
            write!(out,"7E3A9B{tools:02X}").unwrap();for actor in 0..4 {let offset=actor*37;write!(out,"{:06X}{:02X}{:06X}{:02X}",0x7e3010+actor*2,offset,0x7e3011+actor*2,0).unwrap();for hand in 0..2 {write!(out,"{:06X}{:02X}",0x7e161f+offset+hand,hands[actor][hand]).unwrap();}}out};
        let before=snap(&s,&ids,&quantities,tools);let mode=case%2;
        if mode==0 {s.import_field(&ids,&quantities,&hands,&mut tools,&table,&masks);}else{s.export_field(&mut ids,&mut quantities);}
        writeln!(rows,"{case},{mode},{before},{}",snap(&s,&ids,&quantities,tools)).unwrap();
    }fs::write(dir.join("field-inventory.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod field_tests {
    use super::*;
    #[test]fn hands_stay_separate_and_zero_quantity_tools_are_owned() {let table=ItemTable::load(Path::new("extracted/twue-3.07")).unwrap();let mut s=Inventory::default();let mut ids=[255;256];ids[4]=163;let mut tools=128;let hands=[[1,2],[3,4],[5,6],[7,8]];s.import_field(&ids,&[0;256],&hands,&mut tools,&table,&[1,2,4,8]);assert_eq!(tools,129);assert_eq!(s.slots[256..].iter().map(|r|r[0]).collect::<Vec<_>>(),vec![1,3,5,7,2,4,6,8]);assert!(s.slots[256..].iter().all(|r|r[3]==1));}
    #[test]fn empty_export_clears_quantity_and_equipped_hands_never_export() {let mut s=Inventory::default();s.slots[0]=[255,1,2,99,3];s.slots[1]=[7,1,2,0,3];s.slots[256]=[8,0,0,1,0];let mut ids=[0;256];let mut qty=[99;256];s.export_field(&mut ids,&mut qty);assert_eq!((ids[0],qty[0],ids[1],qty[1]),(255,0,7,0));assert!(!ids.contains(&8));}
}
