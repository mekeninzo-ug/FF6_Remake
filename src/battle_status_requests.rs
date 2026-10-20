//! MagicStatusEffect C2/4406..447E and status request helpers. This queues
//! changes; UpdateStatus's per-status handlers and commit are separate.
use std::{fs,path::Path};
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct Changes {pub set12:u16,pub set34:u16,pub clear12:u16,pub clear34:u16}
impl Changes {pub fn resolved(self,current12:u16,current34:u16)->(u16,u16) {((current12|self.set12)&!self.clear12,(current34|self.set34)&!self.clear34)}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Target {
    pub current12:u16,pub current34:u16,pub pending:Changes,
    pub allowed12:u16,pub allowed34:u16,pub monster_flags:u8,pub mask:u16,
    pub failed_mask:u16,pub miss_targets:u16,
}
pub fn queue(magic:&[u8;14],flags_b3:u8,t:&mut Target)->Result<Changes,String> {
    let request12=u16::from_le_bytes([magic[10],magic[11]]);let request34=u16::from_le_bytes([magic[12],magic[13]]);
    let mut delta=Changes::default();
    match magic[4]&0x0c {
        0=>{delta.set12=request12&!t.current12;delta.set34=request34&!t.current34;},
        4=>{delta.clear12=request12&t.current12;delta.clear34=request34&t.current34;},
        8=>{delta.set12=request12&!t.current12;delta.set34=request34&!t.current34;delta.clear12=request12&t.current12;delta.clear34=request34&t.current34;},
        _=>return Err("Status mode 0C has no source StatusModTbl entry".into()),
    }
    if magic[2]&1==0 && t.current12&0x10!=0 && request12&0x10==0 {delta.clear12|=0x10;}
    if magic[1]&1!=0 && t.current34&0x0200!=0 && request34&0x0200==0 {delta.clear34|=0x0200;}
    if flags_b3&0x80==0 {delta.clear12&=!0x10;}
    if t.monster_flags&0x80!=0 && magic[2]&8!=0 && magic[4]&4!=0 && magic[10]&0x82!=0 {delta.set12|=0x80;}
    t.pending.set12|=delta.set12;t.pending.set34|=delta.set34;t.pending.clear12|=delta.clear12;t.pending.clear34|=delta.clear34;
    if magic[7]&1!=0 && (delta.set12|delta.clear12)&t.allowed12==0 && (delta.set34|delta.clear34)&t.allowed34==0 {t.failed_mask=t.mask;t.miss_targets|=t.mask;}
    Ok(delta)
}
pub fn capture(directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let mut seed=0x6ff6u32;
    let mut rows=String::from("case,magic,b3,current12,current34,set12,set34,clear12,clear34,allowed12,allowed34,monster,mask,failed,miss,set12_out,set34_out,clear12_out,clear34_out,failed_out,miss_out,delta_set12,delta_set34,delta_clear12,delta_clear34,resolved12,resolved34\n");
    for case in 0..16384usize {
        let mut next=|| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u16};
        let mut magic=std::array::from_fn(|_|next() as u8);magic[4]=(magic[4]&!0xc)|((case%3) as u8*4);
        let b3=next() as u8;let slot=case%10;
        let mut t=Target {current12:next(),current34:next(),pending:Changes {set12:next(),set34:next(),clear12:next(),clear34:next()},allowed12:if case%5==0 {0}else{next()},allowed34:if case%7==0 {0}else{next()},monster_flags:next() as u8,mask:1<<if slot<4 {slot}else{slot+4},failed_mask:next(),miss_targets:next()};let before=t;let delta=queue(&magic,b3,&mut t)?;let resolved=delta.resolved(t.current12,t.current34);
        writeln!(rows,"{case},{},{b3},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",magic.iter().map(|v|format!("{v:02X}")).collect::<String>(),before.current12,before.current34,before.pending.set12,before.pending.set34,before.pending.clear12,before.pending.clear34,before.allowed12,before.allowed34,before.monster_flags,before.mask,before.failed_mask,before.miss_targets,t.pending.set12,t.pending.set34,t.pending.clear12,t.pending.clear34,t.failed_mask,t.miss_targets,delta.set12,delta.set34,delta.clear12,delta.clear34,resolved.0,resolved.1).unwrap();
    }
    fs::write(directory.join("requests.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn target()->Target {Target {current12:0,current34:0,pending:Changes::default(),allowed12:65535,allowed34:65535,monster_flags:0,mask:1,failed_mask:0,miss_targets:0}}
    #[test]fn toggle_requests_keep_current_status_separate() {
        let mut t=target();t.current12=2;let mut m=[0;14];m[4]=8;m[10]=3;
        let delta=queue(&m,128,&mut t).unwrap();assert_eq!((delta.set12,delta.clear12,t.current12),(1,2,2));assert_eq!(delta.resolved(t.current12,t.current34),(1,0));
    }
    #[test]fn magic_dispels_vanish_and_fire_thaws_unless_request_preserves_them() {
        let mut t=target();t.current12=16;t.current34=512;let mut m=[0;14];m[1]=1;
        let delta=queue(&m,128,&mut t).unwrap();assert_eq!((delta.clear12,delta.clear34),(16,512));
        let mut t=target();t.current12=16;assert_eq!(queue(&m,0,&mut t).unwrap().clear12,0);
    }
    #[test]fn immunity_failure_marks_target_without_committing_status() {
        let mut t=target();t.allowed12=0;t.allowed34=0;let mut m=[0;14];m[7]=1;m[10]=128;
        queue(&m,128,&mut t).unwrap();assert_eq!((t.pending.set12,t.current12,t.failed_mask,t.miss_targets),(128,0,1,1));
    }
    #[test]fn target_monster_poison_special_queues_status_without_hp_damage() {
        use crate::{battle_data::{BattleData,MonsterProperties},battle_rage::Traits,battle_attack_prep as prep,battle_target_init,elemental_damage};
        let root=Path::new("extracted/twue-3.07");let records=fs::read(root.join("build/native_battle/monster_properties.bin")).unwrap();let monster=MonsterProperties {raw:records[41*32..42*32].try_into().unwrap()};assert_eq!(monster.raw[31],0x42);
        let mut traits=Traits {special:0,weak_half:0,allowed:[255;4],pending12:0,pending34:0,relic45:0,equip_status23:0,absorb_null:0,metamorph_flags:0};traits.apply(&monster);
        let mut working=battle_target_init::State {restrictions:0,cursor:0,flags_b1:0,last_spell:0,last_item:0,magic:[0;14]};battle_target_init::Initializer::load(root).unwrap().initialize(0,239,&mut working);working.magic[6]=monster.raw[1];
        let mut state=prep::State {magic:working.magic,flags_b2:64,flags_b3:128,graphics_flags:0,animation:0,multiplier:0,base:100};
        prep::prepare(prep::Context {special_index:6,special_property:traits.special,special_animation:BattleData::load(root).unwrap().special_animation(41).unwrap(),status2:0,status4:0,targets:1,row_flags:0},&mut state,&mut Vec::new());
        assert_eq!((state.magic[6],state.magic[7]&1,state.magic[10]),(0,1,4));
        for allowed in [65535,65531] {
            let mut t=target();t.allowed12=allowed;queue(&state.magic,state.flags_b3,&mut t).unwrap();
            assert_eq!((t.pending.set12,t.current12),(4,0));assert_eq!(t.failed_mask,if allowed&4==0 {1}else{0});
            assert_eq!(elemental_damage::resolve(state.base,elemental_damage::Context {power:state.magic[6],..elemental_damage::Context::default()}).unwrap(),None);
        }
    }
    #[test]fn target_fire_beam_requests_thaw_before_status_commit() {
        let root=Path::new("extracted/twue-3.07");let initializer=crate::battle_target_init::Initializer::load(root).unwrap();
        let mut working=crate::battle_target_init::State {restrictions:0,cursor:0,flags_b1:0,last_spell:0,last_item:0,magic:[0;14]};initializer.initialize(29,131,&mut working);
        let mut t=target();t.current34=512;let delta=queue(&working.magic,128,&mut t).unwrap();assert_eq!((delta.clear34,t.pending.clear34,t.current34),(512,512,512));assert_eq!(delta.resolved(t.current12,t.current34),(0,0));
    }
}
