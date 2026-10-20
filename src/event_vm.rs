//! FF6 event bytecode, not 65816 instructions. Encodings are taken from the
//! pinned event_cmd.mac and field/obj.asm. Object blocks currently execute to
//! endpoints for static diagnostics. The live path schedules object commands and
//! fixed-point movement; fades/audio remain diagnostic requests.

pub const NARSHE_ENTRY: usize = 0x029a4f;
pub const NARSHE_ENTRY_END: usize = 0x029ad5;
pub const SWITCH_BYTES: usize = 0x700 / 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventActor {
    pub x: i16,
    pub y: i16,
    pub facing: u8,
    pub speed: u8,
    pub visible: bool,
    pub enabled: bool,
    pub graphic: u8,
    pub palette: u8,
    pub collision_enabled: bool,
    pub vehicle: u8,
    pub pose: u8,
    /// Signed displacement from the current tile, in 1/256 pixel units.
    pub offset_x: i16,
    pub offset_y: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventEffect {
    LoadMap { word: u16, x: u8, y: u8, flags: u8 },
    ObjectBlock { object: u8, start: usize, bytes: u8, awaited: bool },
    ObjectWait { object: u8, frames: u16 },
    SupersededBlock { object: u8, next: usize, end: usize },
    WaitFrames(u16),
    SongFadeIn { song: u8, duration: u8 },
    SpcCommand([u8; 3]),
    FadeIn(u8),
    SortObjects,
}

#[derive(Clone)]
pub struct EventState {
    pub map_word: u16,
    pub party_x: i16,
    pub party_y: i16,
    /// Selected field display slots ($07FB...), distinct from membership.
    pub party: [u8; 4],
    pub char_parties:[u8;16],
    pub active_party:u8,
    pub actors: [EventActor; 0x35],
    pub gil: u32,
    pub switches: [u8; SWITCH_BYTES],
    pub camera_locked: bool,
    pub player_control: bool,
    pub effects: Vec<EventEffect>,
}

impl Default for EventState {
    fn default() -> Self {
        let mut actors=[EventActor::default();0x35];
        actors[0].enabled=true;actors[0].palette=2;actors[0].speed=2;
        let mut char_parties=[0;16];char_parties[0]=1;
        Self { map_word: 0, party_x: 0, party_y: 0, party: [0, 0xff, 0xff, 0xff],
            char_parties,active_party:1,actors, gil: 0, switches: [0; SWITCH_BYTES],
            camera_locked: false, player_control:true, effects: Vec::new() }
    }
}

impl EventState {
    /// Known preconditions from `_cc985b`, whose actor scheduler is not ported.
    pub fn opening_context() -> Self {
        let mut state = Self::default();
        state.party = [14, 15, 0, 0xff];
        for id in [0, 14, 15] {
            state.actors[id].enabled=true;
            state.actors[id].graphic=if id==0 {0} else {14};
            state.actors[id].palette=if id==0{2}else{1};
            state.char_parties[id]=1;
            state.actors[id].speed = 2;
            state.actors[id].vehicle = 0xc0;
        }
        state.set_switch(0x1cc, true);
        state
    }

    pub fn leader_index(&self)->usize {active_slots(self).into_iter().next().unwrap_or(0x30)}

    pub fn switch(&self, id: usize) -> bool {
        self.switches.get(id / 8).is_some_and(|byte| byte & (1 << (id % 8)) != 0)
    }

    pub fn set_switch(&mut self, id: usize, value: bool) {
        if let Some(byte) = self.switches.get_mut(id / 8) {
            let mask = 1 << (id % 8);
            if value { *byte |= mask; } else { *byte &= !mask; }
        }
    }
}

pub fn capture_opening_entry(code: &[u8], directory: &std::path::Path) -> std::io::Result<()> {
    use std::fs;
    let invalid = |message: String| std::io::Error::new(std::io::ErrorKind::InvalidData, message);
    let mut vm = EventVm::new(NARSHE_ENTRY);
    let mut state = EventState::opening_context();
    fs::create_dir_all(directory)?;
    let mut snapshots = String::from("stage,event_offset,map,gil,party,terra_x,terra_y,terra_vehicle,wedge_x,wedge_y,wedge_visible,vicks_x,vicks_y,vicks_visible,switch_01cc,switch_01c1\n");
    for stage in ["dialogue_11", "returned"] {
        let yielded = vm.run_until_yield(code, &mut state).map_err(|error| invalid(error.to_string()))?;
        let expected = if stage == "dialogue_11" { EventYield::Dialogue { id: 11, flags: 0 } }
            else { EventYield::Returned };
        if yielded != expected { return Err(invalid(format!("Unexpected event yield: {yielded:?}"))); }
        let terra = state.actors[0];
        let wedge = state.actors[14];
        let vicks = state.actors[15];
        snapshots.push_str(&format!("{stage},0x{:06X},{},{},{}:{}:{}:{},{},{},{},{},{},{},{},{},{},{},{}\n",
            vm.pc, state.map_word & 0x1ff, state.gil,
            state.party[0], state.party[1], state.party[2], state.party[3],
            terra.x, terra.y, terra.vehicle, wedge.x, wedge.y, u8::from(wedge.visible),
            vicks.x, vicks.y, u8::from(vicks.visible), u8::from(state.switch(0x1cc)),
            u8::from(state.switch(0x1c1))));
    }
    let mut cursor = NARSHE_ENTRY;
    let mut commands = String::from("event_offset,cpu,file_offset,scope,object,opcode,length\n");
    for entry in &vm.trace {
        if entry.pc != cursor { return Err(invalid(String::from("Event trace has a coverage gap"))); }
        cursor += entry.bytes;
        commands.push_str(&format!("0x{:06X},0x{:06X},0x{:06X},{},{},0x{:02X},{}\n",
            entry.pc, entry.pc + 0xca0000, entry.pc + 0x0a0000,
            if entry.object.is_some() { "object" } else { "event" },
            entry.object.map(|id| id.to_string()).unwrap_or_default(), entry.opcode, entry.bytes));
    }
    if cursor != NARSHE_ENTRY_END { return Err(invalid(String::from("Unexpected event return address"))); }
    fs::write(directory.join("snapshots.csv"), snapshots)?;
    fs::write(directory.join("commands.csv"), commands)?;
    let effects = state.effects.iter().map(|effect| format!("{effect:?}\n")).collect::<String>();
    fs::write(directory.join("requests.txt"), effects)
}

/// Frame-by-frame live scheduler evidence, separate from the structural decoder.
pub fn capture_opening_timeline(code: &[u8], directory: &std::path::Path) -> std::io::Result<()> {
    use std::fs;
    fs::create_dir_all(directory)?;
    let mut vm = EventVm::scheduled(NARSHE_ENTRY);
    let mut state = EventState::opening_context();
    let mut rows = String::from("frame,event_offset,yield,object,x,y,offset_x,offset_y,visible,facing,gil\n");
    let mut returned = false;
    for _ in 0..36000 {
        let yielded = vm.step_frame(code, &mut state)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?;
        let stage = match yielded { EventYield::Pending => "pending", EventYield::Dialogue { .. } => "dialogue", EventYield::Returned => "returned", EventYield::Battle {..} => "battle" };
        for id in [0, 14, 15] {
            let actor = state.actors[id];
            rows.push_str(&format!("{},0x{:06X},{stage},{id},{},{},{},{},{},{},{}\n", vm.frame, vm.pc,
                actor.x, actor.y, actor.offset_x, actor.offset_y, u8::from(actor.visible), actor.facing, state.gil));
        }
        // The diagnostic acknowledges the one blocking dialogue on the next
        // logical tick, without asserting any original player response time.
        if yielded == EventYield::Returned { returned = true; break; }
    }
    if !returned { return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "scheduler did not return")); }
    fs::write(directory.join("timeline.csv"), rows)?;
    let mut commands = String::from("frame,event_offset,cpu,file_offset,scope,object,opcode,length\n");
    for entry in &vm.trace {
        commands.push_str(&format!("{},0x{:06X},0x{:06X},0x{:06X},{},{},0x{:02X},{}\n", entry.frame,
            entry.pc, entry.pc + 0xca0000, entry.pc + 0x0a0000,
            if entry.object.is_some() { "object" } else { "event" },
            entry.object.map(|id| id.to_string()).unwrap_or_default(), entry.opcode, entry.bytes));
    }
    fs::write(directory.join("scheduled-commands.csv"), commands)?;
    fs::write(directory.join("scheduled-requests.txt"), state.effects.iter().map(|effect| format!("{effect:?}\n")).collect::<String>())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventYield {
    Pending,
    Dialogue { id: u16, flags: u16 },
    Returned,
    Battle {group:u8,flags:u8},
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventError {
    pub pc: usize,
    pub message: String,
}

impl std::fmt::Display for EventError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "event ${:06X}, file 0x{:06X}: {}",
            0xca0000usize.saturating_add(self.pc), 0x0a0000usize.saturating_add(self.pc), self.message)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EventTrace {
    pub pc: usize,
    pub opcode: u8,
    pub bytes: usize,
    pub object: Option<u8>,
    pub frame: u64,
}

