//! Character equipment-to-battle projection C2/2883..2943. Sprite selection,
//! visual flags and subsequent FixImmuneStatus are separate operations.
use crate::equipment::EquipmentStats;
use std::{fs,path::Path};
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct BattleEquipment {
    pub strength:u8,pub speed:u8,pub stamina:u8,pub magic:u8,pub evade:u8,pub magic_evade:u8,
    pub equip_status:u16,pub run_factor:u8,pub monster_flags:u8,pub relic13:u16,
    pub hand_power:u16,pub hand_hit:u16,pub hand_spell:u16,pub hand_element:u16,
    pub hand_flags:u16,pub defense:u16,pub immune12:u16,pub absorb_null:u16,
    pub weak_half:u16,pub hand_special:u16,pub hand_items:u16,pub relics:u16,
    pub block_gfx:u16,pub relic45:u16,pub genji:u8,pub party_relics:u8,
}
/// Source doubles evade, saturates on carry, negates, then replaces zero
/// with FF. Thus raw 0 -> FF, 127 -> 2, 128..255 -> 1.
pub fn invert_evade(raw:u8)->u8 {if raw==0 {255}else{raw.saturating_mul(2).wrapping_neg()}}
pub fn project(stats:&EquipmentStats,party_relics:u8)->(BattleEquipment,[u8;64]) {
    let mut r=stats.raw;let word=|r:&[u8],i:usize|u16::from_le_bytes([r[i],r[i+1]]);
    let genji=r[0x38]&0x10;
    r[0x38]&=!r[0x2f];
    if r[0x38]&8==0 {r[0x3a]&=!0x40;r[0x3b]&=!0x40;}
    let out=BattleEquipment {
        strength:r[6].saturating_mul(2),speed:r[4],stamina:r[2],magic:r[0],evade:invert_evade(r[8]),magic_evade:invert_evade(r[10]),
        equip_status:u16::from(r[0x1c])|(u16::from(r[0x34])<<8),run_factor:r[0x3c],monster_flags:(r[0x39]&0x80)|0x10,
        relic13:u16::from((r[0x35]&0x7f)|(r[0x36]&0x80))|(u16::from(r[0x37])<<8),
        hand_power:word(&r,12),hand_hit:word(&r,14),hand_spell:word(&r,20),hand_element:word(&r,16),hand_flags:word(&r,58),
        defense:word(&r,26),immune12:!word(&r,50),absorb_null:word(&r,22),weak_half:word(&r,24),hand_special:word(&r,30),
        hand_items:word(&r,38),relics:word(&r,42),block_gfx:word(&r,48),relic45:word(&r,56),genji,party_relics:party_relics|r[0x36],
    };(out,r)
}
impl BattleEquipment {
    pub fn encode(&self)->Vec<u8> {
        let mut b=vec![self.strength,self.speed,self.speed,self.stamina,self.magic,self.evade,self.magic_evade];
        b.extend(self.equip_status.to_le_bytes());b.extend([self.run_factor,self.monster_flags]);
        for word in [self.relic13,self.hand_power,self.hand_hit,self.hand_spell,self.hand_element,self.hand_flags,self.defense,self.immune12,self.absorb_null,self.weak_half,self.hand_special,self.hand_items,self.relics,self.block_gfx,self.relic45] {b.extend(word.to_le_bytes());}
        b.extend([self.genji,self.party_relics]);b
    }
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,scratch,party,result,scratch_out\n");let mut seed=0x6ff6u32;
    for case in 0..8192 {
        let mut raw=[0u8;64];for v in &mut raw {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);*v=(seed>>16) as u8;}
        let party=(case%256) as u8;let (out,after)=project(&EquipmentStats {raw,hp_boost:0,mp_boost:0},party);
        let hex=|bytes:&[u8]|bytes.iter().map(|b|format!("{b:02X}")).collect::<String>();
        writeln!(rows,"{case},{},{party},{},{}",hex(&raw),hex(&out.encode()),hex(&after)).unwrap();
    }
    let chars=crate::characters::CharacterTables::load(root)?;
    let items=crate::equipment::ItemTable::load(root)?;
    let mut party=0;let mut case=8192;
    for character in chars.opening_party()?.iter().flatten() {
        let stats=items.calculate(character,character.equipment(),0)?;
        let (out,after)=project(&stats,party);
        let hex=|bytes:&[u8]|bytes.iter().map(|b|format!("{b:02X}")).collect::<String>();
        writeln!(rows,"{case},{},{party},{},{}",hex(&stats.raw),hex(&out.encode()),hex(&after)).unwrap();
        party=out.party_relics;case+=1;
    }
    fs::write(directory.join("projection.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn evade_zero_and_overflow_are_distinct() {assert_eq!([invert_evade(0),invert_evade(1),invert_evade(127),invert_evade(128),invert_evade(255)],[255,254,2,1,1]);}
    #[test]fn hand_mask_removes_genji_after_display_flag_is_recorded() {
        let mut raw=[0u8;64];raw[0x38]=0x10;raw[0x2f]=0x10;raw[0x3a]=0xff;raw[0x3b]=0xff;
        let (out,after)=project(&EquipmentStats {raw,hp_boost:0,mp_boost:0},0);
        assert_eq!(out.genji,0x10);assert_eq!(out.relic45,0);assert_eq!(out.hand_flags,0xbfbf);assert_eq!(after[0x38],0);
    }
    #[test]fn opening_terra_uses_equipment_derived_battle_evasion() {
        let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("extracted/twue-3.07");
        let tables=crate::characters::CharacterTables::load(&root).unwrap();
        let roster=tables.opening_party().unwrap();let terra=roster[0].as_ref().unwrap();
        let items=crate::equipment::ItemTable::load(&root).unwrap();
        let stats=items.calculate(terra,terra.equipment(),0).unwrap();let (battle,_)=project(&stats,0);
        assert_eq!((battle.strength,battle.speed,battle.stamina,battle.magic),(62,33,28,39));
        assert_eq!((battle.evade,battle.magic_evade),(226,242));
        use crate::{battle_hit::{self,Context,State,Outcome},battle_hit_gates::Context as Gates,battle_rng::{BattleRng,RngTable}};
        let mut state=State::default();let mut rng=BattleRng::default();
        let c=Context {gates:Gates {command:29,flags0:1,hit_rate:100,magic_evade:battle.magic_evade,..Gates::default()},evade:battle.evade,physical_block:battle.block_gfx as u8,magical_block:(battle.block_gfx>>8) as u8,..Context::default()};
        assert_eq!(battle_hit::check(c,&mut state,&mut rng,&RngTable::identity()).unwrap(),Outcome::Hit);
        assert_eq!(rng.index,1);
    }
}
