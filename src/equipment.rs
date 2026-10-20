//! Native equipment aggregation from C2/0E77..C2/1104. The scratch layout
//! preserves source arithmetic, including byte clamps and hand-mask indexing.
use std::{fs,path::Path};
use crate::characters::Character;

#[derive(Clone)]
pub struct ItemTable { records:Vec<u8>,names:Vec<String> }
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct EquipmentStats {
    /// Source $11A0..$11DF. Not a CPU or memory-bus implementation.
    pub raw:[u8;64],
    pub hp_boost:u8,
    pub mp_boost:u8,
}
impl EquipmentStats {
    pub fn vigor(&self)->u8 {self.raw[6]}
    pub fn speed(&self)->u8 {self.raw[4]}
    pub fn stamina(&self)->u8 {self.raw[2]}
    pub fn magic(&self)->u8 {self.raw[0]}
    pub fn defense(&self)->u8 {self.raw[0x1a]}
    pub fn magic_defense(&self)->u8 {self.raw[0x1b]}
    pub fn evade(&self)->u8 {self.raw[8]}
    pub fn magic_block(&self)->u8 {self.raw[10]}
    pub fn battle_strength(&self)->u8 {self.vigor().saturating_mul(2)}
    pub fn field_effects(&self)->u8 {self.raw[0x3f]}
}
fn word(raw:&[u8],i:usize)->u16 {u16::from_le_bytes([raw[i],raw[i+1]])}
fn set_word(raw:&mut[u8],i:usize,v:u16) {raw[i..i+2].copy_from_slice(&v.to_le_bytes());}
fn boost(flags:u8,base:u8)->u8 {
    if flags&base!=0 {0x80} else if flags&(base>>1)!=0 {0x40} else if flags&base.wrapping_mul(2)!=0 {0xc0} else {0}
}
impl ItemTable {
    pub fn load(root:&Path)->Result<Self,String> {
        let records=fs::read(root.join("build/native_items/item_properties.bin")).map_err(|e|format!("Item properties unavailable: {e}; run tools/extract_native_items.py"))?;
        if records.len()!=7680 {return Err("Item properties must contain 256 thirty-byte records".into());}
        let text=fs::read_to_string(root.join("assets/text/item_name.en.txt")).map_err(|e|e.to_string())?;
        let names=parse_names(&text)?;
        if names.len()!=256 {return Err("Expected 256 target item names".into());}
        Ok(Self {records,names})
    }
    pub fn name(&self,id:u8)->&str {&self.names[usize::from(id)]}
    pub fn record(&self,id:u8)->&[u8] {let i=usize::from(id)*30;&self.records[i..i+30]}
    pub fn calculate(&self,character:&Character,equipment:[u8;6],status1:u8)->Result<EquipmentStats,String> {
        let p=&character.properties;let mut out=[0u8;64];
        out[12]=p[10];out[13]=p[10];out[26]=p[11];out[27]=p[12];out[8]=p[13];out[10]=p[14];
        out[60]=((p[21]&3)^3)+2;
        for (i,source) in [9,8,7,6].iter().enumerate() {out[i*2]=p[*source];}
        out[38..44].copy_from_slice(&equipment);
        let evade=[0i16,10,20,30,40,50,-10,-20,-30,-40,-50];
        for slot in (0..6).rev() {
            let item=self.record(equipment[slot]);
            out[63]|=item[5];
            for i in 0..8 {out[50+i]|=item[6+i];}
            let modifiers=word(item,16);
            for i in 0..4 {
                let nibble=((modifiers>>(i*4))&15) as i16;
                let delta=if nibble&8!=0 {-(nibble&7)} else {nibble};
                let index=6-i*2;let sum=word(&out,index).wrapping_add(delta as u16);
                set_word(&mut out,index,sum);
            }
            for (index,nibble) in [(8,item[26]&15),(10,item[26]>>4)] {
                let delta=*evade.get(usize::from(nibble)).ok_or("Invalid equipment evade nibble")?;
                let sum=word(&out,index).wrapping_add(delta as u16);set_word(&mut out,index,sum);
            }
            let power=if (item[2]&0x40!=0)==(status1&0x20!=0) {item[20]} else {1};
            if item[0]&7==1 {
                if slot<2 {
                    let hand=slot;let mut mask=if hand==0 {4} else {2};
                    if equipment[slot]==255 {mask<<=2;}
                    out[46]|=mask;
                    out[18+hand]=item[22];out[16+hand]=item[15];
                    out[12+hand]=out[12+hand].saturating_add(power);
                    out[14+hand]=item[21];out[20+hand]=item[18];out[58+hand]=item[19];
                }
            } else {
                out[28]|=item[25];out[24]|=item[24];out[25]|=item[15];out[22]|=item[22];out[23]|=item[23];
                out[26]=out[26].saturating_add(power);out[27]=out[27].saturating_add(item[21]);
            }
            if out[53]&2!=0 {
                out[53]&=!2;let existing=out[55]&2;out[55]|=2;
                if existing!=0 {out[53]|=2;}
            }
            let block=item[27];out[30+slot]=block;
            let graphic=1<<(block&3);
            if block&4!=0 {out[48]|=graphic;}
            if block&8!=0 {out[49]|=graphic;}
        }
        for index in (0..12).step_by(2) {
            if out[index+1]!=0 {out[index]=if out[index+1]&0x80!=0 {0} else {255};}
        }
        let hands=out[46]>>1;
        match hands {
            1|4 => {out[58]&=!0x40;out[59]&=!0x40;out[12]=0;},
            9 => out[12]=0,
            2|8|12 => {out[58]&=!0x40;out[59]&=!0x40;out[13]=0;},
            6 => out[13]=0,
            3 => {out[47]|=0x10;out[58]&=!0x40;out[59]&=!0x40;},
            0|5|7|10|11 => {},
            _ => return Err("Equipment hand mask is outside the source jump table".into()),
        }
        if out[55]&0x80!=0 {let vigor=word(&out,6);set_word(&mut out,6,vigor.wrapping_add(vigor>>1));}
        Ok(EquipmentStats {hp_boost:boost(out[53],8),mp_boost:boost(out[53],0x40),raw:out})
    }
    pub fn capture(&self,root:&Path,directory:&Path)->Result<(),String> {
        use std::fmt::Write;
        fs::create_dir_all(directory).map_err(|e|e.to_string())?;
        let chars=crate::characters::CharacterTables::load(root)?;
        let mut rows=String::from("case,property,status,equipment,scratch,hp_boost,mp_boost,battle_strength\n");
        let mut emit=|case:&str,id:u8,status:u8,equipment:[u8;6]|->Result<(),String> {
            let c=chars.at_level(id,3)?;let stats=self.calculate(&c,equipment,status)?;
            let hex=|bytes:&[u8]|bytes.iter().map(|b|format!("{b:02X}")).collect::<String>();
            writeln!(rows,"{case},{id},{status},{},{},{},{},{}",hex(&equipment),hex(&stats.raw),stats.hp_boost,stats.mp_boost,stats.battle_strength()).unwrap();Ok(())
        };
        for (slot,c) in chars.opening_party()?.iter().enumerate() {if let Some(c)=c {emit(&format!("opening-{slot}"),c.property_id,0,c.equipment())?;}}
        for id in 0..=255 {for slot in 0..6 {for status in [0,0x20] {
            let mut equipment=[255;6];equipment[slot]=id;emit(&format!("item-{id}-slot-{slot}-status-{status}"),0,status,equipment)?;
        }}}
        for equipment in [[1,1,105,132,0xc4,0xc4],[255,1,105,132,255,255],[1,255,105,132,255,255],[255;6],[1,90,105,132,0xc6,0xc7]] {
            emit("combination",0,0,equipment)?;
        }
        fs::write(directory.join("equipment.csv"),rows).map_err(|e|e.to_string())
    }
}

