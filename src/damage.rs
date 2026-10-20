//! Base damage translated from the target's rewritten C2/2B69 routine and
//! helpers C2/0D4A..0D86. Do not substitute the vanilla disassembly formula.
use std::{fs,path::Path};

#[derive(Clone,Copy,Debug,Default)]
pub struct BaseDamage {
    pub level:u8,
    pub power:u8,
    /// Source $11AE: projected Strength for physical, MagicPower otherwise.
    pub stat:u8,
    pub physical:bool,
    pub monster:bool,
    pub restorative:bool,
    pub affects_mp:bool,
    /// Source $B2 word. Bit 4000 skips the physical power's 1.75 scaling.
    pub attack_flags:u16,
    pub command:u8,
    /// Source $3C44 word: Atlas Armlet / single and double earrings bits.
    pub relic_damage:u16,
    pub relic4:u8,
}
impl BaseDamage {
    pub fn calculate(self)->u16 {
        let level=u64::from(self.level);let power=u64::from(self.power);let stat=u64::from(self.stat);
        let mut damage=if self.physical {
            let p=power*if self.monster {4} else {1};
            let adjusted=if self.attack_flags&0x4000!=0 {p} else {(p*7+3)/4};
            let mut value=((adjusted+stat)*level*level)/256;
            if !self.monster {
                value=(value*3+1)/2+p;
                if self.command==0 || self.command==6 {
                    if self.relic4&1!=0 {value=(value+1)/2;}
                    if self.relic4&0x10!=0 {value=(value*3+3)/4;}
                }
            }
            value
        } else {
            power*if self.level==0 {1} else {4}+(power*level*stat)/32
        };
        if !self.restorative {
            if self.physical && !self.affects_mp {
                if self.relic_damage&1!=0 {damage+=damage/4;}
            } else if self.relic_damage&2!=0 {damage+=damage/2;}
            else if self.relic_damage&0x200!=0 {damage+=damage/4;}
        }
        damage.min(65535) as u16
    }
}

/// Target C2/0C9E: effects applied before elemental interactions and hit-point
/// application. RNG is supplied by the caller, not replaced with host randomness.
#[derive(Clone,Copy,Debug)]
pub struct Modifiers {
    pub rng:u8,
    pub flags:[u8;3],
    pub element:u8,
    pub attacker_monster:bool,
    pub target_monster:bool,
    pub defense:u8,
    pub magic_defense:u8,
    pub status3:u8,
    pub status4:u8,
    pub target_flags:u8,
    pub normal_defense:bool,
    pub enabled:bool,
    pub attack_flags:u16,
    pub multiplier:u8,
}
impl Default for Modifiers {
    fn default()->Self {Self {rng:0,flags:[0;3],element:0,attacker_monster:false,target_monster:true,defense:0,magic_defense:0,status3:0,status4:0,target_flags:0,normal_defense:true,enabled:true,attack_flags:0,multiplier:0}}
}
impl Modifiers {
    pub fn calculate(self,base:u16)->u16 {
        if !self.enabled {return base;}
        let mult=|value:u32,factor:u32|value*factor/256+1;
        let mut damage=mult(u32::from(base),u32::from(self.rng|0xe0));
        let physical=self.flags[0]&1!=0 && self.flags[1]&0x80==0;
        if self.flags[0]&0x20==0 {
            let defense=if physical {self.defense} else {self.magic_defense};
            let defense=if defense==255 || self.normal_defense {defense} else {192};
            damage=mult(damage,255-u32::from(defense));
            if self.status3&(if physical {0x40} else {0x20})!=0 {damage=mult(damage,0xaa);}
            if physical {
                if self.target_flags&2!=0 {damage/=2;}
                if self.target_flags&0x20!=0 {damage/=2;}
            } else if self.status4&8!=0 {damage/=2;}
        }
        if self.flags[2]&1==0 && !self.attacker_monster && !self.target_monster {damage/=2;}
        let flag_word=u16::from(self.element)|(u16::from(self.flags[0])<<8);
        if ((u32::from(self.attack_flags)<<1)&u32::from(flag_word)&0x2000)==0 {
            damage=(damage+(damage/2)*u32::from(self.multiplier)).min(65535);
        }
        damage as u16
    }
}

#[derive(Clone)] pub struct AttackTable {records:Vec<u8>}
impl AttackTable {
    pub fn load(root:&Path)->Result<Self,String> {
        let records=fs::read(root.join("build/native_magic/attack_properties.bin")).map_err(|e|format!("Attack properties unavailable: {e}; run tools/extract_native_magic.py"))?;
        if records.len()!=3584 {return Err("Attack table must contain 256 fourteen-byte records".into());}Ok(Self {records})
    }
    pub fn record(&self,id:u8)->&[u8] {let i=usize::from(id)*14;&self.records[i..i+14]}
    pub fn base(&self,id:u8,level:u8,stat:u8,monster:bool,command:u8)->BaseDamage {
        let r=self.record(id);
        BaseDamage {level,power:r[6],stat,physical:r[2]&1!=0,monster,restorative:r[4]&1!=0,affects_mp:r[3]&0x80!=0,command,..BaseDamage::default()}
    }
}

pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let mut rows=String::from("level,power,stat,physical,monster,restorative,affects_mp,attack_flags,command,relic_damage,relic4,damage\n");
    let mut emit=|d:BaseDamage| {
        writeln!(rows,"{},{},{},{},{},{},{},{},{},{},{},{}",d.level,d.power,d.stat,u8::from(d.physical),u8::from(d.monster),u8::from(d.restorative),u8::from(d.affects_mp),d.attack_flags,d.command,d.relic_damage,d.relic4,d.calculate()).unwrap();
    };
    for level in [0,1,3,50,99,255] {for power in [0,1,60,255] {for stat in [0,39,255] {for physical in [false,true] {for monster in [false,true] {for flags in [0,0x4000] {
        emit(BaseDamage {level,power,stat,physical,monster,attack_flags:flags,..BaseDamage::default()});
    }}}}}}
    let mut seed=0x6ff6_u32;
    for _ in 0..8192 {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let level=next();let power=next();let stat=next();let flags=next();
        let command=[0,6,29,2,7][usize::from(next()%5)];
        let relic_damage=[0,1,2,0x200,0x201,0x202,0x203][usize::from(next()%7)];
        let relic4=[0,1,0x10,0x11][usize::from(next()%4)];
        emit(BaseDamage {level,power,stat,physical:flags&1!=0,monster:flags&2!=0,restorative:flags&4!=0,affects_mp:flags&8!=0,attack_flags:if flags&16!=0 {0x4000} else {0},command,relic_damage,relic4});
    }
    fs::write(directory.join("base-damage.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("base,rng,flags0,flags1,flags2,element,attacker_monster,target_monster,defense,magic_defense,status3,status4,target_flags,normal_defense,enabled,attack_flags,multiplier,damage\n");
    for index in 0..4096 {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let base=(u16::from(next())<<8)|u16::from(next());let rng=next();let choose=next();
        let flags=[(choose&1)|if choose&2!=0 {0x20}else{0},if choose&4!=0 {0x80}else{0},if choose&8!=0 {1}else{0}];
        let element=next();let defense=next();let magic_defense=next();let status3=next();let status4=next();let target_flags=next();
        let context=Modifiers {rng,flags,element,attacker_monster:choose&16!=0,target_monster:choose&32!=0,defense,magic_defense,status3,status4,target_flags,normal_defense:choose&64!=0,enabled:index%7!=0,attack_flags:if choose&128!=0 {0x1000}else{0},multiplier:next()%9};
        writeln!(rows,"{base},{rng},{},{},{},{element},{},{},{defense},{magic_defense},{status3},{status4},{target_flags},{},{},{},{},{}",flags[0],flags[1],flags[2],u8::from(context.attacker_monster),u8::from(context.target_monster),u8::from(context.normal_defense),u8::from(context.enabled),context.attack_flags,context.multiplier,context.calculate(base)).unwrap();
    }
    fs::write(directory.join("damage-modifiers.csv"),rows).map_err(|e|e.to_string())?;
    let attacks=AttackTable::load(root)?;let chars=crate::characters::CharacterTables::load(root)?;let items=crate::equipment::ItemTable::load(root)?;
    let mut rows=String::from("slot,attack,level,power,magic,mp_cost,base_damage\n");
    let mut target_rows=String::from("slot,attack,rng,base,modified\n");
    for (slot,c) in chars.opening_party()?.iter().enumerate() {if let Some(c)=c {
        let stats=items.calculate(c,c.equipment(),0)?;
        for attack in [131,132,133,135] {let d=attacks.base(attack,c.level,stats.magic(),false,29);
            writeln!(rows,"{slot},{attack},{},{},{},{},{}",c.level,d.power,stats.magic(),attacks.record(attack)[5],d.calculate()).unwrap();
            let r=attacks.record(attack);
            for rng in 0..=255 {
                let modifiers=Modifiers {rng,flags:r[2..5].try_into().unwrap(),element:r[1],magic_defense:140,..Modifiers::default()};
                writeln!(target_rows,"{slot},{attack},{rng},{},{}",d.calculate(),modifiers.calculate(d.calculate())).unwrap();
            }
        }
    }}
    fs::write(directory.join("opening-magitek-base.csv"),rows).map_err(|e|e.to_string())?;
    fs::write(directory.join("opening-magitek-modifiers.csv"),target_rows).map_err(|e|e.to_string())
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn magic_zero_level_overflow_and_restorative_relics() {
        let d=BaseDamage {level:3,power:60,stat:39,..BaseDamage::default()};
        assert_eq!(d.calculate(),459);assert_eq!(BaseDamage {level:0,..d}.calculate(),60);
        assert_eq!(BaseDamage {level:99,power:255,stat:255,..d}.calculate(),65535);
        assert_eq!(BaseDamage {relic_damage:0x200,..d}.calculate(),573);
        assert_eq!(BaseDamage {relic_damage:2,..d}.calculate(),688);
        assert_eq!(BaseDamage {relic_damage:2,restorative:true,..d}.calculate(),459);
    }
    #[test] fn physical_relic_reductions_apply_only_to_fight_or_capture() {
        let d=BaseDamage {level:3,power:42,stat:62,physical:true,..BaseDamage::default()};
        assert_eq!(d.calculate(),48);
        assert_eq!(BaseDamage {relic4:1,..d}.calculate(),24);
        assert_eq!(BaseDamage {relic4:16,..d}.calculate(),36);
        assert_eq!(BaseDamage {relic4:17,..d}.calculate(),18);
        assert_eq!(BaseDamage {command:29,relic4:17,..d}.calculate(),48);
        assert_eq!(BaseDamage {monster:true,..d}.calculate(),12);
    }
    #[test] fn modifier_variance_defense_and_zero_damage_rounding() {
        let m=Modifiers {magic_defense:140,..Modifiers::default()};
        assert_eq!(m.calculate(459),181);assert_eq!(Modifiers {rng:255,..m}.calculate(459),206);
        assert_eq!(Modifiers {magic_defense:255,..m}.calculate(459),1);
        assert_eq!(Modifiers {enabled:false,..m}.calculate(459),459);
        assert_eq!(Modifiers {flags:[0x20,0,0],..m}.calculate(459),402);
    }
}
