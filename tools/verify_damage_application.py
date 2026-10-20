#!/usr/bin/env python3
"""Bounded actual target HP/MP helpers, including pending-status writes."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x21350 if int(row['affects_mp']) else 0x21323
        self.p=0x10;self.y=8;self.x=0;self.row=row
        for address,key in [(0x3bf4,'hp'),(0x3c08,'mp'),(0x3c1c,'max_hp'),(0x3c30,'max_mp'),(0x3ee4,'status'),(0x3dd4,'pending'),(0x3204,'update'),(0x3c95,'monster'),(0x33d0,'taken'),(0x33e4,'healed')]:self.write(0x7e0000+address+self.y,int(row[key]),16)
        self.write(0x7e3018+self.y,4,16);self.write(0x7e3a3c,4 if int(row['invincible']) else 0,16)
        self.write(0x7e3a82,int(row['golem']));self.write(0x7e3a83,int(row['dog']));self.write(0x7e3a89,int(row['weapon']))
    def allowed(self):return 0x21323<=self.pc<0x213d3 or 0x20e32<=self.pc<0x20e39 or 0x2464c<=self.pc<0x24653
    def result(self):
        return [self.read(0x7e0000+a+self.y,16) for a in [0x3bf4,0x3c08,0x3dd4,0x3204]]+[self.read(0x7e3a89),self.read(0x7e33d0+self.y,16),self.p&1]
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    for lo,hi in [(0x21323,0x213d3),(0x20e32,0x20e39),(0x2464c,0x24653)]:assert rom[lo:hi]==clean[lo:hi]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-damage-application'
    subprocess.run([str(exe),'--capture-damage-application',str(out)],cwd=ROOT,check=True)
    with (out/'application.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==16384;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,1,200,701]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=[int(row[k]) for k in ['hp_out','mp_out','pending_out','update_out','weapon_out','taken_out','carry']]
        assert actual==expected,(i,row,actual,expected)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'reference_kind':'actual target C2/1323 HP and C2/1350 MP helpers, net calculation and pending status/update writes; no hooks; excludes outer attacker attribution and status commit','result':'pass'}
    (ROOT/'reverse-engineering/damage-application-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
