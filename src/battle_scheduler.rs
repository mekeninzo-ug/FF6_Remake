//! Queue and ATB coordination from C2/0019, C2/111B and C2/4E66.
//! Dispatch returns work; it does not pretend that a queued action has executed.
use crate::battle_time::{BattleClock,Gauge};
use std::{fs,path::Path};

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Ring {pub bytes:[u8;256],pub head:u8,pub tail:u8}
impl Default for Ring {fn default()->Self {Self {bytes:[255;256],head:0,tail:0}}}
impl Ring {
    pub fn push(&mut self,actor:u8) {self.bytes[usize::from(self.tail)]=actor;self.tail=self.tail.wrapping_add(1);}
    pub fn pop(&mut self)->Option<u8> {
        while self.head!=self.tail {
            let value=self.bytes[usize::from(self.head)];self.head=self.head.wrapping_add(1);
            if value&0x80==0 {return Some(value);}
        }
        None
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Work {Immediate(u8),Retaliation(u8),Advance(u8),Action(u8)}
#[derive(Clone,Debug,Default)]
pub struct Queues {
    pub advance:Ring,pub action:Ring,pub retaliation:Ring,
    pub immediate:Option<u8>,pub active_retaliation:Option<u8>,pub active_action:Option<u8>,
}
impl Queues {
    /// Source main-loop precedence. Active normal work does not outrank
    /// pending advance-wait work. Callers clear active work after execution.
    pub fn dispatch(&mut self)->Option<Work> {
        if let Some(id)=self.immediate.filter(|id|*id!=255) {return Some(Work::Immediate(id));}
        if let Some(id)=self.active_retaliation.filter(|id|id&0x80==0) {return Some(Work::Retaliation(id));}
        if let Some(id)=self.retaliation.pop() {return Some(Work::Retaliation(id));}
        if let Some(id)=self.advance.pop() {return Some(Work::Advance(id));}
        if let Some(id)=self.active_action.filter(|id|id&0x80==0) {return Some(Work::Action(id));}
        self.action.pop().map(Work::Action)
    }
}
#[derive(Clone,Copy,Debug)]
pub struct Actor {
    pub flags0:u8,pub flags1:u8,pub status1:u8,pub update_flags:u8,pub extra_status:u8,
    pub gauge:Gauge,
}
impl Default for Actor {
    fn default()->Self {Self {flags0:0,flags1:0,status1:0,update_flags:0,extra_status:0,
        gauge:Gauge {counter:0,increment:0,advance_counter:0,advance_duration:255}}}
}
#[derive(Clone,Copy,Debug)]
pub struct TimeContext {
    pub program_stop:u8,pub wait_mode:u8,pub running:u8,
    pub jumping:u8,pub enemy_characters:u8,pub zinger_actor:u8,pub morph_actor:u8,pub menu_flags:u8,
}
impl Default for TimeContext {fn default()->Self {Self {program_stop:0,wait_mode:0,running:0,jumping:0,enemy_characters:0,zinger_actor:255,morph_actor:255,menu_flags:0}}}
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum TimingEvent {Counters,Morph(u8),RunCheck(u8),OpenMenu(u8),Graphics}
impl TimingEvent {pub fn label(&self)->String {match self {Self::Counters=>"C".into(),Self::Morph(id)=>format!("M{id}"),Self::RunCheck(id)=>format!("R{id}"),Self::OpenMenu(id)=>format!("O{id}"),Self::Graphics=>"G".into()}}}
#[derive(Clone,Debug,Default)]
pub struct Scheduler {pub clock:BattleClock,pub actors:[Actor;10],pub queues:Queues}
impl Scheduler {
    pub fn enqueue_advance(&mut self,slot:usize) {self.queues.advance.push((slot*2) as u8);self.actors[slot].flags1&=!1;}
    /// Called after GetPlayerActions has supplied any input-side changes.
    /// Counter/morph/run/graphics work is emitted for separate subsystems.
    pub fn advance_time(&mut self,frame:u8,context:TimeContext)->Vec<TimingEvent> {
        let mut events=Vec::new();let mut context=context;
        self.advance_time_with(frame,&mut context,|_,_,event|events.push(event));events
    }
    pub fn advance_time_with(&mut self,frame:u8,context:&mut TimeContext,mut callback:impl FnMut(&mut Self,&mut TimeContext,TimingEvent)) {
        if !self.clock.update(frame,context.program_stop,context.wait_mode) {return;}
        callback(self,context,TimingEvent::Counters);
        for slot in (0..10).rev() {
            let id=(slot*2) as u8;
            if id==context.morph_actor {callback(self,context,TimingEvent::Morph(id));}
            let shifted=self.actors[slot].flags0>>1;
            if self.actors[slot].flags0&1==0 || id==context.zinger_actor || shifted&0x3a!=0 {continue;}
            let character=slot<4;let mask=if character {1u8<<slot} else {0};
            if shifted&4!=0 && (context.running==0 || self.actors[slot].status1&2!=0 || !character || mask&(context.jumping|context.enemy_characters)!=0) {
                if self.actors[slot].gauge.tick_advance() {
                    self.queues.action.push(id);self.actors[slot].flags0|=0x20;
                }
            }
            if !self.actors[slot].gauge.tick() {continue;}
            if character {callback(self,context,TimingEvent::RunCheck(id));}
            let already=self.actors[slot].flags0&8!=0;
            if !already {
                self.actors[slot].flags0|=8;
                if self.actors[slot].flags0&2==0 {self.enqueue_advance(slot);continue;}
            }
            self.actors[slot].flags0|=0x80;
            if !character {self.enqueue_advance(slot);continue;}
            if self.actors[slot].update_flags&0x80==0 || context.menu_flags&0x80!=0 {continue;}
            self.actors[slot].flags0|=4;
            let menu_id=slot as u8|if self.actors[slot].extra_status&1!=0 {0x80} else {0};
            callback(self,context,TimingEvent::OpenMenu(menu_id));
        }
        callback(self,context,TimingEvent::Graphics);
    }
}

#[derive(Clone)] pub struct CommandDelays {values:[u8;32]}
impl CommandDelays {
    pub fn load(root:&Path)->Result<Self,String> {
        let bytes=fs::read(root.join("build/native_battle/command_delays.bin")).map_err(|e|e.to_string())?;
        Ok(Self {values:bytes.try_into().map_err(|_|"Expected 32 command-delay bytes")?})
    }
    pub fn add(&self,current:u8,command:u8)->u8 {
        if command>=30 {return current;}
        (u16::from(current)+u16::from(self.values[usize::from(command)])).min(254) as u8
    }
}

pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let delays=CommandDelays::load(root)?;let mut rows=String::from("command,before,after\n");
    for command in 0..=255 {for before in [0,1,16,127,223,254,255] {writeln!(rows,"{command},{before},{}",delays.add(before,command)).unwrap();}}
    fs::write(directory.join("command-delays.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,kind,head,tail,operation,actor,after_head,after_tail,value,bytes\n");
    for case in 0..=255u8 {
        let mut q=Ring {head:case,tail:case.wrapping_add(3),..Ring::default()};
        for step in 0..3 {q.bytes[usize::from(case.wrapping_add(step))]=if step==1 {0xfe} else {step*2};}
        for operation in 0..6 {
            let head=q.head;let tail=q.tail;let actor=case%20;
            let value=if operation%2==0 {q.push(actor);255} else {q.pop().unwrap_or(255)};
            let hex=q.bytes.iter().map(|b|format!("{b:02X}")).collect::<String>();
            writeln!(rows,"{case},ring,{head},{tail},{operation},{actor},{},{},{value},{hex}",q.head,q.tail).unwrap();
        }
    }
    fs::write(directory.join("ring-operations.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,immediate,retaliation_active,action_active,head,tail,retaliation,advance,action,work,ret_head,advance_head,action_head\n");
    for case in 0..512usize {
        let mut q=Queues::default();let head=case as u8;let tail=head.wrapping_add(3);
        q.immediate=Some(if case%7==0 {99}else{255});q.active_retaliation=Some(if case%5==0 {6}else{255});q.active_action=Some(if case%3==0 {4}else{255});
        for (kind,ring) in [&mut q.retaliation,&mut q.advance,&mut q.action].iter_mut().enumerate() {
            ring.head=head;ring.tail=if case%(kind+2)==0 {head}else{tail};
            ring.bytes[usize::from(head)]=255;ring.bytes[usize::from(head.wrapping_add(1))]=((kind+1)*2) as u8;ring.bytes[usize::from(head.wrapping_add(2))]=0x80;
        }
        let hex=|ring:&Ring|ring.bytes.iter().map(|b|format!("{b:02X}")).collect::<String>();
        let before=[hex(&q.retaliation),hex(&q.advance),hex(&q.action)];
        let tails=format!("{:02X}{:02X}{:02X}",q.retaliation.tail,q.advance.tail,q.action.tail);
        let work=q.dispatch().map(|work|match work {Work::Immediate(id)=>format!("I{id}"),Work::Retaliation(id)=>format!("R{id}"),Work::Advance(id)=>format!("W{id}"),Work::Action(id)=>format!("A{id}")}).unwrap_or_default();
        writeln!(rows,"{case},{},{},{},{head},{tails},{},{},{},{work},{},{},{}",q.immediate.unwrap(),q.active_retaliation.unwrap(),q.active_action.unwrap(),before[0],before[1],before[2],q.retaliation.head,q.advance.head,q.action.head).unwrap();
    }
    fs::write(directory.join("dispatch.csv"),rows).map_err(|e|e.to_string())?;
    let actor_hex=|actors:&[Actor;10]| {
        let mut bytes=Vec::new();for a in actors {
            bytes.extend_from_slice(&[a.flags0,a.flags1,a.status1,a.update_flags,a.extra_status]);
            for v in [a.gauge.counter,a.gauge.increment,a.gauge.advance_counter] {bytes.extend_from_slice(&v.to_le_bytes());}
            bytes.push(a.gauge.advance_duration);
        }bytes.iter().map(|b|format!("{b:02X}")).collect::<String>()
    };
    let mut rows=String::from("case,frame,previous,turns,stop,wait,running,jumping,enemy_chars,zinger,morph,menu_flags,before,after,clock,after_turns,advance,action,flags1,events\n");
    for case in 0..1024usize {
        let frame=case as u8;let previous=frame.wrapping_sub((case%5) as u8);
        let mut state=Scheduler::default();state.clock=BattleClock {previous_frame:previous,turns:(65500+case) as u16};
        for (slot,a) in state.actors.iter_mut().enumerate() {
            let n=case+slot;
            a.flags0=[1,3,9,11,0,0x21,0x41,0x81][n%8];a.flags1=n as u8;
            a.status1=if n%9==0 {2} else {0};a.update_flags=if n%7!=0 {128} else {0};a.extra_status=(n%2) as u8;
            a.gauge=Gauge {counter:if n%3==0 {0xfff0} else if n%3==1 {0x1234} else {0x0034},increment:if n%2==0 {512}else{100},advance_counter:if n%2==0 {0xffd0}else{0x0160},advance_duration:(n%5) as u8};
        }
        let before=actor_hex(&state.actors);let context=TimeContext {program_stop:if case%7==0 {1}else{0},wait_mode:if case%2==0 {1}else{0},running:(case%2) as u8,jumping:(case%16) as u8,enemy_characters:((case/4)%16) as u8,zinger_actor:if case%3==0 {12}else{255},morph_actor:if case%11==0 {4}else{255},menu_flags:if case%13==0 {128}else{0},..TimeContext::default()};
        let events=state.advance_time(frame,context).iter().map(TimingEvent::label).collect::<Vec<_>>().join("|");
        let queue=|q:&Ring|q.bytes[..usize::from(q.tail)].iter().map(|b|format!("{b:02X}")).collect::<String>();
        let flags=state.actors.iter().map(|a|format!("{:02X}",a.flags1)).collect::<String>();
        writeln!(rows,"{case},{frame},{previous},{},{},{},{},{},{},{},{},{},{before},{},{},{},{},{},{flags},{events}",(65500+case) as u16,context.program_stop,context.wait_mode,context.running,context.jumping,context.enemy_characters,context.zinger_actor,context.morph_actor,context.menu_flags,actor_hex(&state.actors),state.clock.previous_frame,state.clock.turns,queue(&state.queues.advance),queue(&state.queues.action)).unwrap();
    }
    fs::write(directory.join("timing.csv"),rows).map_err(|e|e.to_string())
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn ring_wrap_tombstones_and_full_alias_follow_byte_pointers() {
        let mut q=Ring {head:254,tail:254,..Ring::default()};q.push(0xff);q.push(18);q.push(0xfe);q.push(2);
        assert_eq!(q.pop(),Some(18));assert_eq!(q.pop(),Some(2));assert_eq!(q.pop(),None);
        for _ in 0..256 {q.push(4);}assert_eq!(q.head,q.tail);assert_eq!(q.pop(),None);
    }
    #[test] fn dispatch_prioritizes_advance_over_current_normal_actor() {
        let mut q=Queues {active_action:Some(4),..Queues::default()};q.action.push(6);q.advance.push(2);q.retaliation.push(8);q.immediate=Some(99);
        assert_eq!(q.dispatch(),Some(Work::Immediate(99)));q.immediate=None;
        assert_eq!(q.dispatch(),Some(Work::Retaliation(8)));assert_eq!(q.dispatch(),Some(Work::Advance(2)));
        assert_eq!(q.dispatch(),Some(Work::Action(4)));q.active_action=None;assert_eq!(q.dispatch(),Some(Work::Action(6)));
    }
    #[test] fn ready_automatic_actor_enters_advance_queue_and_clears_flag1() {
        let mut state=Scheduler::default();state.actors[6].flags0=1;state.actors[6].flags1=255;
        state.actors[6].gauge.counter=65530;state.actors[6].gauge.increment=512;
        state.advance_time(2,TimeContext::default());assert_eq!(state.queues.dispatch(),Some(Work::Advance(12)));
        assert_eq!(state.actors[6].flags1,254);assert_eq!(state.actors[6].flags0,9);
    }
    #[test] fn callbacks_mutate_actors_and_context_at_the_source_call_position() {
        let mut state=Scheduler::default();state.actors[0].flags0=1;state.actors[0].update_flags=128;
        state.actors[0].gauge.counter=65530;state.actors[0].gauge.increment=512;
        let mut context=TimeContext::default();let mut events=Vec::new();
        state.advance_time_with(2,&mut context,|state,context,event| {
            if event==TimingEvent::Counters {state.actors[0].flags0|=2;}
            if event==TimingEvent::RunCheck(0) {context.menu_flags|=0x80;}
            events.push(event);
        });
        assert_eq!(events,vec![TimingEvent::Counters,TimingEvent::RunCheck(0),TimingEvent::Graphics]);
        assert_eq!(state.actors[0].flags0,0x8b);assert_eq!(state.queues.dispatch(),None);
        assert_eq!(context.menu_flags,0x80);
    }
}
