//! LoadRageProp C2/2DC1..2E39: shared by monster setup and Rage.
//! Pending status words are requests, not committed current status.
use crate::battle_data::MonsterProperties;
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Traits {
    pub special:u8,pub weak_half:u16,pub allowed:[u8;4],
    pub pending12:u16,pub pending34:u16,pub relic45:u16,
    pub equip_status23:u16,pub absorb_null:u16,pub metamorph_flags:u16,
}
impl Traits {
    pub fn apply(&mut self,p:&MonsterProperties) {
        let r=p.raw;let word=|i|u16::from_le_bytes([r[i],r[i+1]]);
        self.special=r[31];self.weak_half|=u16::from(r[25]);
        self.allowed[2]&=!r[22];
        self.pending12=word(27);
        let status=word(29);
        self.pending34=(status|if status&1!=0 {0x8000}else{0})&0x84fe;
        if r[30]&1!=0 {self.relic45|=0x40;}
        self.equip_status23|=word(28);
        let allowed12=u16::from_le_bytes([self.allowed[0],self.allowed[1]])&!word(20);
        self.allowed[..2].copy_from_slice(&allowed12.to_le_bytes());
        self.absorb_null|=word(23);self.metamorph_flags=word(17);
    }
    pub fn encode(self)->Vec<u8> {
        let mut bytes=vec![self.special];bytes.extend(self.weak_half.to_le_bytes());bytes.extend(self.allowed);
        for v in [self.pending12,self.pending34,self.relic45,self.equip_status23,self.absorb_null,self.metamorph_flags] {bytes.extend(v.to_le_bytes());}bytes
    }
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let table=fs::read(root.join("build/native_battle/monster_properties.bin")).map_err(|e|e.to_string())?;
    if table.len()!=384*32 {return Err("Monster property table must have 384 records".into());}
    let mut seed=0x6ff6u32;let mut rows=String::from("case,id,before,after\n");
    for id in 0..384 {for n in 0..16 {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u16};
        let mut state=Traits {special:next() as u8,weak_half:next(),allowed:[next() as u8,next() as u8,next() as u8,next() as u8],pending12:next(),pending34:next(),relic45:next(),equip_status23:next(),absorb_null:next(),metamorph_flags:next()};
        let before=state.encode();state.apply(&MonsterProperties {raw:table[id*32..id*32+32].try_into().unwrap()});
        let hex=|a:&[u8]|a.iter().map(|v|format!("{v:02X}")).collect::<String>();
        writeln!(rows,"{},{id},{},{}",id*16+n,hex(&before),hex(&state.encode())).unwrap();
    }}
    fs::write(directory.join("traits.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn state()->Traits {Traits {special:0,weak_half:0x1200,allowed:[255;4],pending12:0xffff,pending34:0xffff,relic45:0x100,equip_status23:0x8000,absorb_null:0,metamorph_flags:0}}
    #[test]fn flying_becomes_pending_float_and_character_only_statuses_are_filtered() {
        let mut p=MonsterProperties {raw:[0;32]};p.raw[29]=255;p.raw[30]=255;
        let mut s=state();s.apply(&p);assert_eq!(s.pending34,0x84fe);assert_eq!(s.relic45,0x140);assert_eq!(s.pending12,0);
    }
    #[test]fn traits_merge_resistances_and_replace_pending_and_special_data() {
        let mut p=MonsterProperties {raw:[0;32]};p.raw[20]=0x80;p.raw[21]=4;p.raw[22]=0x10;p.raw[23]=2;p.raw[24]=8;p.raw[25]=1;p.raw[17]=3;p.raw[18]=0x80;p.raw[31]=0x20;
        let mut s=state();s.apply(&p);assert_eq!(s.allowed,[0x7f,0xfb,0xef,255]);assert_eq!((s.weak_half,s.absorb_null,s.metamorph_flags,s.special),(0x1201,0x0802,0x8003,0x20));
    }
}