fn parse_names(text:&str)->Result<Vec<String>,String> {
    text.split("#text:").skip(1).map(|section| {
        let (_,body)=section.split_once('\n').ok_or("Missing item text body")?;
        let mut name=body.trim();
        if name.starts_with('{') {
            let (symbol,rest)=name.split_once('}').ok_or("Unterminated item symbol")?;
            if !["{armor","{brush","{card","{claw","{dagger","{helm","{katana","{pad","{relic","{scroll","{shield","{spear","{special","{staff","{star","{sword","{tool"].contains(&symbol) {return Err("Unknown item symbol".into());}
            name=rest;
        }
        let name=name.replace("{pad}"," ");
        if name.contains(['{','}','\n','\r']) {return Err("Unsupported item-name controls".into());}
        Ok(name)
    }).collect()
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn sign_magnitude_zero_and_boost_priority() {
        assert_eq!(boost(0x1c,8),0x80);assert_eq!(boost(0x14,8),0x40);assert_eq!(boost(0x10,8),0xc0);
        assert_eq!(boost(0xe0,0x40),0x80);assert_eq!(boost(0xa0,0x40),0x40);assert_eq!(boost(0x80,0x40),0xc0);
    }
    #[test] fn target_name_symbols_are_explicitly_recognized() {
        assert_eq!(parse_names("#text:A\n{dagger}MythrilKnife\n\n#text:B\n{pad}Empty\n").unwrap(),vec!["MythrilKnife","Empty"]);
        assert!(parse_names("#text:A\n{unknown}Knife").is_err());
        assert!(parse_names("#text:A\n{dagger}Knife{n}Oops").is_err());
        assert_eq!(parse_names("#text:A\n{claw}Kaiser{pad}Claws").unwrap(),vec!["Kaiser Claws"]);
    }
}
