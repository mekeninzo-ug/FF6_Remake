#!/usr/bin/env python3
"""Compare target-resolution stage and single-counter arithmetic with ROM code."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class ResolutionCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x20b83;self.p=0x30;self.x=0;self.y=8;self.row=row;self.resolved=None
        for address,key in [(0x11a6,'power'),(0x11a1,'element'),(0x11a2,'flags0'),(0x11a4,'flags2'),(0x11aa,'attack_status1'),(0x3ee4+self.y,'status1'),(0x3c95+self.y,'monster_flags'),(0x3ec8,'blocked'),(0x3bcc+self.y,'absorb'),(0x3bcd+self.y,'null'),(0x3be1+self.y,'half'),(0x3be0+self.y,'weak')]:self.write(0x7e0000+address,int(row[key]))
    def allowed(self):return 0x20b83<=self.pc<0x20c2d
    def hook(self,dest):
        if dest in [0x0c9e,0x0d87]:self.write(0xf0,int(self.row['amount']),16);return True
        if dest==0x0c2d:self.resolved=(self.read(0xf0,16),self.read(0xf2));return True
        return super().hook(dest)
    def result(self):return self.resolved

class AccumulatorCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x20c76;self.p=0x30;self.y=int(row['slot'])*2;self.a=0
        self.healing=int(row['healing']);self.write(0xf2,self.healing);self.write(0xf0,int(row['amount']),16)
        self.selected=0x7e33d0+self.y+self.healing*20;self.other=0x7e33d0+self.y+(1-self.healing)*20
        self.write(self.selected,int(row['current']),16);self.write(self.other,0x4321,16)
    def allowed(self):return 0x20c76<=self.pc<0x20c9e
    def result(self):
        assert self.read(self.other,16)==0x4321
        return self.read(self.selected,16)
def read(path):
    with path.open(newline='') as stream:return list(csv.DictReader(stream))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert rom[0x20b83:0x20c9e]==clean[0x20b83:0x20c9e]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-elemental-damage'
    subprocess.run([str(exe),'--capture-elemental-damage',str(out)],cwd=ROOT,check=True)
    rows=read(out/'resolution.csv');assert len(rows)==16384;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=ResolutionCpu(rom,row)
        if i in [1,100,555,1000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=None if row['result']=='' else (int(row['result']),int(row['flags']))
        assert actual==expected,(i,row,actual,expected)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    counters=read(out/'accumulation.csv');assert len(counters)==4152;counter_steps=0
    for row in counters:
        cpu=AccumulatorCpu(rom,row);actual=cpu.run();counter_steps+=cpu.steps
        assert actual==int(row['result']),(row,actual)
    (out/'target-resolution-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'resolution_cases':len(rows),'counter_cases':len(counters),'resolution_target_instructions':steps,'counter_target_instructions':counter_steps,'reference_kind':'bounded actual target elemental stage and accumulator; prior modifiers/ratio and later CalcMaxDmg effects intercepted, no current HP application','result':'pass'}
    (ROOT/'reverse-engineering/elemental-damage-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
