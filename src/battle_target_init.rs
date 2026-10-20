//! InitTarget C2/26D3..2777 and LoadMagicProp C2/2966..2989.
use crate::{damage::AttackTable,equipment::ItemTable};
use std::{fs,path::Path};
#[derive(Clone)]
pub struct Initializer {commands:[u8;64],modes:[u8;30],tool_items:[u8;5],tool_offsets:[u8;5],attacks:AttackTable,items:ItemTable}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct State {pub restrictions:u8,pub cursor:u8,pub flags_b1:u8,pub last_spell:u8,pub last_item:u8,pub magic:[u8;14]}
impl Initializer {
    pub fn load(root:&Path)->Result<Self,String> {
        let dir=root.join("build/native_battle");
        let read=|name:&str|fs::read(dir.join(format!("{name}.bin"))).map_err(|e|e.to_string());
        Ok(Self {commands:read("command_properties")?.try_into().map_err(|_|"Command properties require 64 bytes")?,modes:read("command_target_modes")?.try_into().map_err(|_|"Command target modes require 30 bytes")?,tool_items:read("throw_tool_items")?.try_into().map_err(|_|"Throw/tool items require five bytes")?,tool_offsets:read("throw_tool_offsets")?.try_into().map_err(|_|"Throw/tool offsets require five bytes")?,attacks:AttackTable::load(root)?,items:ItemTable::load(root)?})
    }
    fn load_magic(&self,id:u8,s:&mut State) {
        s.magic.copy_from_slice(self.attacks.record(id));
        s.magic[9]=if s.magic[9]&0x80!=0 {0}else{s.magic[9]<<1};
        if s.cursor==255 {s.cursor=s.magic[0];}
        s.restrictions|=((s.magic[2]&4)<<1)|((s.magic[2]&0x10)<<2)|(s.magic[2]&0x80);
    }
    fn item(&self,id:u8,s:&mut State)->u8 {
        s.last_item=id;let r=self.items.record(id);s.cursor=r[14];
        if r[21]&0xc2==0 {s.restrictions&=!8;}
        r[18]
    }
    pub fn initialize(&self,command:u8,attack:u8,s:&mut State) {
        s.restrictions=0;s.cursor=0x40;
        let mode=if command<30 {
            let mode=self.modes[usize::from(command)];s.restrictions=(mode&0xe1)|((mode&0x18)>>1);s.cursor=self.commands[usize::from(command)*2+1];mode&6
        }else{0};
        let spell=match mode {
            0=>238,
            4=>{s.last_spell=attack;attack},
            2=>{
                let value=self.item(attack,s);
                if attack>=230 {return;}
                if value&0x80==0 {s.flags_b1|=0x10;}
                let id=value&0x3f;s.last_spell=id;id
            },
            6=>{
                if let Some(index)=self.tool_items.iter().rposition(|v|*v==attack) {
                    let id=attack.wrapping_sub(self.tool_offsets[index]);s.last_spell=id;id
                }else{self.item(attack,s);return;}
            },
            _=>unreachable!("two-bit target mode"),
        };
        self.load_magic(spell,s);
    }
}
pub fn capture(root:&Path,directory:&Path)->Result<(),String> {
    use std::fmt::Write;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;let init=Initializer::load(root)?;
    let mut rows=String::from("command,attack,restrictions,cursor,flags,last_spell,last_item,magic\n");
    for command in 0..=255 {for attack in 0..=255 {
        let mut s=State {restrictions:255,cursor:255,flags_b1:0x61,last_spell:0x55,last_item:0x66,magic:[0xaa;14]};init.initialize(command,attack,&mut s);
        writeln!(rows,"{command},{attack},{},{},{},{},{},{}",s.restrictions,s.cursor,s.flags_b1,s.last_spell,s.last_item,s.magic.iter().map(|v|format!("{v:02X}")).collect::<String>()).unwrap();
    }}
    fs::write(directory.join("initialization.csv"),rows).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests {
    use super::*;
    fn state()->State {State {restrictions:255,cursor:255,flags_b1:0,last_spell:85,last_item:102,magic:[170;14]}}
    #[test]fn fight_ignores_selected_special_and_internal_commands_use_default_attack() {
        let init=Initializer::load(Path::new("extracted/twue-3.07")).unwrap();let mut s=state();init.initialize(0,239,&mut s);
        assert_eq!((s.restrictions,s.cursor,s.last_spell,s.last_item),(0x20,0x41,85,102));
        let mut other=state();init.initialize(31,131,&mut other);assert_eq!((other.restrictions,other.cursor),(0,0x40));assert_eq!(s.magic,other.magic);
    }
    #[test]fn magitek_loads_attack_properties_and_records_spell() {
        let init=Initializer::load(Path::new("extracted/twue-3.07")).unwrap();let mut s=state();init.initialize(29,131,&mut s);
        assert_eq!((s.cursor,s.last_spell,s.magic[6],s.magic[9]),(0x43,131,60,0));
    }
}
