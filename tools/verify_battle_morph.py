#!/usr/bin/env python3
"""Actual target UpdateMorph and SaveMorphCounter, including menu refresh."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x20aa8;self.p=0x30;self.x=int(row['slot'])*2;self.base=0x202e+self.x*6;self.row=row
        self.write(0xb1,int(row['b1']))
        for address,key,width in [(0x3ee2,'actor',8),(0x3f30,'counter',16),(0x3f32,'rate',16),(0x1cf6,'saved',8),(0x3ebb,'story',8),(0x3a58,'changed',16)]:self.write(0x7e0000+address,int(row[key]),width)
        for address,key,width in [(0x3aa0,'flags',8),(0x3218,'gauge',16),(0x3ef9,'status4',8),(0x3ee4,'status1',8),(0x3ee5,'status2',8),(0x3ba4,'right',8),(0x3ba5,'left',8),(0x3c58,'relic',8),(0x3b04,'morph_gauge',8)]:self.write(0x7e0000+address+self.x,int(row[key]),width)
        for i,v in enumerate(bytes.fromhex(row['records'])):self.write(0x7e0000+self.base+i,v)
        self.write(0x7e3018+self.x,1<<(int(row['slot']) if int(row['slot'])<4 else int(row['slot'])+4),16)
    def write(self,address,value,width=8):
        super().write(address,value,width)
        if address==0x4202 and width==16:super().write(0x4216,(value&255)*(value>>8),16)
        if address==0x4206:
            dividend=self.read(0x4204,16);divisor=value&255
            super().write(0x4214,dividend//divisor if divisor else 65535,16);super().write(0x4216,dividend%divisor if divisor else dividend,16)
    def allowed(self):return 0x20aa8<=self.pc<0x20b4a or 0x211b4<=self.pc<0x211bb or 0x24781<=self.pc<0x247b7 or 0x2527d<=self.pc<0x2532c
    def hook(self,dest):return False
    def result(self):
        assert self.x==int(self.row['slot'])*2
        return [self.read(0x7e3ee2),self.read(0x7e3f30,16),self.read(0x7e3f32,16),self.read(0x7e1cf6),self.read(0x7e3a58,16),self.read(0x7e3aa0+self.x),self.read(0x7e3b04+self.x),bytes(self.read(0x7e0000+self.base+i) for i in range(12)).hex().upper()]
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes();assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9';assert rom[0x20aa8:0x20b4a]==clean[0x20aa8:0x20b4a]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-morph';subprocess.run([str(exe),'--capture-battle-morph',str(out)],cwd=ROOT,check=True)
    with (out/'morph.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==8192;steps=0;traces=[];keys=['actor_out','counter_out','rate_out','saved_out','changed_out','flags_out','morph_gauge_out','records_out']
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,100,1000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps;expected=[row[k] if k=='records_out' else int(row[k]) for k in keys]
        assert actual==expected,(i,[(k,a,b) for k,a,b in zip(keys,actual,expected) if a!=b],row)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'reference_kind':'actual target UpdateMorph, SaveMorphCounter, UpdateCmdList and hardware multiply/divide helpers; no routine hooks; register arithmetic modeled without cycle timing; not full AfterAction2','result':'pass'}
    (ROOT/'reverse-engineering/battle-morph-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
