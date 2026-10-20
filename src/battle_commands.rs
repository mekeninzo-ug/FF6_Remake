//! Shared command pool from C2/4E54, C2/4ECB..4F07 and C2/0301.
//! Payload storage is separate from actor dispatch queues.
use crate::{battle_scheduler::{Queues,Scheduler},damage::AttackTable};
use std::{fs,path::Path};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Payload {pub command:u8,pub attack:u8,pub targets:u16,pub mp_cost:u8}
impl Default for Payload {fn default()->Self {Self {command:255,attack:255,targets:65535,mp_cost:255}}}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct CommandLists {
    pub links:[u8;128],pub payloads:[Payload;128],pub normal:[u8;10],pub retaliation:[u8;10],
}
impl Default for CommandLists {fn default()->Self {Self {links:[255;128],payloads:[Payload::default();128],normal:[255;10],retaliation:[255;10]}}}
impl CommandLists {
    /// Source full-pool behavior overwrites slot zero, rather than returning
    /// a capacity error. Each newly allocated slot initially links to itself.
    pub fn allocate(&mut self)->u8 {
        let id=self.links.iter().rposition(|v|v&0x80!=0).unwrap_or(0) as u8;
        self.links[usize::from(id)]=id;id
    }
    fn append(&mut self,head:u8,payload:Payload)->Result<(u8,u8),String> {
        let id=self.allocate();let new_head=if head==255 {id}else{head};let mut cursor=new_head;
        let mut found=false;
        for _ in 0..128 {
            let next=*self.links.get(usize::from(cursor)).ok_or("Command-list pointer outside modeled pool")?;
            if cursor==next || next&0x80!=0 {found=true;break;}
            cursor=next;
        }
        if !found {return Err("Command-list cycle does not have a terminal self-link".into());}
        self.links[usize::from(cursor)]=id;self.payloads[usize::from(id)]=payload;
        Ok((new_head,id))
    }
    pub fn normal(&mut self,slot:usize,payload:Payload)->Result<u8,String> {
        let head=*self.normal.get(slot).ok_or("Invalid command actor slot")?;
        let (head,id)=self.append(head,payload)?;self.normal[slot]=head;Ok(id)
    }
    pub fn retaliation(&mut self,slot:usize,payload:Payload,queues:&mut Queues)->Result<u8,String> {
        let head=*self.retaliation.get(slot).ok_or("Invalid retaliation actor slot")?;
        let (next,id)=self.append(head,payload)?;self.retaliation[slot]=next;
        if head==255 {queues.retaliation.push((slot*2) as u8);}
        Ok(id)
    }
    pub fn immediate(&mut self,payload:Payload,queues:&mut Queues)->Result<u8,String> {
        let (head,id)=self.append(queues.immediate.unwrap_or(255),payload)?;
        queues.immediate=Some(head);Ok(id)
    }
    pub fn remove_normal(&mut self,slot:usize)->Result<Option<Payload>,String> {
        let head=*self.normal.get(slot).ok_or("Invalid command actor slot")?;
        if head&0x80!=0 {return Ok(None);}
        let next=self.links[usize::from(head)];self.normal[slot]=if next==head {255}else{next};
        self.links[usize::from(head)]=255;
        // Source only frees the link. Payload bytes remain untouched.
        Ok(Some(self.payloads[usize::from(head)]))
    }
}

#[derive(Clone)]
pub struct MpCosts {pub master:[u8;256],pub character:[Vec<u8>;4],pub attacks:[u8;256]}
impl MpCosts {
    pub fn from_attacks(attacks:&AttackTable)->Self {
        Self {master:[255;256],character:std::array::from_fn(|_|Vec::new()),attacks:std::array::from_fn(|i|attacks.record(i as u8)[5])}
    }
    /// C2/4F08 consumes/clears B1 bit 40 even for nonmagical commands.
    /// Character spell lists contain already modified costs, not base ROM MP.
    pub fn get(&self,command:u8,attack:u8,actor_offset:u8,flags:&mut u8)->Result<u8,String> {
        let waived=*flags&0x40!=0;*flags&=!0x40;
        if waived || ![2,12,23,25].contains(&command) {return Ok(0);}
        if actor_offset>=8 {return Ok(self.attacks[usize::from(attack)]);}
        if actor_offset&1!=0 {return Err("Unaligned character spell-list pointer is not modeled".into());}
        let index=self.master[usize::from(attack)];if index==255 {return Ok(0);}
        self.character[usize::from(actor_offset/2)].get(usize::from(index)).copied().ok_or_else(||"Master spell index outside modified character list".into())
    }
}

