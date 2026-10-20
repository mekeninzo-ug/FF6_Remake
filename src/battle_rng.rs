//! Table/index RNG from C2/4B5A and C2/4B65; bit selection C2/522A.
//! Clock seeding is explicit. No host RNG is used for battle behavior.
use std::{fs,path::Path};

#[derive(Clone)] pub struct RngTable {values:[u8;256]}
impl RngTable {
    #[cfg(test)] pub(crate) fn identity()->Self {Self {values:std::array::from_fn(|i|i as u8)}}
    pub fn load(root:&Path)->Result<Self,String> {
        let bytes=fs::read(root.join("build/native_battle/rng_table.bin")).map_err(|e|format!("Battle RNG table: {e}; run tools/extract_native_battle.py"))?;
        let values=bytes.try_into().map_err(|_|"Battle RNG table must contain 256 bytes")?;
        Ok(Self {values})
    }
}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct BattleRng {pub index:u8}
impl BattleRng {
    pub fn from_game_frames(frames:u8)->Self {Self {index:frames.wrapping_mul(4)}}
    /// C2/0019 also advances the index without reading a table value.
    /// The live main-loop caller must preserve that separate operation.
    pub fn advance_index(&mut self) {self.index=self.index.wrapping_add(1);}
    pub fn next(&mut self,table:&RngTable)->u8 {
        self.advance_index();table.values[usize::from(self.index)]
    }
    /// C2/4B65: consumes a byte even for bound 0 or 1, returns high product byte.
    pub fn below(&mut self,table:&RngTable,bound:u8)->u8 {
        ((u16::from(self.next(table))*u16::from(bound))>>8) as u8
    }
    pub fn bit(&mut self,table:&RngTable,mask:u16)->u16 {
        if mask==0 {return 0;}
        let mut rank=self.below(table,mask.count_ones() as u8);
        for index in 0..16 {
            let bit=1u16<<index;
            if mask&bit!=0 {if rank==0 {return bit;}rank-=1;}
        }
        unreachable!("RandA rank is strictly less than popcount")
    }
}

#[derive(Clone,Copy,Debug)]
pub struct GaugeActor {pub present:bool,pub speed:u8,pub gauge:u16}
#[derive(Clone,Copy,Debug,Default)]
pub struct GaugeContext {pub battle_type:u8,pub battle_flags:u8,pub enemy_characters:u8,pub scrolling_state:u8}

/// C2/2575. All ten slots consume a shuffled rank, even if not present.
/// Their existing gauge/scroll state determines whether RandA(speed) is called.
pub fn initialize_gauges(table:&RngTable,rng:&mut BattleRng,actors:&mut[GaugeActor;10],context:GaugeContext)->Result<(),String> {
    if context.battle_type>3 {return Err("Unsupported battle type for initial gauges".into());}
    let absent=actors.iter().filter(|a|!a.present).count() as u16*16;
    let mut remaining=0x3ff;
    for slot in (0..10).rev() {
        let selected=rng.bit(table,remaining);remaining&=!selected;
        let rank=selected.trailing_zeros() as u16*8;
        let actor=&mut actors[slot];
        if actor.gauge>>8!=255 || context.scrolling_state!=255 {continue;}
        let enemy=slot>=4 || context.enemy_characters&(1<<slot)!=0;
        let first=context.battle_flags&0x40!=0;
        let value=if !enemy && (first || context.battle_type==3) {continue;}
        else if enemy && (first || context.battle_type==3) {2}
        else if !enemy && context.battle_type!=0 {(rank+1).min(255)}
        else {
            // RandA preserves carry. For normal characters, the immediately
            // preceding ASL of battle flags supplies bit 7 to the first ADC.
            let carry=u16::from(!enemy && context.battle_flags&0x80!=0);
            (u16::from(rng.below(table,actor.speed))+u16::from(actor.speed)+carry+rank+absent+1).min(255)
        };
        actor.gauge=(value<<8)|(actor.gauge&255);
    }
    Ok(())
}

pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let table=RngTable::load(root)?;
    let mut rows=String::from("seed,bound,result,index\n");
    for seed in 0..=255 {for bound in 0..=255 {let mut rng=BattleRng {index:seed};let value=rng.below(&table,bound);writeln!(rows,"{seed},{bound},{value},{}",rng.index).unwrap();}}
    fs::write(directory.join("bounded.csv"),rows).map_err(|e|e.to_string())?;
    let masks=[0,1,0x8000,0xffff,0x3ff,0x0109,0xAAAA,0x5555];let mut rows=String::from("seed,mask,result,index\n");
    for seed in 0..=255 {for mask in masks {let mut rng=BattleRng {index:seed};let result=rng.bit(&table,mask);writeln!(rows,"{seed},{mask},{result},{}",rng.index).unwrap();}}
    fs::write(directory.join("bits.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("frame,seed,step,value,index\n");
    for frame in [0,1,63,64,255] {let mut rng=BattleRng::from_game_frames(frame);let seed=rng.index;
        for step in 0..512 {let value=rng.next(&table);writeln!(rows,"{frame},{seed},{step},{value},{}",rng.index).unwrap();}
    }
    fs::write(directory.join("sequences.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,seed,battle_type,battle_flags,enemy_characters,scrolling,present,speeds,before,after,index\n");
    for case in 0..1024 {
        let seed=case as u8;let context=GaugeContext {battle_type:(case%4) as u8,battle_flags:[0,0x40,0x80,0xc0][case/4%4],enemy_characters:[0,1,5,15][case/16%4],scrolling_state:if case%11==0 {0} else {255}};
        let mut actors=std::array::from_fn(|slot|GaugeActor {present:(case+slot)%3!=0,speed:[0,1,30,33,127,255][(case+slot)%6],gauge:if (case+slot)%7==0 {0x3a45} else {0xff00|(slot as u16*13)}});
        let mask=actors.iter().enumerate().fold(0u16,|mask,(slot,a)|mask|if a.present {1<<slot} else {0});
        let speeds=actors.iter().map(|a|format!("{:02X}",a.speed)).collect::<String>();
        let before=actors.iter().map(|a|format!("{:04X}",a.gauge)).collect::<String>();
        let mut rng=BattleRng {index:seed};initialize_gauges(&table,&mut rng,&mut actors,context)?;
        let after=actors.iter().map(|a|format!("{:04X}",a.gauge)).collect::<String>();
        writeln!(rows,"{case},{seed},{},{},{},{},{mask},{speeds},{before},{after},{}",context.battle_type,context.battle_flags,context.enemy_characters,context.scrolling_state,rng.index).unwrap();
    }
    fs::write(directory.join("initial-gauges.csv"),rows).map_err(|e|e.to_string())
}

#[cfg(test)] mod tests {
    use super::*;
    fn table()->RngTable {RngTable {values:std::array::from_fn(|i|i as u8)}}
    #[test] fn increment_wrap_scale_and_zero_bound_consume_source_bytes() {
        let t=table();let mut rng=BattleRng {index:255};assert_eq!(rng.next(&t),0);
        assert_eq!(rng.below(&t,0),0);assert_eq!(rng.index,1);
        assert_eq!(BattleRng::from_game_frames(64).index,0);
        assert_eq!(BattleRng {index:254}.below(&t,255),254);
        rng.index=255;rng.advance_index();assert_eq!(rng.index,0);assert_eq!(rng.next(&t),1);
    }
    #[test] fn empty_mask_does_not_consume_randomness_and_selection_uses_set_bits() {
        let t=table();let mut rng=BattleRng {index:5};assert_eq!(rng.bit(&t,0),0);assert_eq!(rng.index,5);
        assert_eq!(rng.bit(&t,0x8000),0x8000);assert_eq!(rng.index,6);
        assert_eq!(BattleRng {index:254}.bit(&t,0x0109),0x0100);
    }
    #[test] fn gauge_initialization_preserves_low_bytes_and_consumes_ranks_for_skipped_slots() {
        let t=table();let mut rng=BattleRng::default();let mut actors=[GaugeActor {present:true,speed:30,gauge:0x1234};10];
        initialize_gauges(&t,&mut rng,&mut actors,GaugeContext {scrolling_state:255,..GaugeContext::default()}).unwrap();
        assert_eq!(rng.index,10);assert!(actors.iter().all(|a|a.gauge==0x1234));
        for a in &mut actors {a.gauge=0xff34;}
        initialize_gauges(&t,&mut rng,&mut actors,GaugeContext {scrolling_state:255,battle_flags:0x40,..GaugeContext::default()}).unwrap();
        assert!(actors[..4].iter().all(|a|a.gauge==0xff34));assert!(actors[4..].iter().all(|a|a.gauge==0x0234));
    }
}
