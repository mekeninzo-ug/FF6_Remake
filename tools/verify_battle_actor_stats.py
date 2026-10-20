#!/usr/bin/env python3
"""Bounded monster working stats and status immunity normalization."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class MonsterCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x22ce4;self.p=0;self.x=int(row['id'])*32;self.y=8+(int(row['case'])%6)*2;self.stop_pc=0x22d49
        self.write(0x7e3c1c+self.y,int(row['max_hp']),16);self.write(0x7e3e4c+self.y,int(row['retal']))
    def allowed(self):return 0x22ce4<=self.pc<=0x22d49 or 0x22861<=self.pc<0x2286d or 0x247d6<=self.pc<0x247e3
    def result(self):return bytes(self.read(0x7e0000+a+self.y) for a in [0x3b40,0x3b68,0x3ca8,0x3b54,0x3b55,0x3b7c,0x3b18,0x3b19,0x3b41,0x3e4c,0x3c80])
class ImmunityCpu(EquipmentCpu):
    def __init__(self,rom,row,update=False):
        super().__init__(rom,0,0,[255]*6);self.pc=0x22675 if update else 0x22650;self.p=0x30;self.x=2*(int(row['case'])%10)
        for address,value in zip([0x331c,0x331d,0x3330,0x3331],bytes.fromhex(row['fixed' if update else 'allowed'])):self.write(0x7e0000+address+self.x,value)
        self.write(0x7e3aa1+self.x,int(row['protection_out' if update else 'protection']));self.write(0x7e3bcd+self.x,int(row['null']))
        self.write(0x7e3c6c+self.x,int(row['equip']),16);self.write(0x7e3ebb,int(row['morph']))
    def allowed(self):return 0x22650<=self.pc<0x226c9
    def result(self):return bytes(self.read(0x7e0000+a+self.x) for a in [0x331c,0x331d,0x3330,0x3331]),self.read(0x7e3aa1+self.x)
def read(path):
    with path.open(newline='') as f:return list(csv.DictReader(f))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    for lo,hi in [(0x22ce4,0x22d49),(0x22650,0x226c9)]:assert rom[lo:hi]==clean[lo:hi]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-actor-stats'
    subprocess.run([str(exe),'--capture-battle-actor-stats',str(out)],cwd=ROOT,check=True)
    monsters=read(out/'monsters.csv');immune=read(out/'immunity.csv');assert len(monsters)==3456;assert len(immune)==8192
    steps=0;traces=[]
    for i,row in enumerate(monsters):
        cpu=MonsterCpu(rom,row)
        if i in [0,6,7,8]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        assert actual==bytes.fromhex(row['result']),(i,row,actual.hex())
        if cpu.trace is not None:traces.append({'kind':'monster','case':i,'instructions':cpu.trace})
    for i,row in enumerate(immune):
        for update in [False,True]:
            cpu=ImmunityCpu(rom,row,update)
            if i==100:cpu.trace=[]
            actual=cpu.run();steps+=cpu.steps
            assert actual==(bytes.fromhex(row['updated' if update else 'fixed']),int(row['protection_out'])),(i,update,row,actual)
            if cpu.trace is not None:traces.append({'kind':'update-immunity' if update else 'fix-immunity','case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'monster_cases':len(monsters),'immunity_cases_per_routine':len(immune),'target_instructions':steps,'reference_kind':'actual target monster working-stat projection, InvertEvade, AddHalf, FixImmuneStatus and UpdateImmuneStatus; no hooks; excludes complete actor loading','result':'pass'}
    (ROOT/'reverse-engineering/battle-actor-stats-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