pub struct EventVm {
    pub pc: usize,
    pub trace: Vec<EventTrace>,
    finished: bool,
    scheduled: bool,
    pub frame: u64,
    tasks: [Option<ObjectTask>; 0x35],
    awaiting: Option<u8>,
    pause: u16,
    sort_pending: bool,
    calls: Vec<CallFrame>,
    pub pending_battle: Option<(u8,u8)>,
    slots:Vec<usize>,
}

#[derive(Clone,Copy)]
struct CallFrame {start:usize,return_pc:usize,remaining:u16}


#[derive(Clone, Copy)]
struct ObjectTask {
    pc: usize,
    end: usize,
    wait: u8,
    steps: u8,
    direction: u8,
    moving: bool,
}

fn byte(code: &[u8], pc: &mut usize) -> Result<u8, EventError> {
    let value = code.get(*pc).copied().ok_or_else(|| EventError {
        pc: *pc, message: String::from("truncated command"),
    })?;
    *pc += 1;
    Ok(value)
}

fn word(code: &[u8], pc: &mut usize) -> Result<u16, EventError> {
    Ok(u16::from_le_bytes([byte(code, pc)?, byte(code, pc)?]))
}

pub(crate) fn active_slots(state:&EventState)->Vec<usize> {
    let mut slots=Vec::new();
    for id in state.party {
        let id=usize::from(id);
        if id<16 && state.actors[id].enabled && state.char_parties[id]==state.active_party && !slots.contains(&id) {slots.push(id);}
    }
    for id in 0..16 {if state.actors[id].enabled && state.char_parties[id]==state.active_party && !slots.contains(&id) {slots.push(id);}}
    slots.push(0x30); // camera follows all active-party characters
    for id in 0..0x30 {if state.actors[id].enabled && !slots.contains(&id) {slots.push(id);}}
    slots.truncate(24);
    slots
}

fn pointer(code:&[u8],pc:&mut usize)->Result<usize,EventError> {
    Ok(usize::from(word(code,pc)?)|(usize::from(byte(code,pc)?)<<16))
}
fn valid_target(code:&[u8],target:usize,start:usize)->Result<(),EventError> {
    if target>=code.len(){Err(EventError {pc:start,message:format!("event target ${target:06X} outside script export")})}else{Ok(())}
}
fn resolve_object(state:&EventState,raw:u8,pc:usize)->Result<u8,EventError> {
    let id=if (0x31..=0x34).contains(&raw) {state.party[usize::from(raw-0x31)]} else {raw};
    if usize::from(id)>=state.actors.len(){Err(EventError {pc,message:format!("object alias ${raw:02X} has no supported actor")})}else{Ok(id)}
}

impl EventVm {
    pub fn new(pc: usize) -> Self {
        Self { pc, trace: Vec::new(), finished: false, scheduled: false, frame: 0,
            tasks: [None; 0x35], awaiting: None, pause: 0, sort_pending: false,calls:Vec::new(),pending_battle:None,slots:Vec::new() }
    }

    pub fn scheduled(pc: usize) -> Self {
        let mut vm = Self::new(pc);
        vm.scheduled = true;
        vm
    }

    pub fn actor_order(&self)->&[usize] {&self.slots}
    pub fn refresh_actor_order(&mut self,state:&EventState) {self.slots=active_slots(state);}

