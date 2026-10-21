//! InitSpellList C2/5551..568C, from freshly initialized battle buffers.
use crate::{damage::AttackTable,battle_commands::MpCosts};
use std::{fs,path::Path,collections::BTreeMap};
const ORDER:[[i8;6];3]=[[9,30,0,0,30,21],[9,-15,0,9,-24,-24],[-45,-45,0,-21,-24,0]];
#[derive(Clone,Copy,Debug)]pub struct Context {pub actors:[u8;4],pub learning:[[u8;54];12],pub lores:[u8;3],pub order:u8,pub game_time:[u8;2]}
#[derive(Clone,Debug,PartialEq,Eq)]pub struct Lists {pub master:[u8;78],pub pointers:[u8;256],pub records:[[[u8;4];79];4],pub lore_count:u8,pub sorted:[u8;54]}
impl Default for Lists {fn default()->Self {Self {master:[255;78],pointers:[255;256],records:[[[255;4];79];4],lore_count:0,sorted:[255;54]}}}
pub fn build(c:Context,attacks:&AttackTable)->Result<Lists,String> {
    let order=usize::from(c.order&7);if order>=6 {return Err("Invalid spell order 6/7 reads outside source order table".into());}
    let mut s=Lists::default();s.pointers[54..81].fill(0);
    for lore in (0..24).rev() {if c.lores[lore/8]&(1<<(lore%8))!=0 {s.lore_count+=1;s.pointers[139+lore]=(55+lore) as u8;s.master[54+lore]=(139+lore) as u8;}}
    for actor in c.actors.into_iter().rev() {if actor<12 {for spell in (0..54).rev() {if c.learning[usize::from(actor)][spell]==255 {s.master[spell]=spell as u8;}}}}
    for spell in (0..54).rev() {let id=s.master[spell];if id==255 {continue;}let kind=if id<24 {0}else if id<45 {1}else{2};let dest=id.wrapping_add(ORDER[kind][order] as u8);s.sorted[usize::from(dest)]=spell as u8;}
    s.master[..54].fill(255);let mut dest=0;
    for start in (0..54).step_by(3) {if s.sorted[start..start+3].iter().any(|v|*v!=255) {s.master[dest..dest+3].copy_from_slice(&s.sorted[start..start+3]);dest+=3;}}
    for i in (0..54).rev() {let id=s.master[i];if id!=255 {s.pointers[usize::from(id)]=(i+1) as u8;}}
    for i in (0..78).rev() {let id=s.master[i];if id==255 {continue;}let record=usize::from(s.pointers[usize::from(id)]);let prop=attacks.record(id);let cost=if i==68 {c.game_time[0].wrapping_shl(1)|u8::from(c.game_time[1]>=30)}else{prop[5]};
        for menu in &mut s.records {menu[record][0]=if id>=139 {id-139}else{id};menu[record][2]=prop[0];menu[record][3]=cost;}
    }
    Ok(s)
}
impl Lists {
    /// Validated per-character menus replace these records before MP lookup.
    pub fn mp_costs(&self,attacks:&AttackTable)->MpCosts {let mut costs=MpCosts::from_attacks(attacks);costs.master=self.pointers;costs.character=std::array::from_fn(|slot|self.records[slot].iter().map(|r|r[3]).collect());costs}
}
fn snapshot(c:Context,s:&Lists)->String {
    let mut m=BTreeMap::<u32,u8>::new();
    for (i,v) in s.master.iter().enumerate() {m.insert(0x7e3034+i as u32,*v);}for (i,v) in s.pointers.iter().enumerate() {m.insert(0x7e3084+i as u32,*v);}for (i,v) in s.sorted.iter().enumerate() {m.insert(0x7e11a0+i as u32,*v);}
    for slot in 0..4 {m.insert(0x7e3ed8+slot as u32*2,c.actors[slot]);for (i,v) in s.records[slot].concat().iter().enumerate() {m.insert(0x7e208e+slot as u32*316+i as u32,*v);}}
    for actor in 0..12 {for (i,v) in c.learning[actor].iter().enumerate() {m.insert(0x7e1a6e+actor as u32*54+i as u32,*v);}}
    for (i,v) in c.lores.iter().enumerate() {m.insert(0x7e1d29+i as u32,*v);}m.insert(0x7e1d54,c.order);m.insert(0x7e1863,c.game_time[0]);m.insert(0x7e1864,c.game_time[1]);m.insert(0x7e3a87,s.lore_count);
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(root:&Path,dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let attacks=AttackTable::load(root)?;let mut seed=0x6ff6u32;let mut rows=String::from("case,before,after\n");
    for case in 0..384usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let c=Context {actors:std::array::from_fn(|i|if case%7==0 {255}else{((case+i*3)%17) as u8}),learning:std::array::from_fn(|_|std::array::from_fn(|_|if case%11==0 {0}else if case%13==0 || next()%3==0 {255}else{next()%255})),lores:if case%17==0 {[0;3]}else if case%19==0 {[255;3]}else{std::array::from_fn(|_|next())},order:(case%6) as u8,game_time:[next(),next()]};
        let before=snapshot(c,&Lists::default());let s=build(c,&attacks)?;writeln!(rows,"{case},{before},{}",snapshot(c,&s)).unwrap();
    }fs::write(dir.join("spell-lists.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn context()->Context {Context {actors:[0,255,255,255],learning:[[0;54];12],lores:[0;3],order:2,game_time:[0;2]}}
    fn attacks()->AttackTable {AttackTable::load(Path::new("extracted/twue-3.07")).unwrap()}
    #[test]fn union_keeps_three_spell_rows_and_partial_holes() {let mut c=context();c.learning[0][1]=255;c.learning[0][6]=255;let s=build(c,&attacks()).unwrap();assert_eq!(&s.master[..9],&[255,1,255,6,255,255,255,255,255]);assert_eq!((s.pointers[1],s.pointers[6]),(2,4));assert_eq!(s.records[0][2][0],1);assert_eq!(s.records[0][1][0],255);}
    #[test]fn all_six_orders_preserve_unique_spell_coverage() {let mut c=context();c.learning[0]=[255;54];for order in 0..6 {c.order=order;let s=build(c,&attacks()).unwrap();let mut ids=s.master[..54].to_vec();ids.sort();assert_eq!(ids,(0..54).collect::<Vec<u8>>());assert!(s.pointers[..54].iter().all(|v|(1..=54).contains(v)));}}
    #[test]fn gogo_and_guests_do_not_add_owned_magic_to_union() {let mut c=context();c.actors=[12,13,14,255];c.learning=[[255;54];12];let s=build(c,&attacks()).unwrap();assert!(s.master[..54].iter().all(|v|*v==255));assert!(s.pointers[54..81].iter().all(|v|*v==0));}
    #[test]fn lore_slots_are_fixed_and_step_mine_uses_original_byte_rotate() {let mut c=context();c.lores=[1,64,128];c.game_time=[200,30];let s=build(c,&attacks()).unwrap();assert_eq!(s.lore_count,3);assert_eq!((s.master[54],s.master[68],s.master[77]),(139,153,162));assert_eq!((s.records[0][55][0],s.records[0][69][0],s.records[0][78][0]),(0,14,23));assert_eq!(s.records[0][69][3],145);}
    #[test]fn master_lists_validate_then_drive_command_eligibility_and_cached_costs() {
        use crate::{battle_spell_validation as v,battle_command_init as cmd,battle_action_init::CommandProperties};let a=attacks();let mut c=context();c.learning[0][0]=255;let mut lists=build(c,&a).unwrap();let mut s=v::State {records:lists.records[0],known:0,cached_esper:255,cached_target:255};let mut menu=cmd::State {records:[[0;3];4],mp:27,max_mp:27,pending_status4:0,morph_counter:0};
        v::validate_commands(v::Context {slot:0,actor:0,esper:255,relic3:32,learning:c.learning[0]},&mut s,cmd::Context {slot:0,actor:0,commands:[0,3,2,1],status1:8,global11:0,global12:0,relic_commands:0,magic_count:0,esper:255,dances:0,battle_flags:0},&mut menu,&a,&CommandProperties::load(Path::new("extracted/twue-3.07")).unwrap()).unwrap();assert_eq!((s.known,menu.records[2][0],menu.mp),(1,2,27));lists.records[0]=s.records;let costs=lists.mp_costs(&a);assert_eq!(costs.get(2,0,0,&mut 0).unwrap(),2);
    }
}
