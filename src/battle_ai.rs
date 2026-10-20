//! Initial AI attack selection from CF/8400 pointers and CF/8700 scripts.
//! This is not the full conditional/retaliation/queue interpreter yet.
use std::{fs,path::Path};
use crate::battle_rng::{BattleRng,RngTable};
#[derive(Clone)] pub struct AiTable {pointers:Vec<u16>,script:Vec<u8>,sizes:[u8;16],thresholds:[u8;11],commands:[u8;11]}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct AiAction {pub command:u8,pub attack:u8,pub targets:u16}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct AiLoop {pub resume:u16,pub block:u8}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ScriptEnd {pub terminator:u8,pub next:u16}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ScriptPointers {pub normal:u16,pub retaliation:u16,pub normal_end:u16}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AiChoice {Attack(u8),Continue,End}
impl AiTable {
    pub fn load(root:&Path)->Result<Self,String> {
        let dir=root.join("build/native_battle");
        let pointers=fs::read(dir.join("ai_pointers.bin")).map_err(|e|e.to_string())?;
        let script=fs::read(dir.join("ai_script.bin")).map_err(|e|e.to_string())?;
        if pointers.len()!=768 || script.len()!=0x3950 {return Err("Invalid AI table lengths".into());}
        let pointers=pointers.chunks_exact(2).map(|v|u16::from_le_bytes([v[0],v[1]])).collect::<Vec<_>>();
        if pointers.iter().any(|v|usize::from(*v)>=script.len()) {return Err("AI pointer outside script region".into());}
        let sizes:[u8;16]=fs::read(dir.join("ai_command_sizes.bin")).map_err(|e|e.to_string())?.try_into().map_err(|_|"AI command sizes must contain sixteen bytes")?;
        if sizes.iter().any(|size|*size==0) {return Err("AI command sizes cannot be zero".into());}
        let thresholds=fs::read(dir.join("ai_attack_thresholds.bin")).map_err(|e|e.to_string())?.try_into().map_err(|_|"AI thresholds require eleven bytes")?;
        let commands=fs::read(dir.join("ai_attack_commands.bin")).map_err(|e|e.to_string())?.try_into().map_err(|_|"AI commands require eleven bytes")?;
        Ok(Self {pointers,script,sizes,thresholds,commands})
    }
    pub fn command_for_attack(&self,attack:u8)->u8 {
        self.thresholds.iter().rposition(|first|attack>=*first).map(|i|self.commands[i]).unwrap_or(2)
    }
    /// Bounded normal AI execution for attack, random attack/command, wait and
    /// end markers. Other opcodes fail explicitly before invoking their effects.
    /// The caller handles target normalization, delay and command-list emission.
    pub fn run_normal(&self,monster:u16,state:&mut AiLoop,targets:u16,confused:bool,rng:&mut BattleRng,table:&RngTable,mut emit:impl FnMut(AiAction,&mut BattleRng)->Result<(),String>)->Result<(),String> {
        let mut ptr=*self.pointers.get(usize::from(monster)).ok_or("AI monster index outside table")?;
        let block=0u8;let mut previous=state.block;
        for _ in 0..16384 {
            let read=|p:u16|self.script.get(usize::from(p)).copied().ok_or_else(||format!("AI execution outside script at {p:04X}"));
            let mut opcode=read(ptr)?;
            if opcode<0xfc && block==previous {previous=255;ptr=state.resume;opcode=read(ptr)?;}
            let size=if opcode<0xf0 {1}else{self.sizes[usize::from(opcode-0xf0)]};
            let end=ptr.checked_add(u16::from(size)).ok_or("AI execution pointer overflow")?;
            if usize::from(end)>self.script.len() {return Err("Truncated AI command".into());}
            match opcode {
                0xfd=>{state.resume=end;state.block=block;return Ok(());},
                0xfe|0xff=>{state.block=255;return Ok(());},
                0..=0xef|0xf0|0xf4=> {
                    let selected=if opcode==0xf0 || opcode==0xf4 {read(ptr+1+u16::from(rng.below(table,3)))?}else{opcode};
                    if !(opcode>=0xf0 && selected==0xfe) {
                        let (mut command,attack)=if opcode==0xf4 {(selected,0)}else{(self.command_for_attack(selected),selected)};
                        if command==12 && attack==140 {command=30;}
                        emit(AiAction {command,attack,targets:if confused {0}else{targets}},rng)?;
                    }
                },
                other=>return Err(format!("AI opcode {other:02X} at CF/{:04X} needs unimplemented effects/condition evaluation",0x8700+usize::from(ptr))),
            }
            ptr=end;
        }
        Err("AI turn exceeded bounded command budget".into())
    }
    /// C2/1A43 skips whole commands; FE/FF bytes inside operands do not end a block.
    pub fn find_end(&self,start:u16)->Result<ScriptEnd,String> {
        let mut ptr=usize::from(start);
        loop {
            let opcode=*self.script.get(ptr).ok_or_else(||format!("AI scan outside extracted script at {ptr:04X}"))?;
            ptr+=1;
            if opcode>=0xfe {return Ok(ScriptEnd {terminator:opcode,next:u16::try_from(ptr).map_err(|_|"AI scan exceeds pointer width")?});}
            if opcode>=0xf0 {ptr+=usize::from(self.sizes[usize::from(opcode-0xf0)])-1;}
        }
    }
    /// C2/2D71 preserves the existing retaliation pointer if no script follows.
    pub fn initialize(&self,monster:u16,previous_retaliation:u16)->Result<ScriptPointers,String> {
        let normal=*self.pointers.get(usize::from(monster)).ok_or("AI monster index outside table")?;
        let mut ptr=normal;
        loop {let end=self.find_end(ptr)?;ptr=end.next;if end.terminator==255 {break;}}
        let next=*self.script.get(usize::from(ptr)).ok_or("AI retaliation probe outside extracted script")?;
        Ok(ScriptPointers {normal,normal_end:ptr,retaliation:if next==255 {previous_retaliation}else{ptr}})
    }
    pub fn first_choice(&self,monster:u16,rng:&mut BattleRng,table:&RngTable)->Result<AiChoice,String> {
        let ptr=usize::from(*self.pointers.get(usize::from(monster)).ok_or("AI monster index outside table")?);
        match self.script[ptr] {
            0..=0xef => Ok(AiChoice::Attack(self.script[ptr])),
            0xf0 => {
                let choices=self.script.get(ptr+1..ptr+4).ok_or("Truncated AI attack triple")?;
                let selected=choices[usize::from(rng.below(table,3))];
                Ok(if selected==0xfe {AiChoice::Continue} else {AiChoice::Attack(selected)})
            },
            0xff=>Ok(AiChoice::End),
            opcode=>Err(format!("AI opcode {opcode:02X} at CF/{:04X} is not implemented",0x8700+ptr)),
        }
    }
}
pub fn capture_turn(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let ai=AiTable::load(root)?;let table=RngTable::load(root)?;
    let mut mapping=String::from("attack,command\n");for attack in 0..=255 {writeln!(mapping,"{attack},{}",ai.command_for_attack(attack)).unwrap();}
    fs::write(directory.join("attack-command.csv"),mapping).map_err(|e|e.to_string())?;
    let mut rows=String::from("case,seed,previous_block,resume,targets,confused,after_block,after_resume,index,actions\n");
    for case in 0..1024u16 {
        let seed=case as u8;let previous_block=if case&256==0 {255}else{0};let confused=case&512!=0;
        let mut state=AiLoop {resume:0,block:previous_block};let mut rng=BattleRng {index:seed};let mut actions=Vec::new();
        ai.run_normal(0,&mut state,0x000f,confused,&mut rng,&table,|a,_| {actions.push(format!("{}:{}:{}",a.command,a.attack,a.targets));Ok(())})?;
        writeln!(rows,"{case},{seed},{previous_block},0,15,{},{},{},{},{}",confused as u8,state.block,state.resume,rng.index,actions.join("|")).unwrap();
    }
    fs::write(directory.join("guard-turn.csv"),rows).map_err(|e|e.to_string())?;
    let fixtures:Vec<Vec<u8>>=vec![vec![238,253,239,255],vec![240,254,238,239,253,244,0,2,254,255],vec![244,2,29,254,255],vec![140,255],vec![254,255],vec![240,254,254,254,255],vec![238,239,255],vec![244,255,0,2,253,238,255]];
    let mut rows=String::from("case,script,seed,previous_block,resume,targets,confused,after_block,after_resume,index,actions\n");
    for (id,script) in fixtures.iter().enumerate() {for seed in 0..=255u8 {
        let mut custom=ai.clone();custom.pointers=vec![0];custom.script=script.clone();custom.script.extend([255;4]);
        let resume=if id==0 && seed&1!=0 {2}else{0};let previous_block=if seed&2!=0 {0}else{255};let confused=seed&4!=0;
        let mut state=AiLoop {resume,block:previous_block};let mut rng=BattleRng {index:seed};let mut actions=Vec::new();
        custom.run_normal(0,&mut state,15,confused,&mut rng,&table,|a,_| {actions.push(format!("{}:{}:{}",a.command,a.attack,a.targets));Ok(())})?;
        writeln!(rows,"{},{},{seed},{previous_block},{resume},15,{},{},{},{},{}",id*256+usize::from(seed),custom.script.iter().map(|v|format!("{v:02X}")).collect::<String>(),confused as u8,state.block,state.resume,rng.index,actions.join("|")).unwrap();
    }}
    fs::write(directory.join("fixture-turn.csv"),rows).map_err(|e|e.to_string())?;
    let delays=crate::battle_scheduler::CommandDelays::load(root)?;
    let initializer=crate::battle_target_init::Initializer::load(root)?;
    let mut rows=String::from("case,script,seed,previous_block,resume,targets,confused,after_block,after_resume,index,actions,after_duration\n");
    for case in 0..1024usize {
        let mut custom=ai.clone();custom.pointers=vec![0];custom.script=if case&256==0 {vec![240,238,238,239,255,255,255,255]}else{vec![240,238,238,239,240,238,238,239,255,255,255,255]};
        use crate::battle_targets::{self,Actor,Context,Selection};
        let mut c=Context {actor:6,restrictions:0,cursor:0,actors:std::array::from_fn(|i|Actor {present:[0,1,2,6,7].contains(&i),status12:if i==6 && case&512!=0 {0x2000}else{0},status34:0,charm_attacker:255}),targettable:0x3f0f,valid:0x3f0f,revived:65535,jump_seize:0,remove_jump:false,enemy_characters:0,battle_type:0,pincer_left:0x15,pincer_right:0x2a,previous_attacker:0,backup:0};
        let seed=case as u8;let confused=case&512!=0;let mut state=AiLoop {resume:0,block:255};let mut rng=BattleRng {index:seed};let mut actions=Vec::new();let mut duration=0;
        let mut target_state=crate::battle_target_init::State {restrictions:0,cursor:0,flags_b1:0,last_spell:0,last_item:0,magic:[0;14]};
        custom.run_normal(0,&mut state,0,confused,&mut rng,&table,|a,rng| {
            initializer.initialize(a.command,a.attack,&mut target_state);c.restrictions=target_state.restrictions&!4;c.cursor=target_state.cursor;
            duration=delays.add(duration,a.command);let mut selection=Selection {targets:a.targets,mask_flags:0};battle_targets::choose(&c,&mut selection,rng,&table)?;
            actions.push(format!("{}:{}:{}",a.command,a.attack,selection.targets));Ok(())
        })?;
        writeln!(rows,"{case},{},{seed},255,0,0,{},{},{},{},{},{duration}",custom.script.iter().map(|v|format!("{v:02X}")).collect::<String>(),confused as u8,state.block,state.resume,rng.index,actions.join("|")).unwrap();
    }
    fs::write(directory.join("targeted-turn.csv"),rows).map_err(|e|e.to_string())
}
pub fn capture_init(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let ai=AiTable::load(root)?;
    let mut rows=String::from("case,monster,previous,normal,retaliation,normal_end\n");
    for monster in 0..384 {for previous in [65535,0x1234] {
        let p=ai.initialize(monster,previous)?;
        writeln!(rows,"{},{monster},{previous},{},{},{}",monster*2+u16::from(previous==0x1234),p.normal,p.retaliation,p.normal_end).unwrap();
    }}
    fs::write(directory.join("initialization.csv"),rows).map_err(|e|e.to_string())?;
    let mut rows=String::from("monster,start,terminator,next\n");
    for monster in 0..384 {let mut ptr=ai.pointers[monster];loop {
        let end=ai.find_end(ptr)?;writeln!(rows,"{monster},{ptr},{},{}",end.terminator,end.next).unwrap();ptr=end.next;if end.terminator==255 {break;}
    }}
    fs::write(directory.join("normal-blocks.csv"),rows).map_err(|e|e.to_string())
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    let table=RngTable::load(root)?;let ai=AiTable::load(root)?;
    let mut rows=String::from("seed,attack,index\n");
    for seed in 0..=255 {let mut rng=BattleRng {index:seed};
        let AiChoice::Attack(attack)=ai.first_choice(0,&mut rng,&table)? else {return Err("Guard AI did not produce an attack".into());};
        writeln!(rows,"{seed},{attack},{}",rng.index).unwrap();
    }
    fs::write(directory.join("guard-ai.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn single_attack_end_and_unknown_commands_do_not_consume_rng() {
        let table=AiTable {pointers:vec![0,1,2],script:vec![0xee,0xff,0xfc],sizes:[1;16],thresholds:[255;11],commands:[2;11]};
        let rng_table=RngTable::identity();
        let mut rng=BattleRng {index:9};
        assert_eq!(table.first_choice(0,&mut rng,&rng_table),Ok(AiChoice::Attack(0xee)));
        assert_eq!(table.first_choice(1,&mut rng,&rng_table),Ok(AiChoice::End));
        assert!(table.first_choice(2,&mut rng,&rng_table).is_err());assert_eq!(rng.index,9);
    }
    #[test]fn operands_can_contain_end_bytes_and_empty_retaliation_preserves_pointer() {
        let sizes=[4,2,4,3,4,4,4,2,3,4,4,3,4,1,1,1];
        let table=AiTable {pointers:vec![0],script:vec![0xf0,0xff,0xfe,0xee,0xfe,0xf1,0xff,0xff,0xff],sizes,thresholds:[255;11],commands:[2;11]};
        assert_eq!(table.find_end(0).unwrap(),ScriptEnd {terminator:254,next:5});
        assert_eq!(table.initialize(0,0x1234).unwrap(),ScriptPointers {normal:0,retaliation:0x1234,normal_end:8});
    }
    #[test]fn retaliation_starts_after_normal_end_without_rng_consumption() {
        let table=AiTable {pointers:vec![0],script:vec![0xee,0xff,0xef,0xff],sizes:[1;16],thresholds:[255;11],commands:[2;11]};
        assert_eq!(table.initialize(0,65535).unwrap(),ScriptPointers {normal:0,retaliation:2,normal_end:2});
        assert!(table.find_end(3).is_ok());assert!(table.find_end(4).is_err());
    }
    #[test]fn guard_turn_builds_a_native_normal_command_with_cached_cost() {
        use crate::{battle_commands::{CommandLists,MpCosts,Payload},battle_scheduler::CommandDelays};
        let root=Path::new("extracted/twue-3.07");let ai=AiTable::load(root).unwrap();let table=RngTable::load(root).unwrap();
        let costs=MpCosts::from_attacks(&crate::damage::AttackTable::load(root).unwrap());let delays=CommandDelays::load(root).unwrap();
        let mut pool=CommandLists::default();let mut duration=0;let mut flags=0;let mut rng=BattleRng::default();let mut state=AiLoop {resume:0,block:255};
        ai.run_normal(0,&mut state,15,false,&mut rng,&table,|a,_| {
            duration=delays.add(duration,a.command);
            let mp_cost=costs.get(a.command,a.attack,12,&mut flags)?;
            pool.normal(6,Payload {command:a.command,attack:a.attack,targets:a.targets,mp_cost})?;Ok(())
        }).unwrap();
        let p=pool.payloads[usize::from(pool.normal[6])];assert_eq!((p.command,p.targets,p.mp_cost),(0,15,0));assert!([238,239].contains(&p.attack));assert_eq!(duration,16);assert_eq!((rng.index,state.block),(1,255));
    }
    #[test]fn wait_resumes_after_prior_actions_without_repeating_them() {
        let root=Path::new("extracted/twue-3.07");let mut ai=AiTable::load(root).unwrap();ai.pointers=vec![0];ai.script=vec![238,253,239,255,255,255];
        let mut state=AiLoop {resume:0,block:255};let mut rng=BattleRng::default();let table=RngTable::identity();let mut attacks=Vec::new();
        ai.run_normal(0,&mut state,15,false,&mut rng,&table,|a,_| {attacks.push(a.attack);Ok(())}).unwrap();assert_eq!(state,AiLoop {resume:2,block:0});
        ai.run_normal(0,&mut state,15,false,&mut rng,&table,|a,_| {attacks.push(a.attack);Ok(())}).unwrap();assert_eq!(attacks,vec![238,239]);assert_eq!(state.block,255);assert_eq!(rng.index,0);
    }
}