    /// One logical field update. Active-party slots follow `_c0714a`; other
    /// object slots still use a provisional order until full object loading exists.
    pub fn step_frame(&mut self, code: &[u8], state: &mut EventState) -> Result<EventYield, EventError> {
        if let Some((group,flags))=self.pending_battle {return Ok(EventYield::Battle {group,flags});}
        self.frame += 1;
        if self.slots.is_empty() || self.frame%4==0 {self.refresh_actor_order(state);}
        // reset.asm::FieldLoop dispatches events, then object commands, then
        // fixed-point position updates. An awaited completion is seen next tick.
        if self.sort_pending && self.frame % 4 == 0 { self.sort_pending = false; }
        let result = if self.pause != 0 {
            self.pause -= 1;
            EventYield::Pending
        } else if self.awaiting.is_some() || self.sort_pending { EventYield::Pending }
        else { self.run_until_yield(code, state)? };
        for (slot,object) in self.slots.clone().into_iter().enumerate() {
            let Some(mut task) = self.tasks[object].take() else { continue };
            let phase = ((slot / 6) % 4) as u64;
            if !task.moving && state.actors[object].offset_x==0 && state.actors[object].offset_y==0 && self.frame % 4 == phase {
                if task.wait != 0 { task.wait -= 1; }
                else if task.steps != 0 {
                    task.steps -= 1;
                    task.moving = true;
                } else if self.schedule_object_commands(code, object as u8, &mut task, state)? {
                    if self.awaiting == Some(object as u8) { self.awaiting = None; }
                    continue;
                }
            }
            self.tasks[object]=Some(task);
        }
        // All object commands precede all position updates in FieldLoop.
        for object in self.slots.clone() {
            let Some(mut task)=self.tasks[object].take() else {continue};
            let actor = &mut state.actors[object];
            if task.moving {
                let rate = 64i16 << actor.speed.min(5);
                let (dx, dy) = [(0, -1), (1, 0), (0, 1), (-1, 0)][usize::from(task.direction)];
                actor.offset_x += dx * rate;
                actor.offset_y += dy * rate;
                if actor.offset_x.abs().max(actor.offset_y.abs()) >= 4096 {
                    actor.x += dx;
                    actor.y += dy;
                    actor.offset_x = 0;
                    actor.offset_y = 0;
                    task.moving = false;
                }
            }
            self.tasks[object] = Some(task);
        }
        Ok(result)
    }

    fn schedule_object_commands(&mut self, code: &[u8], object: u8, task: &mut ObjectTask,
        state: &mut EventState) -> Result<bool, EventError> {
        for _ in 0..128 {
            let start = task.pc;
            if start >= task.end { return Err(EventError { pc: start, message: String::from("object block has no terminator") }); }
            let opcode = byte(&code[..task.end], &mut task.pc)?;
            let leader=state.actors[state.leader_index()];
            let actor = &mut state.actors[usize::from(object)];
            match opcode {
                0x00..=0x7f => actor.pose = opcode,
                0x80..=0x9f => {
                    task.direction = opcode & 3;
                    actor.facing = task.direction;
                    task.steps = ((opcode - 0x80) >> 2) + 1;
                    task.steps -= 1;
                    task.moving = true;
                }
                0xc0..=0xc5 => actor.speed = opcode - 0xc0,
                0xcc..=0xcf => actor.facing = opcode - 0xcc,
                0xd0 => actor.visible = true,
                0xd1 => actor.visible = false,
                0xd5 => {
                    actor.x = i16::from(byte(&code[..task.end], &mut task.pc)?);
                    actor.y = i16::from(byte(&code[..task.end], &mut task.pc)?);
                    actor.offset_x = 0; actor.offset_y = 0;
                }
                0xd7 => {
                    actor.x=leader.x;actor.y=leader.y;
                    actor.offset_x=leader.offset_x.div_euclid(256)*256;
                    actor.offset_y=leader.offset_y.div_euclid(256)*256;
                }
                0xe0 => {
                    task.wait = byte(&code[..task.end], &mut task.pc)?;
                    state.effects.push(EventEffect::ObjectWait { object, frames: u16::from(task.wait) * 4 });
                }
                0xff => {
                    if task.pc != task.end { return Err(EventError { pc: task.pc, message: String::from("bytes follow object-script terminator") }); }
                }
                _ => return Err(EventError { pc: start, message: format!("unsupported object opcode ${opcode:02X}") }),
            }
            self.trace.push(EventTrace { pc: start, opcode, bytes: task.pc - start, object: Some(object), frame: self.frame });
            if opcode == 0xff { return Ok(true); }
            if task.moving || matches!(opcode,0xd5|0xd7|0xe0) { return Ok(false); }
        }
        Err(EventError { pc: task.pc, message: String::from("object instruction budget exhausted") })
    }

