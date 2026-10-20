use std::collections::HashMap;
use std::path::Path;
use crate::assets::SpriteSheet;
use crate::event_vm::{EventActor,EventState};

#[derive(Clone,Default)]
pub struct FieldEvents { pub triggers: Vec<u8>, pub npc_properties: Vec<u8>,pub initial_switches:Vec<u8> }

#[derive(Clone,Copy,Debug)]
pub struct NpcProperty { pub bytes: [u8;9] }
impl NpcProperty {
    pub fn graphic(self)->u8 {self.bytes[6]}
    pub fn palette(self)->u8 {(self.bytes[2]>>2)&7}
    pub fn switch(self)->usize {0x300+(usize::from(self.bytes[3])<<2)+usize::from(self.bytes[2]>>6)}
    pub fn normal(self)->bool {self.bytes[8]&0xe0==0 && (self.bytes[4]&0x80==0 || self.bytes[7]&0xc0!=0)}
}

impl FieldEvents {
    pub fn load(root:&Path)->Self {
        let directory=root.join("build/native_events");
        Self {triggers:std::fs::read(directory.join("event_triggers.bin")).unwrap_or_default(),
              npc_properties:std::fs::read(directory.join("npc_properties.bin")).unwrap_or_default(),
              initial_switches:std::fs::read(directory.join("initial_npc_switches.bin")).unwrap_or_default()}
    }
    pub fn initialize_switches(&self,state:&mut EventState) {
        if self.initial_switches.len()==128 {state.switches[0x60..].copy_from_slice(&self.initial_switches);}
    }
    fn range<'a>(data:&'a [u8],map:u16,size:usize)->Option<&'a [u8]> {
        if map>=416 || data.len()<834 {return None;}
        let index=usize::from(map)*2;
        let start=usize::from(u16::from_le_bytes([data[index],data[index+1]]));
        let end=usize::from(u16::from_le_bytes([data[index+2],data[index+3]]));
        if start<834 || end<start || end>data.len() || (end-start)%size!=0 {return None;}
        Some(&data[start..end])
    }
    pub fn trigger_at(&self,map:u16,x:i16,y:i16)->Option<usize> {
        Self::range(&self.triggers,map,5)?.chunks_exact(5)
            .find(|record|i16::from(record[0])==x && i16::from(record[1])==y)
            .map(|record|usize::from(record[2])|(usize::from(record[3])<<8)|(usize::from(record[4])<<16))
    }
    pub fn npcs(&self,map:u16)->Vec<NpcProperty> {
        Self::range(&self.npc_properties,map,9).unwrap_or_default().chunks_exact(9)
            .map(|record|NpcProperty {bytes:record.try_into().unwrap()}).collect()
    }
    pub fn init_npcs(&self,map:u16,state:&mut EventState) {
        for actor in &mut state.actors[0x10..0x30] {*actor=EventActor::default();}
        for (index,prop) in self.npcs(map).into_iter().take(32).enumerate() {
            let enabled=prop.normal() && state.switch(prop.switch());
            state.actors[0x10+index]=EventActor {x:i16::from(prop.bytes[4]&0x7f),y:i16::from(prop.bytes[5]&0x3f),
                facing:prop.bytes[8]&3,speed:prop.bytes[5]>>6,graphic:if prop.normal(){prop.graphic()}else{255},palette:prop.palette(),
                enabled,visible:enabled,collision_enabled:true,vehicle:((prop.bytes[7]&0xc0)>>1)|(prop.bytes[4]&0x80),
                ..EventActor::default()};
        }
    }
    pub fn load_sprites(&self,root:&Path,maps:impl Iterator<Item=u16>)->HashMap<(u8,u8),SpriteSheet> {
        let mut gfx=HashMap::new();let mut palettes=HashMap::new();
        if let Ok(source)=std::fs::read_to_string(root.join("build/native_events/sprite_catalog.csv")) {
            for line in source.lines().skip(1) {
                let values=line.split(',').collect::<Vec<_>>();
                if values.len()!=3 {continue;}
                let Ok(id)=values[1].parse::<u8>() else {continue};
                if values[0]=="gfx" {gfx.insert(id,values[2].trim());} else {palettes.insert(id,values[2].trim());}
            }
            let mut keys=vec![(0,2),(14,1)];
            for map in maps {for prop in self.npcs(map) {if prop.normal(){keys.push((prop.graphic(),prop.palette()));}}}
            keys.sort_unstable();keys.dedup();
            keys.into_iter().filter_map(|key| {
                let sheet=SpriteSheet::load(&root.join(gfx.get(&key.0)?),&root.join(palettes.get(&key.1)?))?;
                Some((key,sheet))
            }).collect()
        } else {HashMap::new()}
    }
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn target_narshe_trigger_and_npc_records_decode() {
  let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("extracted/twue-3.07");let events=FieldEvents::load(&root);
  assert_eq!(events.trigger_at(19,38,50),Some(0x029b1d));assert_eq!(events.trigger_at(19,38,38),Some(0x029b71));
  assert_eq!(events.trigger_at(19,38,49),None);assert_eq!(events.trigger_at(416,38,38),None);
  let npcs=events.npcs(19);assert_eq!(npcs.len(),5);
  assert_eq!((npcs[0].graphic(),npcs[0].palette(),npcs[0].switch()),(52,1,0x604));
  let mut state=EventState::default();events.initialize_switches(&mut state);
  assert!(state.switch(0x300));assert!(!state.switch(0x604));
  events.init_npcs(19,&mut state);assert!(!state.actors[16].enabled);
  state.set_switch(0x604,true);events.init_npcs(19,&mut state);assert!(state.actors[16].visible);
  assert_eq!((state.actors[16].speed,state.actors[16].graphic),(3,52));
 }
}
