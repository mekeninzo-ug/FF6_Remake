//! Advance-wait dispatch C2/2188 and QueueAction C2/031C.
//! AI and target/graphics subsystems remain explicit source-position hooks.
use crate::{battle_commands::{CommandLists,MpCosts,Payload},battle_scheduler::{CommandDelays,Scheduler}};
use std::{fs,path::Path};
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum AdvanceEvent {Ai(u8),InitTarget(u8,u8,u8),ChooseTarget(u8),Graphics(u8,u8)}
impl AdvanceEvent {pub fn label(&self)->String {match self {Self::Ai(s)=>format!("A{s}"),Self::InitTarget(s,c,a)=>format!("I{s}:{c}:{a}"),Self::ChooseTarget(s)=>format!("T{s}"),Self::Graphics(s,c)=>format!("G{s}:{c}")}}}
#[derive(Clone,Debug)]
pub struct AdvanceState {
    pub scheduler:Scheduler,pub commands:CommandLists,pub retaliation_flags:[u8;10],
    pub ai_high:[u8;4],pub changed_targets:u8,pub command:u8,pub attack:u8,pub targets:u16,pub mp_flags:u8,
}
impl Default for AdvanceState {
    fn default()->Self {Self {scheduler:Scheduler::default(),commands:CommandLists::default(),retaliation_flags:[0;10],ai_high:[255;4],changed_targets:0,command:0,attack:0,targets:0,mp_flags:0}}
}
impl AdvanceState {
    pub fn prepare(&mut self,slot:usize,delays:&CommandDelays,costs:&MpCosts,mut hook:impl FnMut(&mut Self,AdvanceEvent)->Result<(),String>)->Result<bool,String> {
        if slot>=10 {return Err("Advance actor outside ten battle slots".into());}
        self.scheduler.actors[slot].flags1|=0x80;
        if self.scheduler.actors[slot].flags0&0x50!=0 {return Ok(false);}
        self.scheduler.actors[slot].flags1=(self.scheduler.actors[slot].flags1&0x7f)|1;
        self.targets=0;
        if self.scheduler.actors[slot].gauge.advance_duration==255 {self.scheduler.actors[slot].gauge.advance_duration=0;}
        self.scheduler.actors[slot].flags1&=0xfd;self.retaliation_flags[slot]&=0xfa;
        let head=self.commands.normal[slot];
        if slot>=4 && head&0x80!=0 {
            self.command=31;
            let mp_cost=costs.get(31,self.attack,(slot*2) as u8,&mut self.mp_flags)?;
            self.commands.normal(slot,Payload {command:31,attack:self.attack,targets:0,mp_cost})?;
        } else {
            if slot<4 {self.changed_targets&=!(1<<slot);}
            if slot<4 && self.ai_high[slot]&0x80==0 {hook(self,AdvanceEvent::Ai(slot as u8))?;}
            else if head&0x80!=0 {return Err("Autonomous character command selection is not implemented".into());}
            else {
                let mut id=head;let mut terminal=false;
                for _ in 0..128 {
                    let p=*self.commands.payloads.get(usize::from(id)).ok_or("Advance command pointer outside pool")?;
                    self.command=p.command;self.attack=p.attack;self.targets=p.targets;
                    if p.command<30 {
                        self.scheduler.actors[slot].gauge.advance_duration=delays.add(self.scheduler.actors[slot].gauge.advance_duration,p.command);
                        hook(self,AdvanceEvent::InitTarget(slot as u8,p.command,p.attack))?;
                        if self.targets==0 {hook(self,AdvanceEvent::ChooseTarget(slot as u8))?;}
                    }
                    self.commands.payloads[usize::from(id)].targets=self.targets;
                    let next=self.commands.links[usize::from(id)];
                    if next==id {terminal=true;break;}id=next;
                }
                if !terminal {return Err("Advance command chain has no terminal self-link".into());}
            }
        }
        let head=self.commands.normal[slot];
        if head&0x80==0 {
            let p=self.commands.payloads[usize::from(head)];
            if p.command==22 {return Err("Jump advance presentation/status changes are not implemented".into());}
            if p.command<30 && slot<4 {self.targets=p.targets;hook(self,AdvanceEvent::Graphics(slot as u8,p.command))?;}
        }
        Ok(true)
    }
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let delays=CommandDelays::load(root)?;let costs=MpCosts::from_attacks(&crate::damage::AttackTable::load(root)?);
    let hex=|b:&[u8]|b.iter().map(|v|format!("{v:02X}")).collect::<String>();
    let payloads=|state:&AdvanceState| {
        let mut b=Vec::new();for p in state.commands.payloads {b.extend_from_slice(&[p.command,p.attack]);b.extend_from_slice(&p.targets.to_le_bytes());b.push(p.mp_cost);}hex(&b)
    };
    let mut rows=String::from("case,slot,flags0,flags1,duration,retal,ai,changed,command,attack,targets,mp_flags,links,heads,payloads,prepared,after_flags1,after_duration,after_retal,after_changed,after_command,after_attack,after_targets,after_mp_flags,after_links,after_heads,after_payloads,events\n");
    for case in 0..512usize {
        let slot=case%10;let mut s=AdvanceState::default();s.ai_high=[255;4];
        let count=case%3;
        if slot<4 && (count==0 || case%7==0) {s.ai_high[slot]=0;}
        let flags0=[0,8,0x10,0x40,0x50,0x81][case%6];let flags1=case as u8;
        let duration=[255,0,16,254][case%4];let retal=(case*3) as u8;
        s.scheduler.actors[slot].flags0=flags0;s.scheduler.actors[slot].flags1=flags1;s.scheduler.actors[slot].gauge.advance_duration=duration;
        s.retaliation_flags[slot]=retal;s.changed_targets=15;s.command=case as u8;s.attack=(case*5) as u8;s.targets=0xaaaa;s.mp_flags=if case%5==0 {0x41}else{1};
        for n in 0..count {s.commands.normal(slot,Payload {command:[0,2,29,23,31][(case+n)%5],attack:(case+n) as u8,targets:if (case+n)%4==0 {0}else{0x103},mp_cost:17})?;}
        let before_links=hex(&s.commands.links);let before_heads=hex(&s.commands.normal);let before_payloads=payloads(&s);
        let command=s.command;let attack=s.attack;let targets=s.targets;let mp_flags=s.mp_flags;let mut events=Vec::new();
        let prepared=s.prepare(slot,&delays,&costs,|s,event| {
            if matches!(event,AdvanceEvent::InitTarget(..)) && case%3==1 {s.targets=0;}
            if matches!(event,AdvanceEvent::ChooseTarget(..)) {s.targets=1<<((case+1)%4);}
            events.push(event.label());Ok(())
        })?;
        writeln!(rows,"{case},{slot},{flags0},{flags1},{duration},{retal},{},{},{command},{attack},{targets},{mp_flags},{before_links},{before_heads},{before_payloads},{},{},{},{},{},{},{},{},{},{},{},{},{}",s.ai_high[slot.min(3)],15,u8::from(prepared),s.scheduler.actors[slot].flags1,s.scheduler.actors[slot].gauge.advance_duration,s.retaliation_flags[slot],s.changed_targets,s.command,s.attack,s.targets,s.mp_flags,hex(&s.commands.links),hex(&s.commands.normal),payloads(&s),events.join("|")).unwrap();
    }
    fs::write(directory.join("advance.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn busy_actor_defers_without_changing_payload_or_wait() {
        let root=Path::new("extracted/twue-3.07");let delays=CommandDelays::load(root).unwrap();let costs=MpCosts::from_attacks(&crate::damage::AttackTable::load(root).unwrap());
        let mut s=AdvanceState::default();s.scheduler.actors[0].flags0=0x10;s.scheduler.actors[0].gauge.advance_duration=255;
        assert!(!s.prepare(0,&delays,&costs,|_,_|panic!("deferred actor called a hook")).unwrap());
        assert_eq!(s.scheduler.actors[0].flags1,128);assert_eq!(s.scheduler.actors[0].gauge.advance_duration,255);
    }
    #[test]fn pending_magitek_waits_then_enters_action_queue_with_cached_cost() {
        let root=Path::new("extracted/twue-3.07");let delays=CommandDelays::load(root).unwrap();let costs=MpCosts::from_attacks(&crate::damage::AttackTable::load(root).unwrap());
        let mut s=AdvanceState::default();s.scheduler.actors[0].flags0=0x8b;s.scheduler.actors[0].status1=8;s.scheduler.actors[0].gauge.increment=512;
        let mut input=crate::battle_commands::PlayerInputs::default();input.bytes[..8].copy_from_slice(&[0,29,0,0,1,0,0,0]);
        input.apply(&mut s.scheduler,&mut s.commands,&costs,&mut s.mp_flags,[0;4]).unwrap();
        assert_eq!(s.scheduler.queues.dispatch(),Some(crate::battle_scheduler::Work::Advance(0)));
        assert!(s.prepare(0,&delays,&costs,|_,_|Ok(())).unwrap());assert_eq!(s.scheduler.actors[0].gauge.advance_duration,16);
        for frame in (2..=32).step_by(2) {s.scheduler.advance_time(frame,crate::battle_scheduler::TimeContext::default());}
        assert_eq!(s.scheduler.queues.dispatch(),Some(crate::battle_scheduler::Work::Action(0)));
        assert_eq!(s.commands.payloads[127].attack,131);assert_eq!(s.commands.payloads[127].mp_cost,0);
        let mut context=crate::battle_action_init::Context::default();let mut rng=crate::battle_rng::BattleRng {index:9};
        let action=crate::battle_action_init::initialize_normal(&s.commands,0,true,s.scheduler.actors[0].status1,&mut context,&mut rng,&crate::battle_rng::RngTable::load(root).unwrap(),&crate::battle_action_init::CommandProperties::load(root).unwrap()).unwrap().unwrap();
        assert_eq!((action.returned_command,action.command,action.attack,action.targets,action.mp_cost),(29,29,131,256,0));
        assert_eq!(s.commands.normal[0],127);assert_eq!(rng.index,9);
    }
}