    pub fn run_until_yield(&mut self, code: &[u8], state: &mut EventState) -> Result<EventYield, EventError> {
        if let Some((group,flags))=self.pending_battle {return Ok(EventYield::Battle {group,flags});}
        if self.finished { return Ok(EventYield::Returned); }
        for _ in 0..16384 {
            let start = self.pc;
            let opcode = byte(code, &mut self.pc)?;
            if opcode <= 0x34 {
                let length_flags = byte(code, &mut self.pc)?;
                let length = length_flags & 0x7f;
                let block_start = self.pc;
                let block_end = block_start + usize::from(length);
                if length == 0 || block_end > code.len() {
                    return Err(EventError { pc: start, message: String::from("invalid object block length") });
                }
                self.trace.push(EventTrace { pc: start, opcode, bytes: 2, object: None, frame: self.frame });
                let object=resolve_object(state,opcode,start)?;
                state.effects.push(EventEffect::ObjectBlock { object, start: block_start,
                    bytes: length, awaited: length_flags & 0x80 != 0 });
                self.pc = block_end;
                if self.scheduled {
                    if let Some(old) = self.tasks[usize::from(object)] {
                        state.effects.push(EventEffect::SupersededBlock { object, next: old.pc, end: old.end });
                    }
                    let old = self.tasks[usize::from(object)];
                    self.tasks[usize::from(object)] = Some(ObjectTask { pc: block_start, end: block_end,
                        wait: old.map(|task| task.wait).unwrap_or(0), steps: 0,
                        direction: old.map(|task| task.direction).unwrap_or(0),
                        moving: old.is_some_and(|task| task.moving) });
                    if length_flags & 0x80 != 0 {
                        self.awaiting = Some(object);
                        return Ok(EventYield::Pending);
                    }
                } else { self.run_object(code, object, block_start, block_end, state)?; }
                continue;
            }
            let mut yielded = None;
            let mut jump=None;
            match opcode {
                0x35 => {
                    let raw=byte(code,&mut self.pc)?;
                    let object=resolve_object(state,raw,start)?;
                    if self.scheduled && self.tasks[usize::from(object)].is_some() {self.awaiting=Some(object);yielded=Some(EventYield::Pending);}
                }
                0x36 | 0x3d | 0x3e | 0x41 | 0x42 | 0x78 => {
                    let raw=byte(code,&mut self.pc)?;
                    let object=usize::from(resolve_object(state,raw,start)?);
                    let actor = state.actors.get_mut(object).ok_or_else(|| EventError {
                        pc: start, message: format!("object {object} is outside the supported object table"),
                    })?;
                    if matches!(opcode,0x3d|0x41) && actor.graphic==255 {
                        return Err(EventError {pc:start,message:String::from("special/animated NPC format is not implemented")});
                    }
                    match opcode {
                        0x36 => actor.collision_enabled = true,
                        0x3d => {if !actor.enabled {actor.enabled=true;actor.visible=false;}},
                        0x3e => {actor.enabled=false;actor.visible=false;},
                        0x78 => actor.collision_enabled = false,
                        0x41 => actor.visible = true,
                        _ => actor.visible = false,
                    }
                }
                0x3a => state.player_control=true,
                0x3b => state.player_control=false,
                0x38 => state.camera_locked = true,
                0x39 => state.camera_locked = false,
                0x3c => {
                    for slot in &mut state.party {
                        let id=byte(code,&mut self.pc)?;
                        if id!=255 && id>=16 {return Err(EventError {pc:start,message:String::from("invalid field character slot")});}
                        *slot=id;
                    }
                }
                0x3f => {
                    let id=usize::from(byte(code,&mut self.pc)?);let party=byte(code,&mut self.pc)?;
                    if id>=16 || party>7 {return Err(EventError {pc:start,message:String::from("invalid character party membership")});}
                    state.char_parties[id]=party;
                }
                0x44 => {
                    let raw=byte(code,&mut self.pc)?;
                    let object=usize::from(resolve_object(state,raw,start)?);
                    let vehicle = byte(code, &mut self.pc)?;
                    let actor = state.actors.get_mut(object).ok_or_else(|| EventError {
                        pc: start, message: format!("invalid vehicle object {object}"),
                    })?;
                    actor.vehicle = vehicle;
                }
                0x45 => state.effects.push(EventEffect::SortObjects),
                0x4b => {
                    let value = word(code, &mut self.pc)?;
                    yielded = Some(EventYield::Dialogue { id: value & 0x3fff, flags: value & 0xc000 });
                }
                0x4d => {
                    let group=byte(code,&mut self.pc)?;let flags=byte(code,&mut self.pc)?;
                    self.pending_battle=Some((group,flags));yielded=Some(EventYield::Battle {group,flags});
                }
                0x59 => {
                    let rate = byte(code, &mut self.pc)?;
                    state.effects.push(EventEffect::FadeIn(rate));
                }
                0x6b => {
                    let map_word = word(code, &mut self.pc)?;
                    let x = byte(code, &mut self.pc)?;
                    let y = byte(code, &mut self.pc)?;
                    let flags = byte(code, &mut self.pc)?;
                    state.map_word = map_word;
                    state.party_x = i16::from(x);
                    state.party_y = i16::from(y);
                    let leader=state.leader_index();
                    if let Some(actor) = state.actors.get_mut(leader) {
                        actor.x = state.party_x;
                        actor.y = state.party_y;
                        actor.facing = ((map_word >> 12) & 3) as u8;
                    }
                    state.effects.push(EventEffect::LoadMap { word: map_word, x, y, flags });
                }
                0x84 => state.gil = state.gil.saturating_add(u32::from(word(code, &mut self.pc)?)),
                0x91..=0x95 => state.effects.push(EventEffect::WaitFrames(
                    [15, 30, 45, 60, 120][usize::from(opcode - 0x91)])),
                0xb5 => {
                    let ticks = byte(code, &mut self.pc)?;
                    state.effects.push(EventEffect::WaitFrames(u16::from(ticks) * 15));
                }
                0xb2 | 0xb3 => {
                    let count=if opcode==0xb3 {u16::from(byte(code,&mut self.pc)?)} else {1};
                    let target=pointer(code,&mut self.pc)?;
                    valid_target(code,target,start)?;
                    if self.calls.len()>=16 {return Err(EventError {pc:start,message:String::from("event call stack exhausted")});}
                    self.calls.push(CallFrame {start:target,return_pc:self.pc,remaining:if count==0 {256} else {count}});
                    jump=Some(target);
                }
                0xc0..=0xcf => {
                    let count=usize::from((opcode&7)+1);let mut any=false;let mut all=true;
                    for _ in 0..count {
                        let test=word(code,&mut self.pc)?;let bit=usize::from(test&0x7fff);
                        if bit>=0x700 {return Err(EventError {pc:start,message:String::from("event condition bit outside switch table")});}
                        let matched=state.switch(bit)==(test&0x8000!=0);any|=matched;all&=matched;
                    }
                    let target=pointer(code,&mut self.pc)?;
                    if if opcode<0xc8 {any} else {all} {valid_target(code,target,start)?;jump=Some(target);}
                }
                0xd0..=0xdd => {
                    let low = usize::from(byte(code, &mut self.pc)?);
                    state.set_switch(usize::from((opcode - 0xd0) / 2) * 256 + low, opcode & 1 == 0);
                }
                0xf1 => {
                    let song = byte(code, &mut self.pc)?;
                    let duration = byte(code, &mut self.pc)?;
                    state.effects.push(EventEffect::SongFadeIn { song, duration });
                }
                0xf6 => state.effects.push(EventEffect::SpcCommand([
                    byte(code, &mut self.pc)?, byte(code, &mut self.pc)?, byte(code, &mut self.pc)?,
                ])),
                0xfb | 0xfd => {if self.scheduled {yielded=Some(EventYield::Pending);}},
                0xfe => {
                    if let Some(frame)=self.calls.last_mut() {
                        if frame.remaining>1 {frame.remaining-=1;jump=Some(frame.start);}
                        else {let frame=self.calls.pop().unwrap();jump=Some(frame.return_pc);}
                    } else {self.finished=true;yielded=Some(EventYield::Returned);}
                }
                _ => return Err(EventError { pc: start, message: format!("unsupported event opcode ${opcode:02X}") }),
            }
            self.trace.push(EventTrace { pc: start, opcode, bytes: self.pc - start, object: None, frame: self.frame });
            if let Some(target)=jump {self.pc=target;}
            if self.scheduled {
                if opcode == 0x6b { return Ok(EventYield::Pending); }
                if matches!(opcode,0x3c|0x45) { self.sort_pending = true; return Ok(EventYield::Pending); }
                let wait = match state.effects.last() {
                    Some(EventEffect::WaitFrames(frames)) if matches!(opcode, 0x91..=0x95 | 0xb5) => *frames,
                    _ => 0,
                };
                if wait != 0 { self.pause = wait - 1; return Ok(EventYield::Pending); }
            }
            if let Some(result) = yielded { return Ok(result); }
        }
        Err(EventError { pc: self.pc, message: String::from("instruction budget exhausted") })
    }

