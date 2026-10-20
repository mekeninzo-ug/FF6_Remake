#!/usr/bin/env python3
"""Actual target living-mask rebuild and complete normal-action cleanup."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class LivingCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x24ab9;self.p=0x30
        actors=bytes.fromhex(row['actors'])
        for slot in range(10):
            self.write(0x7e3018+slot*2,1<<(slot if slot<4 else slot+4),16);self.write(0x7e3aa0+slot*2,actors[slot*2]);self.write(0x7e3ee4+slot*2,actors[slot*2+1])
        for address,key,width in [(0x2f4c,'untargetable',16),(0x2f4e,'override',16),(0x3408,'revived',16),(0x3a40,'enemy',8),(0x3a3a,'dead',8)]:self.write(0x7e0000+address,int(row[key]),width)
    def allowed(self):return 0x24ab9<=self.pc<0x24b53 or 0x251f0<=self.pc<0x25269
    def result(self):return [self.read(0x7e0000+a,16) for a in [0x2f4e,0x3a78,0x3a74,0x3a42]]+[self.read(0x7e3a76),self.read(0x7e3a77)]
class RemovalCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x20a0f;self.p=0x30;self.x=int(row['slot'])*2
        for i,v in enumerate(bytes.fromhex(row['links'])):self.write(0x7e3184+i,v)
        for i,v in enumerate(bytes.fromhex(row['queue'])):self.write(0x7e3720+i,v)
        for slot,v in enumerate(bytes.fromhex(row['normal'])):self.write(0x7e32cc+slot*2,v)
        for slot,v in enumerate(bytes.fromhex(row['retal'])):self.write(0x7e32cd+slot*2,v)
        raw=bytes.fromhex(row['payloads'])
        for i in range(128):
            p=raw[i*5:i*5+5];self.write(0x7e3420+i*2,int.from_bytes(p[:2],'little'),16);self.write(0x7e3520+i*2,int.from_bytes(p[2:4],'little'),16);self.write(0x7e3620+i*2,p[4])
        for address,key,width in [(0x3aa0+self.x,'flags',8),(0x3218+self.x,'gauge',16),(0x3a64,'head',8),(0x3a65,'tail',8),(0x340a,'immediate',8)]:self.write(0x7e0000+address,int(row[key]),width)
        self.write(0xb1,int(row['mp_flags']));self.write(0xb8,int(row['targets']),16)
    def allowed(self):return 0x20a0f<=self.pc<0x20a4a or 0x20301<=self.pc<0x2031c or 0x20792<=self.pc<0x20799 or 0x24e54<=self.pc<0x24f57
    def result(self):
        hexmem=lambda a,n:bytes(self.read(0x7e0000+a+i) for i in range(n)).hex().upper()
        heads=lambda a:bytes(self.read(0x7e0000+a+i*2) for i in range(10)).hex().upper()
        payloads=bytearray()
        for i in range(128):payloads.extend(self.read(0x7e3420+i*2,16).to_bytes(2,'little'));payloads.extend(self.read(0x7e3520+i*2,16).to_bytes(2,'little'));payloads.append(self.read(0x7e3620+i*2))
        return [self.read(0x7e3aa0+self.x),self.read(0x7e3218+self.x,16),hexmem(0x3720,256),hexmem(0x3184,128),heads(0x32cc),heads(0x32cd),self.read(0x7e340a),payloads.hex().upper(),self.read(0xb1)]
def read(path):
    with path.open(newline='') as f:return list(csv.DictReader(f))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    for lo,hi in [(0x24ab9,0x24b53),(0x20a0f,0x20a41)]:assert rom[lo:hi]==clean[lo:hi]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-cleanup'
    subprocess.run([str(exe),'--capture-battle-cleanup',str(out)],cwd=ROOT,check=True)
    living=read(out/'living.csv');removal=read(out/'removal.csv');assert len(living)==16384 and len(removal)==1024;steps=0;traces=[]
    for row in living:
        cpu=LivingCpu(rom,row);actual=cpu.run();steps+=cpu.steps
        assert actual==[int(row[k]) for k in ['override_out','valid','alive','enemy_alive','allies','enemies']],(row,actual)
    for i,row in enumerate(removal):
        cpu=RemovalCpu(rom,row)
        if i in [0,100,1000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        keys=['flags_out','gauge_out','queue_out','links_out','normal_out','retal_out','immediate_out','payloads_out','mp_flags_out'];expected=[int(row[k]) if k in ['flags_out','gauge_out','immediate_out','mp_flags_out'] else row[k] for k in keys]
        assert actual==expected,(i,[(k,a,b) for k,a,b in zip(keys,actual,expected) if a!=b])
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'living_cases':len(living),'removal_cases':len(removal),'target_instructions':steps,'reference_kind':'actual target UpdateDead and RemoveAllActions with command removal, immediate pose storage and MP-cost helper; no hooks; excludes full AfterAction2 and victory','result':'pass'}
    (ROOT/'reverse-engineering/battle-cleanup-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
