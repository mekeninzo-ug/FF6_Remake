#!/usr/bin/env python3
"""Bounded actual Guard ExecAI; intercepts action preparation and storage."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row,mapping=False):
        super().__init__(rom,0,0,[255]*6);self.mapping=mapping;self.actions=[]
        if mapping:self.pc=0x21dbf;self.p=0x30;self.a=int(row['attack']);self.x=18;return
        self.pc=0x21a2f;self.p=0x10;self.x=12
        for address,key in [(0xf2,'resume'),(0xf4,'previous_block'),(0xb8,'targets')]:self.write(address,int(row[key]),16 if address!=0xf4 else 8)
        self.write(0xbe,int(row['seed']));self.write(0xf0,0,16);self.write(0x7e3ee4+12,0x2000 if int(row['confused']) else 0,16)
        self.targeted='after_duration' in row
        if self.targeted:
            for a,v,w in [(0x3a78,0x3f0f,16),(0x3408,65535,16),(0x3403,15,8),(0x2f46,63,8),(0x3a82,255,8),(0x3a83,255,8)]:self.write(0x7e0000+a,v,w)
            for slot in range(10):
                self.write(0x7e3018+slot*2,1<<(slot if slot<4 else slot+4),16)
                self.write(0x7e3aa0+slot*2,int(slot in [0,1,2,6,7]));self.write(0x7e3395+slot*2,255)
    def read(self,address,width=8):
        if hasattr(self,'fixture') and 0xcf8700<=address<0xcf8700+len(self.fixture):
            i=address-0xcf8700;return self.fixture[i] if width==8 else self.fixture[i]|(self.read(address+1)<<8)
        return super().read(address,width)
    def allowed(self):return 0x21a2f<=self.pc<0x21b47 or 0x21dbf<=self.pc<0x21dd8 or 0x24b53<=self.pc<0x24b7b or 0x226d3<=self.pc<0x22778 or 0x22966<=self.pc<0x2298a or 0x22b63<=self.pc<0x22b69 or 0x203b9<=self.pc<0x20420 or 0x2587e<=self.pc<0x25a83 or 0x251f0<=self.pc<0x25269
    def hook(self,dest):
        if getattr(self,'targeted',False) and dest==0x03e4:return False
        if dest==0x03e4:self.write(0x7e3a7a,self.a,16);return True
        if dest==0x4ead:self.actions.append(f'{self.read(0x7e3a7a)}:{self.read(0x7e3a7b)}:{self.read(0xb8,16)}');return True
        return super().hook(dest)
    def result(self):
        if self.mapping:return self.a&255
        assert self.x==12
        result=(self.read(0xf5),self.read(0xf2,16),self.read(0xbe),'|'.join(self.actions))
        return result+(self.read(0x7e322c+12),) if self.targeted else result
def rows(path):
    with path.open(newline='') as f:return list(csv.DictReader(f))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-ai-turn'
    subprocess.run([str(exe),'--capture-battle-ai-turn',str(out)],cwd=ROOT,check=True)
    mapping=rows(out/'attack-command.csv');turns=rows(out/'guard-turn.csv');assert len(mapping)==256 and len(turns)==1024
    fixtures=rows(out/'fixture-turn.csv');assert len(fixtures)==2048
    targeted=rows(out/'targeted-turn.csv');assert len(targeted)==1024
    steps=0;traces=[]
    for row in mapping:
        cpu=Cpu(rom,row,True);actual=cpu.run();steps+=cpu.steps;assert actual==int(row['command']),(row,actual)
    for i,row in enumerate(turns+fixtures+targeted):
        cpu=Cpu(rom,row)
        if 'script' in row:cpu.fixture=bytes.fromhex(row['script'])
        if i in [0,100,600,1025,1600]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=(int(row['after_block']),int(row['after_resume']),int(row['index']),row['actions'])
        if 'after_duration' in row:expected+=(int(row['after_duration']),)
        assert actual==expected,(row,actual)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'command_mapping_cases':len(mapping),'guard_turn_cases':len(turns),'controlled_script_cases':len(fixtures),'targeted_script_cases':len(targeted),'target_instructions':steps,'reference_kind':'actual target ExecAI/GetCmdForAI/RNG with Guard and controlled scripts; emission cases intercept delay/storage, targeted cases execute CalcCmdDelay/InitTarget/ChooseTarget with final storage boundary modeled; no full battle','result':'pass'}
    (ROOT/'reverse-engineering/battle-ai-turn-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