#[derive(Clone,Debug)]
pub struct PlayerInputs {pub bytes:[u8;32],pub cursor:u8}
impl Default for PlayerInputs {fn default()->Self {Self {bytes:[255;32],cursor:0}}}
impl PlayerInputs {
    /// Bounded GetPlayerActions path for Fight, Magic, Magitek and X-Magic.
    /// Other commands need their source-specific FixPlayerAttack side effects.
    pub fn apply(&mut self,scheduler:&mut Scheduler,lists:&mut CommandLists,costs:&MpCosts,flags:&mut u8,status2:[u8;4])->Result<Vec<u8>,String> {
        if self.cursor>24 || self.cursor%8!=0 {return Err("Invalid player input cursor".into());}
        let mut created=Vec::new();
        for _ in 0..4 {
            let at=usize::from(self.cursor);let slot=usize::from(self.bytes[at]);
            if slot&0x80!=0 {return Ok(created);}
            if slot>=4 {return Err("Player input actor outside four character slots".into());}
            let command=self.bytes[at+1];
            if ![0,2,23,29].contains(&command) {return Err(format!("Player command {command:02X} needs unimplemented attack normalization"));}
            scheduler.enqueue_advance(slot);scheduler.actors[slot].flags0&=0x7b;
            self.bytes[at]=255;self.cursor=(self.cursor+8)&0x18;
            let first_command=if command==23 {2}else{command};
            let first_attack=if command==29 {self.bytes[at+2].checked_add(131).unwrap_or(238)}else{self.bytes[at+2]};
            let clear=scheduler.actors[slot].status1&2!=0 || status2[slot]&0x20!=0;
            let first_targets=if clear {0}else{u16::from_le_bytes([self.bytes[at+3],self.bytes[at+4]])};
            let cost=costs.get(first_command,first_attack,(slot*2) as u8,flags)?;
            created.push(lists.normal(slot,Payload {command:first_command,attack:first_attack,targets:first_targets,mp_cost:cost})?);
            if command==23 {
                let attack=self.bytes[at+5];let targets=if clear {0}else{u16::from_le_bytes([self.bytes[at+6],self.bytes[at+7]])};
                let cost=costs.get(23,attack,(slot*2) as u8,flags)?;
                created.push(lists.normal(slot,Payload {command:23,attack,targets,mp_cost:cost})?);
            }
        }
        Ok(created)
    }
}

pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let mut pool=CommandLists::default();let mut queues=Queues::default();
    let mut rows=String::from("step,kind,actor,command,attack,targets,cost,id,links,normal,retaliation,immediate,ret_queue\n");
    for step in 0..100usize {
        let slot=step%10;let payload=Payload {command:if step%4==0 {29}else{0},attack:step as u8,targets:(1u16<<(step%16))|3,mp_cost:(step%32) as u8};
        let kind=step%3;
        let id=match kind {0=>pool.normal(slot,payload)?,1=>pool.retaliation(slot,payload,&mut queues)?,_=>pool.immediate(payload,&mut queues)?};
        let hex=|bytes:&[u8]|bytes.iter().map(|v|format!("{v:02X}")).collect::<String>();
        writeln!(rows,"{step},{kind},{slot},{},{},{},{},{id},{},{},{},{},{}",payload.command,payload.attack,payload.targets,payload.mp_cost,hex(&pool.links),hex(&pool.normal),hex(&pool.retaliation),queues.immediate.unwrap_or(255),hex(&queues.retaliation.bytes[..usize::from(queues.retaliation.tail)])).unwrap();
    }
    fs::write(directory.join("creation.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("step,actor,removed,links,normal\n");
    for step in 0..64 {let slot=step%10;let removed=pool.remove_normal(slot)?;
        let hex=|bytes:&[u8]|bytes.iter().map(|v|format!("{v:02X}")).collect::<String>();
        let value=removed.map(|p|format!("{:02X}{:02X}{:04X}{:02X}",p.command,p.attack,p.targets,p.mp_cost)).unwrap_or_default();
        writeln!(rows,"{step},{slot},{value},{},{}",hex(&pool.links),hex(&pool.normal)).unwrap();
    }
    fs::write(directory.join("removal.csv"),rows).map_err(|e|e.to_string())?;
    let mut fresh=CommandLists::default();let mut rows=String::from("step,id\n");
    for step in 0..140 {writeln!(rows,"{step},{}",fresh.allocate()).unwrap();}
    fs::write(directory.join("allocation.csv"),rows).map_err(|e|e.to_string())?;
    let attacks=AttackTable::load(root)?;let mut costs=MpCosts::from_attacks(&attacks);
    for (i,m) in costs.master.iter_mut().enumerate() {*m=if i%7==0 {255}else{(i%64) as u8};}
    costs.character=std::array::from_fn(|slot|(0..64).map(|i|((i*7+slot*11)%256) as u8).collect());
    let mut rows=String::from("command,attack,actor,flags,cost,after_flags\n");
    for command in [0,1,2,12,23,25,29,255] {for attack in 0..=255 {for actor in [0,2,6,8,18] {for flags in [0,0x41] {
        let mut after=flags;let cost=costs.get(command,attack,actor,&mut after)?;
        writeln!(rows,"{command},{attack},{actor},{flags},{cost},{after}").unwrap();
    }}}}
    fs::write(directory.join("mp-costs.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,cursor,flags,status1,status2,before,after,after_cursor,after_flags,links,normal,advance,flags0,flags1,payloads\n");
    for case in 0..256usize {
        let mut input=PlayerInputs::default();input.cursor=((case%4)*8) as u8;
        for i in 0..case%5 {
            let at=(usize::from(input.cursor)+i*8)%32;let command=[0,2,29,23][(case+i)%4];
            input.bytes[at..at+8].copy_from_slice(&[((case+i)%4) as u8,command,if command==29 {if case%11==0 {255}else{(case%9) as u8}}else{(case%54) as u8},1,1,(case%37) as u8,2,2]);
        }
        let hex=|b:&[u8]|b.iter().map(|v|format!("{v:02X}")).collect::<String>();
        let before=hex(&input.bytes);let cursor=input.cursor;let mut flags=if case%7==0 {0x40}else{0};let before_flags=flags;
        let mut scheduler=Scheduler::default();let status2=std::array::from_fn(|slot|if (case+slot)%5==0 {0x20}else{0});
        for (slot,a) in scheduler.actors[..4].iter_mut().enumerate() {a.flags0=0x8b;a.flags1=255;a.status1=if (case+slot)%3==0 {2}else{0};}
        let status1=scheduler.actors[..4].iter().map(|a|a.status1).collect::<Vec<_>>();
        let mut lists=CommandLists::default();input.apply(&mut scheduler,&mut lists,&costs,&mut flags,status2)?;
        let flags0=scheduler.actors[..4].iter().map(|a|a.flags0).collect::<Vec<_>>();let flags1=scheduler.actors[..4].iter().map(|a|a.flags1).collect::<Vec<_>>();
        let mut payloads=Vec::new();for p in lists.payloads {payloads.extend_from_slice(&[p.command,p.attack]);payloads.extend_from_slice(&p.targets.to_le_bytes());payloads.push(p.mp_cost);}
        writeln!(rows,"{case},{cursor},{before_flags},{},{},{before},{},{},{flags},{},{},{},{},{},{}",hex(&status1),hex(&status2),hex(&input.bytes),input.cursor,hex(&lists.links),hex(&lists.normal),hex(&scheduler.queues.advance.bytes[..usize::from(scheduler.queues.advance.tail)]),hex(&flags0),hex(&flags1),hex(&payloads)).unwrap();
    }
    fs::write(directory.join("player-inputs.csv"),rows).map_err(|e|e.to_string())
}

#[cfg(test)]mod tests {
    use super::*;
    fn payload(attack:u8)->Payload {Payload {command:29,attack,targets:0x100,mp_cost:0}}
    #[test]fn lists_append_at_self_link_and_reuse_highest_free_slot() {
        let mut p=CommandLists::default();assert_eq!(p.normal(0,payload(131)),Ok(127));assert_eq!(p.normal(0,payload(132)),Ok(126));
        assert_eq!(p.normal[0],127);assert_eq!((p.links[127],p.links[126]),(126,126));
        assert_eq!(p.remove_normal(0).unwrap(),Some(payload(131)));assert_eq!(p.normal[0],126);
        assert_eq!(p.normal(1,payload(133)),Ok(127));assert_eq!(p.remove_normal(0).unwrap(),Some(payload(132)));assert_eq!(p.normal[0],255);
    }
    #[test]fn retaliation_enqueues_once_and_immediate_preserves_first_head() {
        let mut p=CommandLists::default();let mut q=Queues::default();
        p.retaliation(6,payload(238),&mut q).unwrap();p.retaliation(6,payload(239),&mut q).unwrap();
        assert_eq!(q.retaliation.tail,1);assert_eq!(q.retaliation.pop(),Some(12));
        p.immediate(payload(1),&mut q).unwrap();let head=q.immediate;
        p.immediate(payload(2),&mut q).unwrap();assert_eq!(q.immediate,head);
    }
    #[test]fn full_pool_allocator_reuses_zero_and_cycles_fail_explicitly() {
        let mut p=CommandLists::default();for expected in (0..128).rev() {assert_eq!(p.allocate(),expected);}
        assert_eq!(p.allocate(),0);assert_eq!(p.links[0],0);
        let mut p=CommandLists::default();p.normal[0]=3;p.links[3]=4;p.links[4]=3;
        assert!(p.normal(0,payload(1)).is_err());
    }
    #[test]fn magitek_input_normalizes_attack_and_clears_menu_flags() {
        let mut costs=MpCosts {master:[255;256],character:std::array::from_fn(|_|Vec::new()),attacks:[0;256]};
        costs.master[131]=0;costs.character[0]=vec![99];
        let mut state=Scheduler::default();state.actors[0].flags0=0x8b;state.actors[0].flags1=255;
        let mut lists=CommandLists::default();let mut input=PlayerInputs::default();input.bytes[..8].copy_from_slice(&[0,29,0,0,1,0,0,0]);
        let ids=input.apply(&mut state,&mut lists,&costs,&mut 0,[0;4]).unwrap();
        assert_eq!(lists.payloads[usize::from(ids[0])],Payload {command:29,attack:131,targets:256,mp_cost:0});
        assert_eq!((state.actors[0].flags0,state.actors[0].flags1),(0x0b,254));
        assert_eq!(state.queues.advance.pop(),Some(0));assert_eq!(input.cursor,8);
    }
    #[test]fn x_magic_consumes_waiver_once_and_status_changes_clear_both_targets() {
        let mut costs=MpCosts {master:[255;256],character:std::array::from_fn(|_|Vec::new()),attacks:[0;256]};
        costs.master[3]=0;costs.master[4]=1;costs.character[0]=vec![7,9];
        let mut state=Scheduler::default();let mut lists=CommandLists::default();let mut input=PlayerInputs::default();
        input.bytes[..8].copy_from_slice(&[0,23,3,0,1,4,0,2]);let mut flags=0x40;
        let ids=input.apply(&mut state,&mut lists,&costs,&mut flags,[0x20,0,0,0]).unwrap();
        assert_eq!(ids,vec![127,126]);assert_eq!(flags,0);
        assert_eq!(lists.payloads[127],Payload {command:2,attack:3,targets:0,mp_cost:0});
        assert_eq!(lists.payloads[126],Payload {command:23,attack:4,targets:0,mp_cost:9});
        assert_eq!(state.queues.advance.tail,1);
        let mut input=PlayerInputs::default();input.bytes[..3].copy_from_slice(&[0,1,0]);let original=input.bytes;
        assert!(input.apply(&mut state,&mut lists,&costs,&mut flags,[0;4]).is_err());assert_eq!(input.bytes,original);
    }
}
