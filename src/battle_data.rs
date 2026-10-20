//! Fixed-table decoding, not a combat simulator. Source: C2/30E8, C2/2EE1,
//! C2/2C30 and data/monster_prop.mac at the pinned FF6 disassembly revision.
use std::{fs, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonsterProperties {
    pub raw: [u8; 32],
}
impl MonsterProperties {
    pub fn word(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self.raw[offset], self.raw[offset+1]])
    }
    pub fn hp(&self) -> u16 { self.word(8) }
    pub fn mp(&self) -> u16 { self.word(10) }
    pub fn exp(&self) -> u16 { self.word(12) }
    pub fn gil(&self) -> u16 { self.word(14) }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonsterSlot {
    pub slot: u8,
    pub monster_id: u16,
    pub present: bool,
    pub position_byte: u8,
    pub hp: u16,
    pub mp: u16,
    pub properties: MonsterProperties,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encounter {
    pub formation_id: u16,
    pub raw_properties: [u8; 4],
    pub loaded_properties: [u8; 4],
    pub raw_monsters: [u8; 15],
    pub monsters: Vec<MonsterSlot>,
}

#[derive(Clone, Default)]
pub struct BattleData {
    groups: Vec<u8>,
    formations: Vec<u8>,
    monsters: Vec<u8>,
    properties: Vec<u8>,
    special_animations: Vec<u8>,
}
impl BattleData {
    pub fn capture(&self, directory: &Path) -> Result<(),String> {
        use std::fmt::Write;
        fs::create_dir_all(directory).map_err(|e|e.to_string())?;
        let mut groups=String::from("group,rng,formation\n");
        for group in 0..=255 {for rng in [0,191,192,255] {
            writeln!(groups,"{group},{rng},{}",self.event_formation(group,rng)?).unwrap();
        }}
        let mut formations=String::from("formation,slot,monster,present,position,hp,mp,exp,gil,speed,level\n");
        for id in 0..576 {
            for m in self.encounter(id)?.monsters {
                writeln!(formations,"{id},{},{},{},{},{},{},{},{},{},{}",m.slot,m.monster_id,
                    u8::from(m.present),m.position_byte,m.hp,m.mp,m.properties.exp(),m.properties.gil(),m.properties.raw[0],m.properties.raw[16]).unwrap();
            }
        }
        fs::write(directory.join("event-groups.csv"),groups).map_err(|e|e.to_string())?;
        fs::write(directory.join("formation-monsters.csv"),formations).map_err(|e|e.to_string())?;
        let mut animations=String::from("monster,animation\n");
        for id in 0..384 {writeln!(animations,"{id},{}",self.special_animation(id)?).unwrap();}
        fs::write(directory.join("special-animations.csv"),animations).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn load(root: &Path) -> Result<Self, String> {
        let dir=root.join("build/native_battle");
        let read=|name: &str, size| -> Result<Vec<u8>,String> {
            let bytes=fs::read(dir.join(format!("{name}.bin"))).map_err(|e|format!("Battle table {name}: {e}"))?;
            if bytes.len()!=size { return Err(format!("Battle table {name}: expected {size} bytes, got {}",bytes.len())); }
            Ok(bytes)
        };
        Ok(Self {groups:read("event_groups",1024)?,formations:read("formations",2304)?,
            monsters:read("formation_monsters",8640)?,properties:read("monster_properties",12288)?,special_animations:read("monster_special_animations",384)?})
    }
    /// CF/37C0: source LoadMonsterProp selects the monster's special animation.
    pub fn special_animation(&self,monster:u16)->Result<u8,String> {
        self.special_animations.get(usize::from(monster)).copied().ok_or_else(||"Monster special animation index outside table".into())
    }
    /// C0/A5A7 selects the second word only when the supplied RNG byte >= C0.
    /// Caller supplies RNG output: this does not approximate the cartridge RNG.
    pub fn event_formation(&self, group: u8, rng: u8) -> Result<u16,String> {
        let offset=usize::from(group)*4+if rng>=0xc0 {2} else {0};
        let bytes=self.groups.get(offset..offset+2).ok_or("Event battle table unavailable")?;
        Ok(u16::from_le_bytes([bytes[0],bytes[1]]))
    }
    /// Decode a concrete formation. Conditional replacement and the high-bit
    /// random-four-formation mode require battle flags/RNG and are rejected.
    pub fn encounter(&self, id: u16) -> Result<Encounter,String> {
        let index=usize::from(id);
        let raw_properties: [u8;4]=self.formations.get(index*4..index*4+4)
            .ok_or_else(||format!("Unsupported formation {id:04X}"))?.try_into().unwrap();
        let raw_monsters: [u8;15]=self.monsters.get(index*15..index*15+15)
            .ok_or("Formation monster table unavailable")?.try_into().unwrap();
        let mut loaded_properties=raw_properties;
        loaded_properties[0]^=0xf0; // C2/3140 toggles battle type flags.
        let mut monsters=Vec::new();
        for slot in 0..6 {
            let monster_id=u16::from(raw_monsters[2+slot]) | (u16::from((raw_monsters[14]>>slot)&1)<<8);
            if monster_id==0x1ff {continue;}
            let offset=usize::from(monster_id)*32;
            let raw: [u8;32]=self.properties.get(offset..offset+32)
                .ok_or_else(||format!("Unsupported monster {monster_id:03X}"))?.try_into().unwrap();
            let properties=MonsterProperties {raw};
            monsters.push(MonsterSlot {slot:slot as u8,monster_id,present:raw_monsters[1]&(1<<slot)!=0,
                position_byte:raw_monsters[8+slot],hp:properties.hp(),mp:properties.mp(),properties});
        }
        Ok(Encounter {formation_id:id,raw_properties,loaded_properties,raw_monsters,monsters})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> BattleData {
        let mut data=BattleData {groups:vec![0;1024],formations:vec![0;2304],monsters:vec![0;8640],properties:vec![0;12288],special_animations:vec![0;384]};
        data.groups[4..8].copy_from_slice(&[2,0,2,0]);
        data.formations[8..12].copy_from_slice(&[0xe3,0,0,0]);
        data.monsters[30..45].copy_from_slice(&[0x80,0x0c,0xff,0xff,0,0,0xff,0xff,0,0,0x4b,0x87,0,0,0x33]);
        data.properties[8..16].copy_from_slice(&[40,0,15,0,48,0,48,0]);data
    }
    #[test] fn first_guard_has_two_guards_in_slots_two_and_three() {
        let data=sample();
        for rng in 0..=255 {assert_eq!(data.event_formation(1,rng),Ok(2));}
        let encounter=data.encounter(2).unwrap();
        assert_eq!(encounter.loaded_properties,[0x13,0,0,0]);
        assert_eq!(encounter.monsters.iter().map(|m|(m.slot,m.monster_id,m.present,m.hp,m.mp)).collect::<Vec<_>>(),vec![(2,0,true,40,15),(3,0,true,40,15)]);
    }
    #[test] fn selection_threshold_and_ninth_id_bit() {
        let mut data=sample();data.groups[6..8].copy_from_slice(&[3,0]);
        assert_eq!(data.event_formation(1,191),Ok(2));assert_eq!(data.event_formation(1,192),Ok(3));
        data.monsters[44]|=4;assert_eq!(data.encounter(2).unwrap().monsters[0].monster_id,256);
        assert!(data.encounter(0x8002).is_err());assert!(BattleData::default().event_formation(1,0).is_err());
    }
}
