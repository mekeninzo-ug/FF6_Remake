#!/usr/bin/env python3
"""Execute complete actual target ChooseTarget and helper routines."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x2587e;self.p=0x10;self.x=int(row['actor'])*2;self.y=0x79
        for address,key in [(0xba,'restrictions'),(0xbb,'cursor'),(0xbe,'seed')]:self.write(address,int(row[key]))
        self.write(0xb8,int(row['targets']),16)
        for address,key in [(0x3a46,'mask_flags'),(0x3a78,'valid'),(0x3408,'revived'),(0x3f2c,'jump'),(0x3a4e,'backup')]:self.write(0x7e0000+address,int(row[key]),8 if key=='mask_flags' else 16)
        targettable=int(row['targettable']);self.write(0x7e3403,targettable&255);self.write(0x7e2f46,targettable>>8)
        self.write(0x7e341b,128 if int(row['remove_jump']) else 0)
        for address,key in [(0x3a40,'enemy'),(0x201f,'battle_type'),(0x2eac,'left'),(0x2ead,'right')]:self.write(0x7e0000+address,int(row[key]))
        self.write(0x7e3a32,0);self.write(0x7e2c5f,int(row['previous']))
        actors=row['actors']
        for slot in range(10):
            x=slot*2;s=actors[slot*12:slot*12+12]
            self.write(0x7e3018+x,1<<(slot if slot<4 else slot+4),16)
            self.write(0x7e3aa0+x,int(s[:2],16));self.write(0x7e3ee4+x,int(s[2:6],16),16);self.write(0x7e3ef8+x,int(s[6:10],16),16)
            self.write(0x7e3395+x,int(s[10:12],16))
    def allowed(self):return 0x2587e<=self.pc<0x25a83 or 0x24b53<=self.pc<0x24b7b or 0x251f0<=self.pc<0x25269
    def result(self):return self.read(0xb8,16),self.read(0x7e3a46),self.read(0xbe)
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert rom[0x2587e:0x25a83]==clean[0x2587e:0x25a83]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-targets'
    subprocess.run([str(exe),'--capture-battle-targets',str(out)],cwd=ROOT,check=True)
    with (out/'selection.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==16384;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,100,2000,10000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        assert actual==tuple(int(row[k]) for k in ['result','after_flags','index']),(i,row,actual)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'reference_kind':'complete actual target ChooseTarget, masking, presence, retarget and battle-type helpers with actual RNG; hardware multiply helper modeled; no full battle runtime','result':'pass'}
    (ROOT/'reverse-engineering/battle-targets-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
