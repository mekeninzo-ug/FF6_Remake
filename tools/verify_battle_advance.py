#!/usr/bin/env python3
"""Compare native advance preparation with target code and explicit subsystem hooks."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class AdvanceCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x22188;self.p=0x30;self.x=int(row['slot'])*2;self.events=[];self.row=row
        for address,key in [(0x3aa0,'flags0'),(0x3aa1,'flags1'),(0x322c,'duration'),(0x3e4c,'retal'),(0x3255,'ai')]:self.write(0x7e0000+address+self.x,int(row[key]))
        self.write(0x7e3a4a,int(row['changed']));self.write(0x7e3a7a,int(row['command']));self.write(0x7e3a7b,int(row['attack']));self.write(0xb8,int(row['targets']),16);self.write(0xb1,int(row['mp_flags']))
        for i,v in enumerate(bytes.fromhex(row['links'])):self.write(0x7e3184+i,v)
        for slot,v in enumerate(bytes.fromhex(row['heads'])):self.write(0x7e32cc+slot*2,v)
        for slot in range(10):self.write(0x7e3018+slot*2,(1<<slot) if slot<4 else (1<<(slot+4)),16)
        payloads=bytes.fromhex(row['payloads'])
        for i in range(128):
            b=payloads[i*5:i*5+5];self.write(0x7e3420+i*2,int.from_bytes(b[:2],'little'),16);self.write(0x7e3520+i*2,int.from_bytes(b[2:4],'little'),16);self.write(0x7e3620+i*2,b[4])
    def allowed(self):return 0x22188<=self.pc<0x2220d or 0x2031c<=self.pc<0x2037b or 0x203e4<=self.pc<0x20420 or 0x20a41<=self.pc<0x20a4a or 0x25bab<=self.pc<0x25bb2 or 0x24e54<=self.pc<0x24f57
    def hook(self,dest):
        slot=self.x//2
        if dest==0x02dc:self.events.append(f'A{slot}');return True
        if dest==0x26d3:
            self.events.append(f'I{slot}:{self.read(0x7e3a7a)}:{self.read(0x7e3a7b)}')
            if int(self.row['case'])%3==1:self.write(0xb8,0,16)
            return True
        if dest==0x587e:self.events.append(f'T{slot}');self.write(0xb8,1<<((int(self.row['case'])+1)%4),16);return True
        if dest in [0x2639,0x6400,0x57c2,0x63db]:return True
        if dest==0x6411:
            assert self.a&255==4;self.events.append(f'G{slot}:{self.read(0x7e2d6f)}');return True
        if dest==0x0019:return True
        return super().hook(dest)
    def result(self):
        h=lambda address,count:bytes(self.read(0x7e0000+address+i) for i in range(count)).hex().upper()
        payloads=bytearray()
        for i in range(128):
            payloads.extend(self.read(0x7e3420+i*2,16).to_bytes(2,'little'));payloads.extend(self.read(0x7e3520+i*2,16).to_bytes(2,'little'));payloads.append(self.read(0x7e3620+i*2))
        heads=bytes(self.read(0x7e32cc+slot*2) for slot in range(10)).hex().upper()
        return (self.read(0x7e3aa1+self.x),self.read(0x7e322c+self.x),self.read(0x7e3e4c+self.x),self.read(0x7e3a4a),self.read(0x7e3a7a),self.read(0x7e3a7b),self.read(0xb8,16),self.read(0xb1),h(0x3184,128),heads,payloads.hex().upper(),'|'.join(self.events))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    regions=[(0x22188,0x2220d),(0x2031c,0x2037b)]
    for start,end in regions:assert rom[start:end]==clean[start:end]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-advance'
    subprocess.run([str(exe),'--capture-battle-advance',str(out)],cwd=ROOT,check=True)
    with (out/'advance.csv').open(newline='') as stream:rows=list(csv.DictReader(stream))
    assert len(rows)==512;steps=0;traces=[]
    for index,row in enumerate(rows):
        cpu=AdvanceCpu(rom,row)
        if index in [0,1,5,511]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=tuple(int(row[k]) for k in ['after_flags1','after_duration','after_retal','after_changed','after_command','after_attack','after_targets','after_mp_flags'])+(row['after_links'],row['after_heads'],row['after_payloads'],row['events'])
        assert actual==expected,(index,actual,expected)
        assert int(row['prepared'])==int(int(row['flags0'])&0x50==0)
        if cpu.trace is not None:traces.append({'case':index,'instructions':cpu.trace})
    (out/'target-advance-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'advance_cases':len(rows),'target_instructions':steps,'reference_kind':'bounded target advance dispatch and QueueAction; AI/targeting/graphics callbacks explicitly supplied, not live battle execution','result':'pass'}
    (ROOT/'reverse-engineering/battle-advance-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
