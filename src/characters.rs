//! Character property assignment and base growth from field EventCmd_40,
//! CalcAverageLevel, InitMaxHP/MP and UpdateExp. Equipment-derived combat
//! properties and natural ability learning are not calculated here yet.
use std::{fs,path::Path};

#[derive(Clone)]
pub struct CharacterTables { properties:Vec<u8>, hp:Vec<u8>, mp:Vec<u8>, exp:Vec<u16> }

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Character {
    pub property_id:u8,
    pub properties:[u8;22],
    pub level:u8,
    pub hp:u16,
    pub max_hp:u16,
    pub mp:u16,
    pub max_mp:u16,
    pub exp:u32,
}
impl Character {
    pub fn commands(&self)->[u8;4] {self.properties[2..6].try_into().unwrap()}
    pub fn equipment(&self)->[u8;6] {self.properties[15..21].try_into().unwrap()}
    pub fn speed(&self)->u8 {self.properties[7]}
}

pub type Roster=[Option<Character>;16];

pub fn encode_roster(roster:&Roster)->String {
    let mut bytes=b"F6CH1".to_vec();
    for c in roster {
        if let Some(c)=c {
            bytes.extend_from_slice(&[1,c.property_id,c.level]);bytes.extend_from_slice(&c.properties);
            for value in [c.hp,c.max_hp,c.mp,c.max_mp] {bytes.extend_from_slice(&value.to_le_bytes());}
            bytes.extend_from_slice(&c.exp.to_le_bytes()[..3]);
        } else {bytes.extend_from_slice(&[0;36]);}
    }
    let checksum=bytes.iter().fold(0x811c9dc5u32,|h,b|(h^u32::from(*b)).wrapping_mul(0x01000193));
    bytes.extend_from_slice(&checksum.to_le_bytes());
    bytes.iter().map(|b|format!("{b:02x}")).collect()
}

pub fn decode_roster(hex:&str)->Result<Roster,String> {
    if hex.len()!=585*2 || !hex.is_ascii() {return Err("Invalid character snapshot length".into());}
    let bytes=(0..hex.len()).step_by(2).map(|i|u8::from_str_radix(&hex[i..i+2],16)).collect::<Result<Vec<_>,_>>().map_err(|_|"Invalid character snapshot hex")?;
    let end=bytes.len()-4;
    let checksum=bytes[..end].iter().fold(0x811c9dc5u32,|h,b|(h^u32::from(*b)).wrapping_mul(0x01000193));
    if &bytes[..5]!=b"F6CH1" || checksum!=u32::from_le_bytes(bytes[end..].try_into().unwrap()) {return Err("Invalid character snapshot checksum/version".into());}
    let mut roster:Roster=std::array::from_fn(|_|None);
    for (slot,b) in bytes[5..end].chunks_exact(36).enumerate() {
        if b[0]==0 {if b.iter().any(|v|*v!=0) {return Err("Invalid empty character record".into());}continue;}
        if b[0]!=1 || b[1]>=64 || !(1..=99).contains(&b[2]) {return Err("Invalid character property/level".into());}
        let word=|i|u16::from_le_bytes([b[i],b[i+1]]);
        let c=Character {property_id:b[1],level:b[2],properties:b[3..25].try_into().unwrap(),hp:word(25),max_hp:word(27),mp:word(29),max_mp:word(31),exp:u32::from_le_bytes([b[33],b[34],b[35],0])};
        if c.hp>c.max_hp || c.mp>c.max_mp {return Err("Character HP/MP exceeds maximum".into());}
        roster[slot]=Some(c);
    }
    Ok(roster)
}
impl CharacterTables {
    pub fn next_level_exp(&self,level:u8)->Option<u32> {
        if !(1..99).contains(&level) {return None;}
        Some(self.exp[..usize::from(level)].iter().map(|v|u32::from(*v)).sum::<u32>()*8)
    }
    pub fn load(root:&Path)->Result<Self,String> {
        let dir=root.join("build/native_characters");
        let read=|name:&str,size|->Result<Vec<u8>,String> {
            let bytes=fs::read(dir.join(format!("{name}.bin"))).map_err(|e|format!("Character table {name}: {e}"))?;
            if bytes.len()!=size {return Err(format!("Character table {name} has invalid length"));}Ok(bytes)
        };
        let exp=read("level_up_exp",196)?.chunks_exact(2).map(|b|u16::from_le_bytes([b[0],b[1]])).collect();
        Ok(Self {properties:read("character_properties",1408)?,hp:read("level_up_hp",98)?,mp:read("level_up_mp",98)?,exp})
    }
    pub fn at_level(&self,property_id:u8,level:u8)->Result<Character,String> {
        if !(1..=99).contains(&level) {return Err("Character level outside 1..99".into());}
        let offset=usize::from(property_id)*22;
        let properties:[u8;22]=self.properties.get(offset..offset+22).ok_or("Character property ID outside table")?.try_into().unwrap();
        let count=usize::from(level-1);
        let hp=u16::from(properties[0])+self.hp[..count].iter().map(|v|u16::from(*v)).sum::<u16>();
        let mp=u16::from(properties[1])+self.mp[..count].iter().map(|v|u16::from(*v)).sum::<u16>();
        let exp=self.exp[..count].iter().map(|v|u32::from(*v)).sum::<u32>()*8;
        Ok(Character {property_id,properties,level,hp,max_hp:hp,mp,max_mp:mp,exp})
    }
    /// Event $40 computes the average from available slots 0..13 before
    /// applying the assigned property's level modifier. Guest slots excluded.
    pub fn assign(&self,roster:&Roster,available:u16,property_id:u8)->Result<Character,String> {
        let mut sum=0u16;let mut count=0u16;
        for (slot,character) in roster.iter().enumerate().take(14) {
            if available&(1<<slot)!=0 {
                let character=character.as_ref().ok_or("Available character has no modeled properties")?;
                sum+=u16::from(character.level);count+=1;
            }
        }
        let average=if count==0 {3} else {(sum/count).min(99) as u8};
        let flags=*self.properties.get(usize::from(property_id)*22+21).ok_or("Character property ID outside table")?;
        let modifier=[0i16,2,5,-3][usize::from((flags&12)>>2)];
        let level=(i16::from(average)+modifier).clamp(1,99) as u8;
        self.at_level(property_id,level)
    }
    pub fn opening_party(&self)->Result<Roster,String> {
        let mut roster:Roster=std::array::from_fn(|_|None);
        roster[0]=Some(self.assign(&roster,0,0)?);
        // GameStart sets switch 02F0 after Terra assignment; this is bit 0
        // of the 1EDE availability word used by CalcAverageLevel.
        roster[14]=Some(self.assign(&roster,1,32)?);
        roster[15]=Some(self.assign(&roster,1,33)?);
        Ok(roster)
    }
    pub fn capture(&self,directory:&Path)->Result<(),String> {
        use std::fmt::Write;
        fs::create_dir_all(directory).map_err(|e|e.to_string())?;
        let mut rows=String::from("property,level,hp,mp,exp,speed,commands,equipment\n");
        for id in 0..64 {for level in 1..=99 {
            let c=self.at_level(id,level)?;
            let hex=|values:&[u8]|values.iter().map(|v|format!("{v:02X}")).collect::<String>();
            writeln!(rows,"{id},{level},{},{},{},{},{},{}",c.hp,c.mp,c.exp,c.speed(),hex(&c.commands()),hex(&c.equipment())).unwrap();
        }}
        fs::write(directory.join("levels.csv"),rows).map_err(|e|e.to_string())?;
        let mut rows=String::from("slot,property,level,hp,mp,exp\n");
        for (slot,c) in self.opening_party()?.iter().enumerate() {if let Some(c)=c {
            writeln!(rows,"{slot},{},{},{},{},{}",c.property_id,c.level,c.hp,c.mp,c.exp).unwrap();
        }}
        fs::write(directory.join("opening-party.csv"),rows).map_err(|e|e.to_string())
    }
}

