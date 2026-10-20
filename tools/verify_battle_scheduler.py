#!/usr/bin/env python3
"""Bounded target ATB and dispatch comparisons with explicit external callbacks."""
from pathlib import Path
import csv,hashlib,json,subprocess,sys
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]

def unpack_actor(data):
    return [data[i:i+12] for i in range(0,len(data),12)]
class TimingCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x2111b;self.p=0x20;self.events=[]
        self.write(0x0e,int(row['frame']));self.write(0xb1,int(row['menu_flags']))
        for address,key in [(0x3a6c,'previous'),(0x2f41,'stop'),(0x3a8f,'wait'),(0x2f45,'running'),(0x3f2c,'jumping'),(0x3a40,'enemy_chars'),(0x33f8,'zinger'),(0x3ee2,'morph')]:self.write(0x7e0000+address,int(row[key]))
        # Zinger attacker is mapped from the target CPX operand below.
        assert rom[0x21151:0x21154]==bytes.fromhex('ec f8 33')
        address=int.from_bytes(rom[0x21152:0x21154],'little')
        self.write(0x7e0000+address,int(row['zinger']))
        self.write(0x7e3a3e,int(row['turns']),16)
        self.before=unpack_actor(bytes.fromhex(row['before']))
        for slot,b in enumerate(self.before):
            x=slot*2
            for address,v in [(0x3aa0,b[0]),(0x3aa1,b[1]),(0x3ee4,b[2]),(0x3205,b[3]),(0x3e4d,b[4]),(0x322c,b[11])]:self.write(0x7e0000+address+x,v)
            for address,offset in [(0x3218,5),(0x3ac8,7),(0x3ab4,9)]:self.write(0x7e0000+address+x,int.from_bytes(b[offset:offset+2],'little'),16)
            self.write(0x7e3018+x,(1<<slot) if slot<4 else (1<<(slot+4)),16)
    def allowed(self):return 0x2111b<=self.pc<0x21211 or 0x24e66<=self.pc<0x24e84 or 0x20a43<=self.pc<0x20a4a
    def hook(self,dest):
        if dest==0x4d1f:return True # GetPlayerActions: input-side state supplied by fixture
        if dest==0x5a83:self.events.append('C');return True # external status counters
        if dest==0x1211:self.events.append(f'M{self.x}');return True
        if dest==0x5be2:self.events.append(f'R{self.x}');return True # no escaped actors in fixtures
        if dest==0x5c54:self.events.append('G');return True # no PPU writes
        if dest==0x6411:
            assert self.a&255==2
            self.events.append(f'O{self.read(0x10)}');return True
        return super().hook(dest)
    def result(self):
        result=bytearray()
        for slot in range(10):
            x=slot*2
            for address in [0x3aa0,0x3aa1,0x3ee4,0x3205,0x3e4d]:result.append(self.read(0x7e0000+address+x))
            for address in [0x3218,0x3ac8,0x3ab4]:result.extend(self.read(0x7e0000+address+x,16).to_bytes(2,'little'))
            result.append(self.read(0x7e322c+x))
        queue=lambda base,end:bytes(self.read(base+i) for i in range(self.read(end))).hex().upper()
        return result.hex().upper(),self.read(0x7e3a6c),self.read(0x7e3a3e,16),queue(0x7e3720,0x7e3a65),queue(0x7e3820,0x7e3a67),'|'.join(self.events)

class DispatchCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x20033;self.p=0x30;self.work=''
        for address,key in [(0x340a,'immediate'),(0x3407,'retaliation_active'),(0x3406,'action_active')]:self.write(0x7e0000+address,int(row[key]))
        tails=bytes.fromhex(row['tail'])
        for key,base,ptr,tail in [('retaliation',0x3920,0x3a68,tails[0]),('advance',0x3720,0x3a64,tails[1]),('action',0x3820,0x3a66,tails[2])]:
            self.write(0x7e0000+ptr,int(row['head']));self.write(0x7e0000+ptr+1,tail)
            for i,b in enumerate(bytes.fromhex(row[key])):self.write(0x7e0000+base+i,b)
    def allowed(self):return 0x20033<=self.pc<0x200c5
    def hook(self,dest):
        if dest==0x47ed:return True # battle-end check kept nonterminal for selector fixtures
        kind={0x2163:'I',0x4b7b:'R',0x2188:'W',0x00f9:'A',0x0019:''}.get(dest)
        if kind is not None:
            self.work=kind+str(self.read(0x7e340a) if kind=='I' else self.x) if kind else ''
            return True
        return super().hook(dest)
    def result(self):return self.work,self.read(0x7e3a68),self.read(0x7e3a64),self.read(0x7e3a66)

class DelayCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x203e4;self.p=0x30;self.a=int(row['command']);self.stop_pc=0x2040a
        self.write(0x7e322c,int(row['before']))
    def allowed(self):return 0x203e4<=self.pc<0x20420
    def result(self):return self.read(0x7e322c)

def read(path):
    with path.open(newline='') as stream:return list(csv.DictReader(stream))
def main():
    subprocess.run([sys.executable,str(ROOT/'tools/extract_native_battle.py')],cwd=ROOT,check=True,capture_output=True)
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    regions=[(0x2111b,0x21211),(0x24e66,0x24e91),(0x20033,0x200c5),(0x203e4,0x20420),(0x2067b,0x2069b),(0x20a43,0x20a4a)]
    for start,end in regions:assert rom[start:end]==clean[start:end]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-scheduler'
    subprocess.run([str(exe),'--capture-battle-scheduler',str(out)],cwd=ROOT,check=True)
    timing=read(out/'timing.csv');assert len(timing)==1024;steps=0;traces=[]
    for index,row in enumerate(timing):
        cpu=TimingCpu(rom,row)
        if index in [2,3,4,999]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=(row['after'],int(row['clock']),int(row['after_turns']),row['advance'],row['action'],row['events'])
        assert actual==expected,(row['case'],actual,expected)
        if cpu.trace is not None:traces.append({'case':index,'instructions':cpu.trace})
    dispatch=read(out/'dispatch.csv');assert len(dispatch)==512
    for row in dispatch:
        cpu=DispatchCpu(rom,row);actual=cpu.run();steps+=cpu.steps
        assert actual==(row['work'],int(row['ret_head']),int(row['advance_head']),int(row['action_head'])),(row,actual)
    delays=read(out/'command-delays.csv');assert len(delays)==1792
    for row in delays:
        cpu=DelayCpu(rom,row);actual=cpu.run();steps+=cpu.steps
        assert actual==int(row['after']),(row,actual)
    rings=read(out/'ring-operations.csv');assert len(rings)==1536
    for start in range(0,len(rings),6):
        row=rings[start];head=int(row['head']);tail=int(row['tail']);data=[255]*256
        for i,value in enumerate([0,254,4]):data[(head+i)&255]=value
        for row in rings[start:start+6]:
            if int(row['operation'])%2==0:data[tail]=int(row['actor']);tail=(tail+1)&255;value=255
            else:
                value=255
                while head!=tail:
                    candidate=data[head];head=(head+1)&255
                    if candidate<128:value=candidate;break
            assert (head,tail,value,bytes(data).hex().upper())==(int(row['after_head']),int(row['after_tail']),int(row['value']),row['bytes'])
    (out/'target-timing-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    provenance=json.loads((ROOT/'extracted/twue-3.07/build/native_battle/provenance.json').read_text())
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'timing_cases':len(timing),'dispatch_cases':len(dispatch),'delay_cases':len(delays),'ring_operations':len(rings),'target_instructions':steps,'unchanged_code_regions':provenance['unchanged_scheduler_code_regions'],'reference_kind':'bounded target queue/ATB instructions, explicit external callbacks and independent ring model; not complete battle execution','result':'pass'}
    (ROOT/'reverse-engineering/battle-scheduler-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