    fn run_object(&mut self, code: &[u8], object: u8, mut pc: usize, end: usize,
        state: &mut EventState) -> Result<(), EventError> {
        let actor = &mut state.actors[usize::from(object)];
        while pc < end {
            let start = pc;
            // Slice at the declared block end so operands cannot consume the
            // following event command if an object block is malformed.
            let opcode = byte(&code[..end], &mut pc)?;
            match opcode {
                0x00..=0x7f => actor.pose = opcode,
                0x80..=0x9f => {
                    let steps = i16::from(((opcode - 0x80) >> 2) + 1);
                    let (dx, dy) = [(0, -1), (1, 0), (0, 1), (-1, 0)][usize::from(opcode & 3)];
                    actor.x += dx * steps;
                    actor.y += dy * steps;
                    actor.facing = opcode & 3;
                }
                0xc0..=0xc5 => actor.speed = opcode - 0xc0,
                0xcc..=0xcf => actor.facing = opcode - 0xcc,
                0xd0 => actor.visible = true,
                0xd1 => actor.visible = false,
                0xd5 => {
                    actor.x = i16::from(byte(&code[..end], &mut pc)?);
                    actor.y = i16::from(byte(&code[..end], &mut pc)?);
                }
                0xd7 => { actor.x = state.party_x; actor.y = state.party_y; }
                0xe0 => {
                    let duration = byte(&code[..end], &mut pc)?;
                    state.effects.push(EventEffect::ObjectWait { object, frames: u16::from(duration) * 4 });
                }
                0xff => {}
                _ => return Err(EventError { pc: start, message: format!("unsupported object opcode ${opcode:02X}") }),
            }
            self.trace.push(EventTrace { pc: start, opcode, bytes: pc - start, object: Some(object), frame: self.frame });
            if opcode == 0xff {
                if pc != end {
                    return Err(EventError { pc, message: String::from("bytes follow object-script terminator") });
                }
                return Ok(());
            }
        }
        Err(EventError { pc: end, message: String::from("object block has no terminator") })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_code() -> Vec<u8> {
        std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("extracted/twue-3.07/event/event_script.bin")).unwrap()
    }

    #[test]
    fn target_opening_entry_yields_dialogue_then_returns_with_party_state() {
        let code = target_code();
        let mut vm = EventVm::new(NARSHE_ENTRY);
        let mut state = EventState::opening_context();
        assert_eq!(vm.run_until_yield(&code, &mut state).unwrap(), EventYield::Dialogue { id: 11, flags: 0 });
        assert_eq!(state.map_word & 0x1ff, 19);
        assert_eq!(state.party, [0, 14, 15, 255]);
        assert_eq!((state.actors[0].x, state.actors[0].y, state.gil), (38, 50, 3000));
        assert_eq!((state.actors[14].x, state.actors[14].y), (38, 54));
        assert_eq!((state.actors[15].x, state.actors[15].y), (40, 53));
        assert!(!state.switch(0x1cc));
        for id in [0, 14, 15] { assert_eq!(state.actors[id].vehicle, 0xc0); }
        assert_eq!(vm.run_until_yield(&code, &mut state).unwrap(), EventYield::Returned);
        assert_eq!(vm.pc, NARSHE_ENTRY_END);
        assert_eq!((state.actors[0].x, state.actors[0].y, state.gil), (38, 49, 3000));
        for id in [14, 15] {
            assert_eq!((state.actors[id].x, state.actors[id].y), (38, 50));
            assert!(!state.actors[id].visible);
            assert!(state.actors[id].collision_enabled);
        }
        assert!(state.switch(0x1c1));
        let mut covered = NARSHE_ENTRY;
        for entry in &vm.trace { assert_eq!(entry.pc, covered); covered += entry.bytes; }
        assert_eq!(covered, NARSHE_ENTRY_END);
        let gil = state.gil;
        assert_eq!(vm.run_until_yield(&code, &mut state).unwrap(), EventYield::Returned);
        assert_eq!(state.gil, gil);
    }

    #[test]
    fn requests_preserve_waits_audio_and_async_blocks() {
        let mut vm = EventVm::new(NARSHE_ENTRY);
        let mut state = EventState::default();
        vm.run_until_yield(&target_code(), &mut state).unwrap();
        assert!(state.effects.contains(&EventEffect::WaitFrames(30)));
        assert!(state.effects.contains(&EventEffect::SongFadeIn { song: 0x39, duration: 160 }));
        assert!(state.effects.contains(&EventEffect::SpcCommand([0x82, 0, 0xff])));
        assert!(state.effects.iter().any(|effect| matches!(effect,
            EventEffect::ObjectBlock { object: 0, awaited: false, .. })));
        let mut short = EventState::default();
        EventVm::new(0).run_until_yield(&[0x95, 0xfe], &mut short).unwrap();
        assert_eq!(short.effects, [EventEffect::WaitFrames(120)]);
    }


    #[test]
    fn scheduled_movement_uses_source_fixed_point_speed_and_awaits_end() {
        // One tile north at normal speed: $40 * 4 = $0100 units/frame.
        let code = [0, 0x83, 0xc2, 0x80, 0xff, 0xfe];
        let mut state = EventState::default();
        let mut vm = EventVm::scheduled(0);
        for _ in 0..4 { assert_eq!(vm.step_frame(&code, &mut state).unwrap(), EventYield::Pending); }
        assert_eq!((state.actors[0].y, state.actors[0].offset_y), (0, -256));
        vm.step_frame(&code, &mut state).unwrap();
        assert_eq!((state.actors[0].y, state.actors[0].offset_y), (0, -512));
        for _ in 0..15 { assert_eq!(vm.step_frame(&code, &mut state).unwrap(), EventYield::Pending); }
        assert_eq!(vm.step_frame(&code, &mut state).unwrap(), EventYield::Returned);
        assert_eq!((vm.frame, state.actors[0].y, state.actors[0].offset_y), (21, -1, 0));
    }

