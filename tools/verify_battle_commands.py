#!/usr/bin/env python3
"""Compare native command storage and MP costs with bounded target instructions."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]

class CommandCpu(EquipmentCpu):
    def __init__(self,rom):
        super().__init__(rom,0,0,[255]*6);self.p=0x30;self.cost=0
        for i in range(128):self.write(0x7e3184+i,255)
        for i in range(128):
            self.write(0x7e3420+i*2,65535,16);self.write(0x7e3520+i*2,65535,16);self.write(0x7e3620+i*2,255)
        for slot in range(10):self.write(0x7e32cc+slot*2,255);self.write(0x7e32cd+slot*2,255)
        self.write(0x7e340a,255)
    def allowed(self):return 0x24e54<=self.pc<0x24f08 or 0x20301<=self.pc<0x2031c
    def hook(self,dest):
        if dest==0x4f08:self.a=(self.a&0xff00)|self.cost;return True
        return super().hook(dest)
    def result(self):return self.a&255
    def create(self,row):
        self.pc={0:0x24ecb,1:0x24eb2,2:0x24e91}[int(row['kind'])];self.p=0x30;self.x=int(row['actor'])*2;self.y=0;self.a=0;self.sp=0x1ff
        self.cost=int(row['cost']);self.write(0x7e3a7a,int(row['command']));self.write(0x7e3a7b,int(row['attack']));self.write(0xb8,int(row['targets']),16)
        if int(row['kind'])==2:self.a=int(row['command']);self.x=int(row['attack'])
        self.run()
    def hex(self,address,count):return bytes(self.read(0x7e0000+address+i) for i in range(count)).hex().upper()
    def heads(self,address):return bytes(self.read(0x7e0000+address+slot*2) for slot in range(10)).hex().upper()

class CostCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x24f08;self.p=0x30;self.x=int(row['actor'])
        self.write(0x7e3a7a,int(row['command']));self.write(0x7e3a7b,int(row['attack']));self.write(0xb1,int(row['flags']))
        for i in range(256):self.write(0x7e3084+i,255 if i%7==0 else i%64)
        for slot in range(4):
            ptr=0x5000+slot*0x100;self.write(0x7e302c+slot*2,ptr,16)
            for i in range(64):self.write(0x7e0000+ptr+i*4+3,(i*7+slot*11)%256)
    def allowed(self):return 0x24f08<=self.pc<0x24f57
    def result(self):return self.a&255,self.read(0xb1)

class InputCpu(CommandCpu):
    def __init__(self,rom,row):
        super().__init__(rom);self.pc=0x24d1f;self.p=0x30
        self.write(0x7e3a6a,int(row['cursor']));self.write(0xb1,int(row['flags']))
        for i,b in enumerate(bytes.fromhex(row['before'])):self.write(0x7e2bae+i,b)
        status1=bytes.fromhex(row['status1']);status2=bytes.fromhex(row['status2'])
        for slot in range(4):
            self.write(0x7e3aa0+slot*2,0x8b);self.write(0x7e3aa1+slot*2,255)
            self.write(0x7e3ee4+slot*2,status1[slot]);self.write(0x7e3ee5+slot*2,status2[slot])
        for i in range(256):self.write(0x7e3084+i,255 if i%7==0 else i%64)
        for slot in range(4):
            ptr=0x5000+slot*0x100;self.write(0x7e302c+slot*2,ptr,16)
            for i in range(64):self.write(0x7e0000+ptr+i*4+3,(i*7+slot*11)%256)
    def hook(self,dest):return EquipmentCpu.hook(self,dest)
    def allowed(self):return 0x24d1f<=self.pc<0x24f57 or 0x20792<=self.pc<0x20799 or 0x20a43<=self.pc<0x20a4a or 0x25217<=self.pc<0x2522a
    def result(self):
        flags=lambda base:bytes(self.read(0x7e0000+base+slot*2) for slot in range(4)).hex().upper()
        payloads=bytearray()
        for i in range(128):
            payloads.extend(self.read(0x7e3420+i*2,16).to_bytes(2,'little'));payloads.extend(self.read(0x7e3520+i*2,16).to_bytes(2,'little'));payloads.append(self.read(0x7e3620+i*2))
        return self.hex(0x2bae,32),self.read(0x7e3a6a),self.read(0xb1),self.hex(0x3184,128),self.heads(0x32cc),self.hex(0x3720,self.read(0x7e3a65)),flags(0x3aa0),flags(0x3aa1),payloads.hex().upper()

def read(path):
    with path.open(newline='') as stream:return list(csv.DictReader(stream))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    regions=[(0x24d1f,0x24daf),(0x24ddb,0x24f57),(0x20301,0x2031c)]
    for start,end in regions:assert rom[start:end]==clean[start:end]
    differences=[{'cpu':f'{i+0xc00000:06X}','file_offset':f'{i:06X}','clean':clean[i],'target':rom[i]} for i in range(0x24daf,0x24ddb) if clean[i]!=rom[i]]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-commands'
    subprocess.run([str(exe),'--capture-battle-commands',str(out)],cwd=ROOT,check=True)
    creations=read(out/'creation.csv');assert len(creations)==100;cpu=CommandCpu(rom);traces=[]
    for row in creations:
        cpu.steps=0;cpu.trace=[] if int(row['step']) in [0,30,99] else None;cpu.create(row)
        assert cpu.hex(0x3184,128)==row['links']
        assert cpu.heads(0x32cc)==row['normal'] and cpu.heads(0x32cd)==row['retaliation']
        assert cpu.read(0x7e340a)==int(row['immediate'])
        assert cpu.hex(0x3920,cpu.read(0x7e3a69))==row['ret_queue']
        offset=int(row['id'])*2
        assert (cpu.read(0x7e3420+offset),cpu.read(0x7e3421+offset),cpu.read(0x7e3520+offset,16),cpu.read(0x7e3620+offset))==(int(row['command']),int(row['attack']),int(row['targets']),int(row['cost']))
        if cpu.trace is not None:traces.append({'step':int(row['step']),'instructions':cpu.trace})
    removals=read(out/'removal.csv');assert len(removals)==64
    for row in removals:
        slot=int(row['actor']);head=cpu.read(0x7e32cc+slot*2)
        before='' if head&128 else f'{cpu.read(0x7e3420+head*2):02X}{cpu.read(0x7e3421+head*2):02X}{cpu.read(0x7e3520+head*2,16):04X}{cpu.read(0x7e3620+head*2):02X}'
        cpu.pc=0x20301;cpu.p=0x30;cpu.x=slot*2;cpu.y=0;cpu.sp=0x1ff;cpu.trace=None;cpu.run()
        assert before==row['removed'] and cpu.hex(0x3184,128)==row['links'] and cpu.heads(0x32cc)==row['normal']
    allocations=read(out/'allocation.csv');assert len(allocations)==140;cpu=CommandCpu(rom)
    for row in allocations:
        cpu.pc=0x24e54;cpu.p=0x30;cpu.x=0;cpu.sp=0x1ff
        assert cpu.run()==int(row['id'])
    costs=read(out/'mp-costs.csv');assert len(costs)==20480;instructions=0
    for row in costs:
        cpu=CostCpu(rom,row);result=cpu.run();instructions+=cpu.steps
        assert result==(int(row['cost']),int(row['after_flags'])),(row,result)
    inputs=read(out/'player-inputs.csv');assert len(inputs)==256;input_steps=0
    for row in inputs:
        cpu=InputCpu(rom,row);actual=cpu.run();input_steps+=cpu.steps
        expected=(row['after'],int(row['after_cursor']),int(row['after_flags']),row['links'],row['normal'],row['advance'],row['flags0'],row['flags1'],row['payloads'])
        assert actual==expected,(row['case'],actual,expected)
    (out/'target-command-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'creation_cases':len(creations),'removal_cases':len(removals),'allocation_cases':len(allocations),'mp_cost_cases':len(costs),'mp_cost_target_instructions':instructions,'player_input_cases':len(inputs),'player_input_target_instructions':input_steps,'unsupported_item_slot_normalization_changes':differences,'reference_kind':'bounded target command-list/MP-cost/player-input execution; standalone creation cost injected, modified spell lists supplied, no live battle execution','result':'pass'}
    (ROOT/'reverse-engineering/battle-commands-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
