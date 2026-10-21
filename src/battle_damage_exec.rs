//! ExecDmg C2/62EF..6397, ApplyDmg C2/12F5..131E and numeral queue helpers.
use crate::{damage_application::{self,Target,Damage},battle_targets::actor_mask};
use std::{fs,path::Path,collections::BTreeMap};
#[derive(Clone,Copy,Debug)]pub struct Actor {pub target:Target,pub taken:u16,pub healed:u16,pub previous_attacker:u8,pub retaliation_target:u8}
#[derive(Clone,Copy,Debug)]pub struct Context {pub attacker:usize,pub attack_flags:u16,pub invincible:u16,pub golem:u8,pub dog:u8,pub golem_hp:u16,pub weapon_spell:u8,pub blackbelt_blocked:u8,pub misses:u16}
#[derive(Clone,Debug)]pub struct Graphics {
    pub command:u16,pub parameter:u16,pub tail:u8,pub script:[u8;259],pub frame_count:u8,pub frames:Vec<[u16;10]>,
}
impl Default for Graphics {fn default()->Self {Self {command:0,parameter:0,tail:0,script:[0;259],frame_count:0,frames:vec![[65535;10];256]}}}
impl Graphics {
    pub(crate) fn append(&mut self) {let offset=usize::from(self.tail);let bytes=[self.command.to_le_bytes(),self.parameter.to_le_bytes()].concat();self.script[offset..offset+4].copy_from_slice(&bytes);self.tail=self.tail.wrapping_add(4);}
    fn numerals(&mut self,actors:&[Actor;10],count:usize) {
        if count<5 {for slot in (0..10).rev() {if actors[slot].taken!=65535 {self.parameter=actors[slot].taken;self.command=((slot as u16)<<8)|11;self.append();}}}
        else {self.command=(self.command&0xff00)|3;self.append();let id=self.frame_count;self.frame_count=self.frame_count.wrapping_add(1);self.frames[usize::from(id)]=std::array::from_fn(|slot|actors[slot].taken);}
    }
}
pub fn run(actors:&mut[Actor;10],c:&mut Context,g:&mut Graphics)->Result<(),String> {
    if c.attacker>=10 {return Err("Damage execution attacker outside ten actors".into());}
    let mut taken_count=0;let mut healed_count=0;
    for slot in (0..10).rev() {
        let a=&mut actors[slot];let mask=actor_mask(slot);
        if a.taken!=65535 || a.healed!=65535 {
            c.misses&=!mask;a.target.weapon_spell=c.weapon_spell;
            let result=damage_application::apply(&mut a.target,Damage {taken:a.taken,healed:a.healed,invincible:c.invincible&mask!=0,golem:c.golem,dog:c.dog,affects_mp:c.attack_flags&0x8000!=0});
            a.taken=result.taken;c.weapon_spell=a.target.weapon_spell;
            if result.carry {
                if a.previous_attacker&128==0 {a.previous_attacker=128|c.attacker as u8;}
                if slot!=c.attacker {a.retaliation_target=(c.attacker*2) as u8;c.blackbelt_blocked&=!(mask as u8);}
            }
            if c.golem==(slot*2) as u8 && a.taken!=65535 {c.golem_hp=c.golem_hp.saturating_sub(a.taken);}
            if a.healed!=65535 {
                a.healed|=0x8000;healed_count+=1;
                if a.taken==65535 {healed_count-=1;a.taken=a.healed;a.healed=65535;}
            }
        }
        if c.misses&mask!=0 {a.taken=0x4000;}
        if a.taken!=65535 {taken_count+=1;}
    }
    g.numerals(actors,taken_count);
    if healed_count!=0 {let mut count=0;for a in actors.iter_mut() {a.taken=a.healed;if a.taken!=65535 {count+=1;}}g.numerals(actors,count);}
    for a in actors {a.taken=65535;a.healed=65535;}
    Ok(())
}
fn snapshot(a:&[Actor;10],c:&Context,g:&Graphics)->String {
    let mut m=BTreeMap::<u32,u8>::new();let mut put=|addr:u32,val:u16,width:usize|{for i in 0..width {m.insert(addr+i as u32,(val>>(i*8)) as u8);}};
    for (slot,a) in a.iter().enumerate() {let x=slot as u32*2;let t=a.target;
        for (addr,val,width) in [(0x3bf4,t.hp,2),(0x3c08,t.mp,2),(0x3c1c,t.max_hp,2),(0x3c30,t.max_mp,2),(0x3ee4,t.status1,2),(0x3dd4,t.pending_status,2),(0x3204,t.update_flags,2),(0x3c95,t.monster_flags,2),(0x33d0,a.taken,2),(0x33e4,a.healed,2),(0x32e0,u16::from(a.previous_attacker),1),(0x327c,u16::from(a.retaliation_target),1),(0x3018,actor_mask(slot),2)] {put(0x7e0000+addr+x,val,width);}
    }
    for (addr,val,width) in [(0x11a2,c.attack_flags,2),(0x3a3c,c.invincible,2),(0x3a82,u16::from(c.golem),1),(0x3a83,u16::from(c.dog),1),(0x3a36,c.golem_hp,2),(0x3a89,u16::from(c.weapon_spell),1),(0x3419,u16::from(c.blackbelt_blocked),1),(0x3a5a,c.misses,2),(0x3a28,g.command,2),(0x3a2a,g.parameter,2),(0x3a72,u16::from(g.tail),1),(0x3a34,u16::from(g.frame_count),1)] {put(0x7e0000+addr,val,width);}
    for (i,v) in g.script.iter().enumerate() {put(0x7e2d6e+i as u32,u16::from(*v),1);}
    // Fixtures use non-aliasing source frame IDs 0..7. The native renderer
    // carries indexed payloads; it does not emulate invalid WRAM aliasing.
    for (frame,values) in g.frames.iter().take(8).enumerate() {for (slot,v) in values.iter().enumerate() {put(0x7e2bce+frame as u32*20+slot as u32*2,*v,2);}}
    m.iter().map(|(addr,v)|format!("{addr:06X}{v:02X}")).collect()
}
pub fn capture(dir:&Path)->Result<(),String> {
    use std::fmt::Write;fs::create_dir_all(dir).map_err(|e|e.to_string())?;let mut seed=0x6ff6u32;let mut rows=String::from("case,attacker,before,after\n");
    for case in 0..4096usize {let mut next=||{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);(seed>>16) as u8};
        let mut c=Context {attacker:case%10,attack_flags:u16::from_le_bytes([next(),next()]),invincible:u16::from_le_bytes([next(),next()]),golem:if case%7==0 {((case+3)%10*2) as u8}else{255},dog:if case%17==0 {((case+5)%10*2) as u8}else{255},golem_hp:u16::from_le_bytes([next(),next()]),weapon_spell:next(),blackbelt_blocked:next(),misses:u16::from_le_bytes([next(),next()])};
        let mut a=std::array::from_fn(|slot| {
            let choices=[65535,0,1,25,40,9999,32767,32768,65534];let mut taken=choices[usize::from(next())%choices.len()];let mut healed=choices[usize::from(next())%choices.len()];if case%5==0 || (case%7==0 && slot!=case%10) {taken=65535;healed=65535;}
            Actor {target:Target {hp:u16::from_le_bytes([next(),next()]),mp:u16::from_le_bytes([next(),next()]),max_hp:u16::from_le_bytes([next(),next()]),max_mp:u16::from_le_bytes([next(),next()]),status1:u16::from_le_bytes([next(),next()]),pending_status:u16::from_le_bytes([next(),next()]),update_flags:u16::from_le_bytes([next(),next()]),monster_flags:u16::from_le_bytes([next(),next()]),weapon_spell:c.weapon_spell},taken,healed,previous_attacker:if case%3==0 {128|(slot as u8)}else{127},retaliation_target:next()}
        });
        let mut g=Graphics::default();g.command=u16::from_le_bytes([next(),next()]);g.parameter=u16::from_le_bytes([next(),next()]);g.tail=next();g.frame_count=(case%7) as u8;g.script=std::array::from_fn(|_|next());for frame in g.frames.iter_mut().take(8) {for v in frame {*v=u16::from_le_bytes([next(),next()]);}}
        let before=snapshot(&a,&c,&g);run(&mut a,&mut c,&mut g)?;writeln!(rows,"{case},{},{before},{}",c.attacker,snapshot(&a,&c,&g)).unwrap();
    }
    fs::write(dir.join("execution.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn actor()->Actor {Actor {target:Target {hp:100,mp:30,max_hp:100,max_mp:30,status1:0,pending_status:0,update_flags:0,monster_flags:0,weapon_spell:0},taken:65535,healed:65535,previous_attacker:127,retaliation_target:255}}
    fn context()->Context {Context {attacker:0,attack_flags:0,invincible:0,golem:255,dog:255,golem_hp:0,weapon_spell:9,blackbelt_blocked:255,misses:0}}
    #[test]fn nonlethal_damage_records_attacker_and_clears_payload_after_numerals() {
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();a[1].taken=10;run(&mut a,&mut c,&mut g).unwrap();assert_eq!((a[1].target.hp,a[1].previous_attacker,a[1].retaliation_target,c.blackbelt_blocked),(90,128,0,253));assert_eq!(&g.script[..4],&[11,1,10,0]);assert_eq!((a[1].taken,a[1].healed),(65535,65535));
    }
    #[test]fn pure_healing_uses_first_display_pass_and_mp_flag_is_word_high_bit() {
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();a[1].target.mp=5;a[1].healed=7;c.attack_flags=0x8000;run(&mut a,&mut c,&mut g).unwrap();assert_eq!((a[1].target.hp,a[1].target.mp,a[1].target.update_flags),(100,12,128));assert_eq!(&g.script[..4],&[11,1,7,128]);assert_eq!(g.tail,4);
    }
    #[test]fn five_targets_use_multi_frame_and_drain_can_add_second_frame() {
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();g.command=0xabcd;for x in a.iter_mut().take(5) {x.taken=20;x.healed=10;}run(&mut a,&mut c,&mut g).unwrap();assert_eq!((g.tail,g.frame_count,g.command),(8,2,0xab03));assert_eq!(&g.frames[0][..5],&[20;5]);assert_eq!(&g.frames[1][..5],&[0x800a;5]);
    }
    #[test]fn misses_show_code_without_hp_change_and_golem_consumes_blocked_damage() {
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();c.misses=1;c.golem=12;c.golem_hp=15;a[6].taken=20;run(&mut a,&mut c,&mut g).unwrap();assert_eq!((a[0].target.hp,a[6].target.hp,c.golem_hp,c.misses),(100,100,0,1));assert_eq!(&g.script[4..8],&[11,0,0,64]);
    }
    #[test]fn lethal_damage_remains_pending_until_the_status_phase_commits_it() {
        use crate::battle_status_update as status;
        let mut a=[actor();10];let mut c=context();let mut g=Graphics::default();a[6].target.hp=40;a[6].taken=50;run(&mut a,&mut c,&mut g).unwrap();assert_eq!((a[6].target.hp,a[6].target.status1,a[6].target.pending_status,c.weapon_spell),(0,0,128,0));
        let mut actors=[status::Actor::default();10];let t=&mut actors[6];t.flags=1;t.status.hp=a[6].target.hp;t.status.max_hp=a[6].target.max_hp;t.status.current12=a[6].target.status1;t.status.pending.set12=a[6].target.pending_status;t.status.allowed12=65535;t.status.allowed34=65535;t.status.retaliation_marker=u16::from(a[6].previous_attacker)<<8;
        let mut globals=status::Globals::default();status::update(&mut actors,&mut globals);assert_ne!(actors[6].status.current12&128,0);assert_ne!(actors[6].status.retaliation12&128,0);assert_ne!(globals.newly_dead&1024,0);assert_eq!(actors[6].status.pending.set12,0);
    }
}