    #[test]
    fn scheduled_entry_moves_actors_concurrently_and_matches_structural_endpoints() {
        let code = target_code();
        let mut state = EventState::opening_context();
        let mut vm = EventVm::scheduled(NARSHE_ENTRY);
        let mut overlap = false;
        let mut dialogue = false;
        for _ in 0..3600 {
            let yielded = vm.step_frame(&code, &mut state).unwrap();
            overlap |= [0, 14, 15].iter().filter(|id| state.actors[**id].offset_y != 0).count() >= 2;
            if let EventYield::Dialogue { id, .. } = yielded {
                assert_eq!(id, 11);
                assert_eq!((state.actors[0].x, state.actors[0].y), (38, 50));
                dialogue = true;
            }
            if yielded == EventYield::Returned { break; }
        }
        assert!(dialogue && overlap && vm.finished);
        assert_eq!((state.actors[0].x, state.actors[0].y, state.gil), (38, 49, 3000));
        for id in [14, 15] { assert!(!state.actors[id].visible); }
        assert!(state.switch(0x1c1));
        assert!(state.effects.iter().any(|effect| matches!(effect, EventEffect::SupersededBlock { object: 15, .. })));
    }

    #[test]
    fn event_calls_repeat_and_condition_groups_follow_polarity() {
        let code=[0xb3,3,9,0,0,0x84,2,0,0xfe,0x84,1,0,0xfe];
        let mut state=EventState::default();let mut vm=EventVm::new(0);
        assert_eq!(vm.run_until_yield(&code,&mut state).unwrap(),EventYield::Returned);assert_eq!(state.gil,5);
        let mut repeated=code;repeated[1]=0;let mut state=EventState::default();
        EventVm::new(0).run_until_yield(&repeated,&mut state).unwrap();assert_eq!(state.gil,258);
        for opcode in 0xc0u8..=0xcf {
            let count=usize::from((opcode&7)+1);
            for mask in 0..(1usize<<count) {
                let mut code=vec![opcode];let mut matches=Vec::new();let mut state=EventState::default();
                for bit in 0..count {
                    let want=bit%2==0;let actual=mask&(1<<bit)!=0;state.set_switch(bit,actual);matches.push(want==actual);
                    code.extend_from_slice(&((bit as u16)|if want{0x8000}else{0}).to_le_bytes());
                }
                let target=code.len()+3+4;
                code.extend_from_slice(&[target as u8,0,0]);code.extend_from_slice(&[0x84,1,0,0xfe,0x84,2,0,0xfe]);
                EventVm::new(0).run_until_yield(&code,&mut state).unwrap();
                let jump=if opcode<0xc8 {matches.iter().any(|value|*value)}else{matches.iter().all(|value|*value)};
                assert_eq!(state.gil,if jump{2}else{1});
            }
        }
    }

    #[test]
    fn party_slot_aliases_resolve_for_object_scripts_and_vehicle_commands() {
        let mut state=EventState::default();
        let code=[0x31,0x82,0xcd,0xff,0x44,0x31,0xc0,0xfe];
        EventVm::new(0).run_until_yield(&code,&mut state).unwrap();
        assert_eq!((state.actors[0].facing,state.actors[0].vehicle),(1,0xc0));
        assert_eq!(state.actors[0x31].vehicle,0);
        let mut context=EventState::opening_context();context.party=[0,255,255,255];
        assert_eq!(&active_slots(&context)[..4],&[0,14,15,0x30]);
        EventVm::new(0).run_until_yield(&[0x3f,14,0,0xfe],&mut context).unwrap();
        assert_eq!(context.char_parties[14],0);assert_eq!(context.party,[0,255,255,255]);
        assert_eq!(&active_slots(&context)[..4],&[0,15,0x30,14]);
    }

    #[test]
    fn cached_slots_and_two_phase_updates_match_field_loop_order() {
        let mut state=EventState::default();state.actors[16].graphic=52;state.actors[16].palette=1;
        let code=[0x3d,16,16,0x84,0xd5,1,1,0xff,0xfe];
        let mut vm=EventVm::scheduled(0);vm.frame=3;
        vm.step_frame(&code,&mut state).unwrap();
        assert!(state.actors[16].enabled);assert!(!vm.actor_order().contains(&16));
        assert_eq!(state.actors[16].x,0);
        let bytes=encode_snapshot(&state,Some(&vm));let (_,restored)=decode_snapshot(&code,&bytes).unwrap();
        assert_eq!(restored.unwrap().actor_order(),vm.actor_order());
        for _ in 0..4 {vm.step_frame(&code,&mut state).unwrap();}
        assert!(vm.actor_order().contains(&16));assert_eq!((state.actors[16].x,state.actors[16].y),(1,1));
        for _ in 0..4 {assert_eq!(vm.step_frame(&code,&mut state).unwrap(),EventYield::Pending);}
        assert_eq!(vm.step_frame(&code,&mut state).unwrap(),EventYield::Returned);
        let mut state=EventState::default();state.actors[14].enabled=true;state.char_parties[14]=1;
        let code=[0,2,0x82,0xff,14,0x82,0xd7,0xff,0xfe];let mut vm=EventVm::scheduled(0);vm.frame=3;
        vm.step_frame(&code,&mut state).unwrap();
        assert_eq!(state.actors[0].offset_y,256);assert_eq!(state.actors[14].offset_y,0);
    }

    #[test]
    fn snapshot_keeps_call_pause_and_rejects_corruption() {
        let code=[0xb2,8,0,0,0x84,2,0,0xfe,0x91,0x84,1,0,0xfe];
        let mut state=EventState::default();let mut vm=EventVm::scheduled(0);
        assert_eq!(vm.step_frame(&code,&mut state).unwrap(),EventYield::Pending);
        let bytes=encode_snapshot(&state,Some(&vm));let (mut restored,copy)=decode_snapshot(&code,&bytes).unwrap();let mut copy=copy.unwrap();
        assert_eq!(encode_snapshot(&restored,Some(&copy)),bytes);
        loop {
            let result=vm.step_frame(&code,&mut state).unwrap();let result2=copy.step_frame(&code,&mut restored).unwrap();
            assert_eq!(result,result2);assert_eq!(encode_snapshot(&restored,Some(&copy)),encode_snapshot(&state,Some(&vm)));
            if result==EventYield::Returned{break;}
        }
        assert_eq!(state.gil,3);
        let mut corrupt=bytes;corrupt[20]^=1;assert!(decode_snapshot(&code,&corrupt).is_err());
    }