#[cfg(test)] mod tests {
    use super::*;
    fn tables()->CharacterTables {
        let mut properties=vec![0;1408];properties[0]=40;properties[1]=16;
        properties[32*22]=68;properties[32*22+21]=0x1d;
        properties[33*22]=70;properties[33*22+21]=0x1d;
        let mut hp=vec![1;98];hp[..2].copy_from_slice(&[11,12]);
        let mut mp=vec![1;98];mp[..2].copy_from_slice(&[5,6]);
        let mut exp=vec![1;98];exp[..2].copy_from_slice(&[4,8]);
        CharacterTables {properties,hp,mp,exp}
    }
    #[test] fn opening_guests_use_property_ids_not_party_slot_ids() {
        let t=tables();let p=t.opening_party().unwrap();let terra=p[0].as_ref().unwrap();
        assert_eq!((terra.level,terra.hp,terra.mp,terra.exp),(3,63,27,96));
        assert_eq!(t.next_level_exp(3),Some(104)); // fixture third gain is 1
        assert_eq!(t.next_level_exp(99),None);
        for (slot,id,hp) in [(14,32,68),(15,33,70)] {let c=p[slot].as_ref().unwrap();assert_eq!((c.property_id,c.level,c.hp,c.mp,c.exp),(id,1,hp,0,0));}
    }
    #[test] fn averages_floor_exclude_guests_and_clamp_modifiers() {
        let mut t=tables();let mut p=t.opening_party().unwrap();
        p[0]=Some(t.at_level(0,98).unwrap());p[1]=Some(t.at_level(0,99).unwrap());
        t.properties[21]=8;
        assert_eq!(t.assign(&p,3,0).unwrap().level,99);
        t.properties[21]=0;assert_eq!(t.assign(&p,3,0).unwrap().level,98);
        assert_eq!(t.assign(&p,0xc000,0).unwrap().level,3);
        assert!(t.assign(&p,4,0).is_err());assert!(t.at_level(64,1).is_err());assert!(t.at_level(0,0).is_err());
    }
    #[test] fn roster_snapshot_preserves_guests_and_rejects_corruption() {
        let mut p=tables().opening_party().unwrap();p[0].as_mut().unwrap().hp=4;
        let encoded=encode_roster(&p);assert_eq!(decode_roster(&encoded).unwrap(),p);
        let mut bad=encoded.into_bytes();bad[30]=if bad[30]==b'0' {b'1'} else {b'0'};
        assert!(decode_roster(std::str::from_utf8(&bad).unwrap()).is_err());
        assert!(decode_roster("bad").is_err());
    }
}
