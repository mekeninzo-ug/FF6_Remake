#!/usr/bin/env python3
"""Run complete bounded target CheckHit, including return and evasion effects."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_battle_hit_gates import Cpu as GateCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(GateCpu):
    def __init__(self,rom,row):
        super().__init__(rom,row);self.x=2 if self.y==0 else 0
        self.p=[0x10,0x30][int(row['case'])%2];self.initial_p=self.p
        self.initial_x=self.x;self.a=0xb669;self.initial_y=self.y
        self.write(0xb2,0x40);self.write(0x7e3ee5+self.y,int(row['status2']))
        self.write(0x7e3ee4+self.x,int(row['blind']));self.write(0x7e3c58+self.y,4*int(row['beads']));self.write(0x7e3e4c+self.y,int(row['retort']))
        self.write(0x7e3b54+self.y,int(row['evade']));self.write(0x7e3a54,int(row['mask'])*int(row['guaranteed']),16)
        self.write(0x7e3018+self.y,int(row['mask']),16)
        self.write(0x7e3ce4+self.y,int(row['physical_block']));self.write(0x7e3ce5+self.y,int(row['magical_block']))
        self.write(0x7e341c,int(row['show_miss']));self.write(0x7e3a89,19)
        self.write(0x7e3dfc+self.y,0x2000,16);self.write(0xa6,0x4000,16);self.write(0x7e3a5a,0x4000,16)
        self.write(0x7e3a82,255);self.write(0x7e3a83,255);self.write(0x7e00aa+self.y//2,9)
        for i,value in enumerate([1,2,4,8]):self.write(0x7e11aa+i,value)
    def allowed(self):return 0x2220d<=self.pc<0x223ed or 0x25864<=self.pc<0x25880 or 0x24b53<=self.pc<0x24b7b or 0x251f0<=self.pc<0x25269
    def result(self):
        assert (self.x,self.y,self.a&(255 if self.initial_p&0x20 else 65535))==(self.initial_x,self.initial_y,0x69 if self.initial_p&0x20 else 0xb669)
        assert self.p&0x7c==self.initial_p
        reflected=self.read(0xa6,16)
        outcome=2 if reflected!=0x4000 else self.p&1
        return [outcome,self.read(0xb2),self.read(0x7e3a89),self.read(0x7e3dfc+self.y,16),reflected,self.read(0x7e3a5a,16),self.read(0x7e3a82),self.read(0x7e3a83),self.read(0x7e00aa+self.y//2),''.join(f'{self.read(0x7e11aa+i):02X}' for i in range(4)),self.read(0xbe)]
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-hit'
    subprocess.run([str(exe),'--capture-battle-hit',str(out)],cwd=ROOT,check=True)
    with (out/'check-hit.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==32768;steps=0;counts={};traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [100,121,500,1001]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        keys=['outcome','b2','weapon','remove','reflected','miss_targets','golem_target','dog_target','graphic','status','index']
        expected=[row[k] if k=='status' else int(row[k]) for k in keys]
        assert actual==expected,(i,row,actual,expected)
        counts[actual[0]]=counts.get(actual[0],0)+1
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    assert set(counts)=={0,1,2}
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'outcomes':counts,'reference_kind':'complete actual target CheckHit and bit/status/RNG helpers; hardware multiplication/division modeled; effects compared without original PPU or full action runtime','result':'pass'}
    (ROOT/'reverse-engineering/battle-hit-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()