    #[test]
    fn malformed_or_unknown_commands_stop_with_an_address() {
        let mut state = EventState::default();
        let error = EventVm::new(0).run_until_yield(&[0xf9], &mut state).unwrap_err();
        assert_eq!(error.pc, 0);
        assert!(error.message.contains("unsupported event opcode"));
        assert!(EventVm::new(0).run_until_yield(&[0x6b, 0], &mut state).is_err());
        // D5 has only one operand before the declared block ends. It must
        // not consume the following FE event return as a Y coordinate.
        assert!(EventVm::new(0).run_until_yield(&[0, 0x82, 0xd5, 4, 0xfe], &mut state).is_err());
    }
}
/// Trace bounded source-backed field scenes and the bit-gated return path.
pub fn capture_field_event_traces(code:&[u8],events:&crate::field_events::FieldEvents,directory:&std::path::Path)->std::io::Result<()> {
    use std::fs;fs::create_dir_all(directory)?;
    let mut rows=String::from("scenario,frame,event_offset,cpu,file_offset,scope,object,opcode,length\n");
    let mut actor_rows=String::from("scenario,object,enabled,visible,graphic,palette,character_party,active_party\n");
    let mut summaries=String::from("scenario,stage,frame,event_offset,terra_x,terra_y,wedge_x,wedge_y,vicks_x,vicks_y,npc1_x,npc1_y,npc2_x,npc2_y,party,dialogue,battle,flag_0128\n");
    for (scenario,entry,party_x,party_y,gated) in [("backtrack",0x029b1d,38,50,false),("guards",0x029b71,38,38,false),("gated",0x029b71,38,38,true)] {
        let mut state=EventState::opening_context();state.map_word=19;state.party=[0,14,15,255];state.party_x=party_x;state.party_y=party_y;
        state.actors[0].x=party_x;state.actors[0].y=party_y;state.actors[0].visible=true;state.gil=3000;
        state.set_switch(0x1cc,false);state.set_switch(0x1c1,true);state.set_switch(0x128,gated);events.init_npcs(19,&mut state);
        let mut vm=EventVm::scheduled(entry);let mut done=false;
        for _ in 0..36000 {
            let yielded=vm.step_frame(code,&mut state).map_err(|error|std::io::Error::other(error.to_string()))?;
            if yielded==EventYield::Pending {continue;}
            let (stage,dialogue,battle)=match yielded {
                EventYield::Dialogue {id,..}=>("dialogue",id.to_string(),String::new()),
                EventYield::Battle {group,..}=>("battle",String::new(),group.to_string()),
                EventYield::Returned=>("returned",String::new(),String::new()),EventYield::Pending=>unreachable!(),
            };
            summaries.push_str(&format!("{scenario},{stage},{},0x{:06X},{},{},{},{},{},{},{},{},{},{},{}:{}:{}:{},{dialogue},{battle},{}\n",vm.frame,vm.pc,
                state.actors[0].x,state.actors[0].y,state.actors[14].x,state.actors[14].y,state.actors[15].x,state.actors[15].y,
                state.actors[16].x,state.actors[16].y,state.actors[17].x,state.actors[17].y,
                state.party[0],state.party[1],state.party[2],state.party[3],u8::from(state.switch(0x128))));
            if matches!(yielded,EventYield::Battle {..}|EventYield::Returned) {done=true;break;}
        }
        if !done {return Err(std::io::Error::other("field script trace did not reach boundary"));}
        for id in [0usize,14,15,16,17] {
            let actor=state.actors[id];
            actor_rows.push_str(&format!("{scenario},{id},{},{},{},{},{},{}\n",u8::from(actor.enabled),u8::from(actor.visible),actor.graphic,actor.palette,
                if id<16 {state.char_parties[id]}else{0},state.active_party));
        }
        for entry in vm.trace {
            rows.push_str(&format!("{scenario},{},0x{:06X},0x{:06X},0x{:06X},{},{},0x{:02X},{}\n",entry.frame,entry.pc,
                entry.pc+0xca0000,entry.pc+0x0a0000,if entry.object.is_some(){"object"}else{"event"},
                entry.object.map(|id|id.to_string()).unwrap_or_default(),entry.opcode,entry.bytes));
        }
    }
    fs::write(directory.join("commands.csv"),rows)?;fs::write(directory.join("actors.csv"),actor_rows)?;fs::write(directory.join("snapshots.csv"),summaries)
}

// Versioned local VM snapshots. Diagnostic traces/effect history are not game state.
struct SnapshotReader<'a> {bytes:&'a [u8],pc:usize}
impl<'a> SnapshotReader<'a> {
    fn take(&mut self,n:usize)->Result<&'a [u8],String> {
        let end=self.pc.checked_add(n).ok_or("snapshot length overflow")?;
        let value=self.bytes.get(self.pc..end).ok_or("truncated event snapshot")?;self.pc=end;Ok(value)
    }
    fn byte(&mut self)->Result<u8,String>{Ok(self.take(1)?[0])}
    fn flag(&mut self)->Result<bool,String>{match self.byte()?{0=>Ok(false),1=>Ok(true),_=>Err("invalid snapshot boolean".into())}}
    fn word(&mut self)->Result<u16,String>{Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))}
    fn signed(&mut self)->Result<i16,String>{Ok(self.word()? as i16)}
    fn addr(&mut self)->Result<usize,String>{Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()) as usize)}
    fn frame(&mut self)->Result<u64,String>{Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))}
}
fn snapshot_checksum(bytes:&[u8])->u32 {bytes.iter().fold(0x811c9dc5u32,|hash,byte|(hash^u32::from(*byte)).wrapping_mul(0x01000193))}

pub fn encode_snapshot(state:&EventState,vm:Option<&EventVm>)->Vec<u8> {
    let mut out=b"F6EV3".to_vec();
    out.extend_from_slice(&state.map_word.to_le_bytes());
    out.extend_from_slice(&state.party_x.to_le_bytes());out.extend_from_slice(&state.party_y.to_le_bytes());
    out.extend_from_slice(&state.party);out.push(state.active_party);out.extend_from_slice(&state.char_parties);
    out.extend_from_slice(&state.gil.to_le_bytes());out.extend_from_slice(&state.switches);
    out.extend_from_slice(&[u8::from(state.camera_locked),u8::from(state.player_control)]);
    for actor in &state.actors {
        for value in [actor.x,actor.y,actor.offset_x,actor.offset_y] {out.extend_from_slice(&value.to_le_bytes());}
        out.extend_from_slice(&[actor.facing,actor.speed,u8::from(actor.visible),u8::from(actor.enabled),actor.graphic,actor.palette,
            u8::from(actor.collision_enabled),actor.vehicle,actor.pose]);
    }
    out.push(u8::from(vm.is_some()));
    if let Some(vm)=vm {
        out.extend_from_slice(&(vm.pc as u32).to_le_bytes());out.extend_from_slice(&vm.frame.to_le_bytes());
        out.extend_from_slice(&[u8::from(vm.finished),u8::from(vm.scheduled)]);out.extend_from_slice(&vm.pause.to_le_bytes());
        out.extend_from_slice(&[vm.awaiting.unwrap_or(255),u8::from(vm.sort_pending)]);
        out.push(vm.slots.len() as u8);out.extend(vm.slots.iter().map(|slot|*slot as u8));
        out.push(u8::from(vm.pending_battle.is_some()));
        if let Some((group,flags))=vm.pending_battle {out.extend_from_slice(&[group,flags]);}
        out.push(vm.calls.len() as u8);
        for frame in &vm.calls {
            out.extend_from_slice(&(frame.start as u32).to_le_bytes());out.extend_from_slice(&(frame.return_pc as u32).to_le_bytes());
            out.extend_from_slice(&frame.remaining.to_le_bytes());
        }
        for task in &vm.tasks {
            out.push(u8::from(task.is_some()));
            if let Some(task)=task {
                out.extend_from_slice(&(task.pc as u32).to_le_bytes());out.extend_from_slice(&(task.end as u32).to_le_bytes());
                out.extend_from_slice(&[task.wait,task.steps,task.direction,u8::from(task.moving)]);
            }
        }
    }
    out.extend_from_slice(&snapshot_checksum(&out).to_le_bytes());out
}

pub fn decode_snapshot(code:&[u8],bytes:&[u8])->Result<(EventState,Option<EventVm>),String> {
    if bytes.len()<9 || bytes.len()>16384 {return Err("invalid event snapshot size".into());}
    let end=bytes.len()-4;let checksum=u32::from_le_bytes(bytes[end..].try_into().unwrap());
    if snapshot_checksum(&bytes[..end])!=checksum {return Err("event snapshot checksum mismatch".into());}
    let mut read=SnapshotReader {bytes:&bytes[..end],pc:0};
    let magic=read.take(5)?;
    if magic!=b"F6EV1" && magic!=b"F6EV2" && magic!=b"F6EV3" {return Err("unsupported event snapshot version".into());}
    let mut state=EventState::default();state.map_word=read.word()?;state.party_x=read.signed()?;state.party_y=read.signed()?;
    state.party.copy_from_slice(read.take(4)?);
    if state.party.iter().any(|id|*id!=255 && *id>=16){return Err("invalid snapshot party".into());}
    if magic!=b"F6EV1" {
        state.active_party=read.byte()?;state.char_parties.copy_from_slice(read.take(16)?);
        if state.active_party>7 || state.char_parties.iter().any(|party|*party>7) {return Err("invalid snapshot character membership".into());}
    } else {state.char_parties[14]=1;state.char_parties[15]=1;}
    state.gil=read.addr()? as u32;state.switches.copy_from_slice(read.take(SWITCH_BYTES)?);
    state.camera_locked=read.flag()?;state.player_control=read.flag()?;
    for actor in &mut state.actors {
        actor.x=read.signed()?;actor.y=read.signed()?;actor.offset_x=read.signed()?;actor.offset_y=read.signed()?;
        actor.facing=read.byte()?;actor.speed=read.byte()?;actor.visible=read.flag()?;actor.enabled=read.flag()?;
        actor.graphic=read.byte()?;actor.palette=read.byte()?;actor.collision_enabled=read.flag()?;actor.vehicle=read.byte()?;actor.pose=read.byte()?;
        if actor.facing>3 || actor.speed>5 || actor.palette>7 || !(-4095..4096).contains(&actor.offset_x) || !(-4095..4096).contains(&actor.offset_y) {
            return Err("invalid snapshot actor".into());
        }
    }
    let vm=if read.flag()? {
        let pc=read.addr()?;let mut vm=EventVm::new(pc);vm.frame=read.frame()?;
        vm.finished=read.flag()?;vm.scheduled=read.flag()?;vm.pause=read.word()?;
        let awaiting=read.byte()?;vm.awaiting=if awaiting==255{None}else{Some(awaiting)};
        vm.sort_pending=read.flag()?;
        if magic==b"F6EV3" {
            let count=usize::from(read.byte()?);
            if count>24 {return Err("snapshot active object list exceeds capacity".into());}
            for id in read.take(count)? {
                let id=usize::from(*id);
                if id>0x30 || vm.slots.contains(&id) {return Err("invalid snapshot active object list".into());}
                vm.slots.push(id);
            }
        } else {vm.refresh_actor_order(&state);}
        if read.flag()? {vm.pending_battle=Some((read.byte()?,read.byte()?));}
        if pc>code.len() || vm.frame==u64::MAX || (awaiting!=255 && awaiting>0x30) {return Err("invalid snapshot VM cursor".into());}
        let calls=read.byte()?;
        if calls>16 {return Err("snapshot call stack exceeds capacity".into());}
        for _ in 0..calls {
            let frame=CallFrame {start:read.addr()?,return_pc:read.addr()?,remaining:read.word()?};
            if frame.start>=code.len() || frame.return_pc>code.len() || !(1..=256).contains(&frame.remaining) {return Err("invalid snapshot call frame".into());}
            vm.calls.push(frame);
        }
        for task in &mut vm.tasks {
            if read.flag()? {
                let value=ObjectTask {pc:read.addr()?,end:read.addr()?,wait:read.byte()?,steps:read.byte()?,direction:read.byte()?,moving:read.flag()?};
                if value.pc>=value.end || value.end>code.len() || value.steps>8 || value.direction>3 {return Err("invalid snapshot object cursor".into());}
                *task=Some(value);
            }
        }
        Some(vm)
    } else {None};
    if read.pc!=end {return Err("trailing event snapshot bytes".into());}
    Ok((state,vm))
}
