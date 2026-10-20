#!/usr/bin/env python3
"""Actual target command and spell/esper menu eligibility refresh, no hooks."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row,spells=False):
        super().__init__(rom,0,0,[255]*6);self.pc=0x25763 if spells else 0x2527d;self.p=0x30;self.x=int(row['slot'])*2;self.row=row;self.spells=spells
        self.base=0x5000 if spells else int.from_bytes(rom[0x2544a+self.x:0x2544c+self.x],'little')
        for i,v in enumerate(bytes.fromhex(row['records'])):self.write(0x7e0000+self.base+i,v)
        self.write(0x7e3018+self.x,1<<int(row['slot']) if int(row['slot'])<4 else 1<<(int(row['slot'])+4),16);self.write(0x7e3ee4+self.x,int(row['status1']))
        if spells:
            self.write(0x7e3c08+self.x,int(row['mp']),16);self.write(0x7e3f2e,int(row['used']));self.write(0x7e302c+self.x,self.base,16);self.write(0x7e3a4c,int(row['max_cost']))
        else:
            for address,key in [(0x3ee5,'status2'),(0x3ba4,'right'),(0x3ba5,'left'),(0x3c58,'relic')]:self.write(0x7e0000+address+self.x,int(row[key]))
            self.write(0x7e1cf6,int(row['morph']));self.write(0x7e3a58,int(row['changed']),16)
    def allowed(self):return 0x2527d<=self.pc<0x2532c or 0x25763<=self.pc<0x257c2
    def result(self):
        assert self.x==int(self.row['slot'])*2
        records=bytes(self.read(0x7e0000+self.base+i) for i in range(316 if self.spells else 12)).hex().upper()
        return self.read(0x7e3a4c) if self.spells else self.read(0x7e3a58,16),records
def read(path):
    with path.open(newline='') as f:return list(csv.DictReader(f))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    for lo,hi in [(0x2527d,0x2532c),(0x25763,0x257c2)]:assert rom[lo:hi]==clean[lo:hi]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-menu-state'
    subprocess.run([str(exe),'--capture-battle-menu-state',str(out)],cwd=ROOT,check=True)
    commands=read(out/'commands.csv');spells=read(out/'spells.csv');assert len(commands)==8192 and len(spells)==2048;steps=0;traces=[]
    for is_spell,rows in [(False,commands),(True,spells)]:
        for i,row in enumerate(rows):
            cpu=Cpu(rom,row,is_spell)
            if i in [0,100,1000]:cpu.trace=[]
            actual=cpu.run();steps+=cpu.steps
            expected=(int(row['max_cost_out' if is_spell else 'changed_out']),row['records_out'])
            assert actual==expected,(is_spell,i,row,actual,expected)
            if cpu.trace is not None:traces.append({'spells':is_spell,'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'command_cases':len(commands),'spell_cases':len(spells),'target_instructions':steps,'reference_kind':'actual target UpdateCmdList, UpdateEnabledMagic and CheckMagicEnabled; no hooks; supplied menu records, no rendering or list construction','result':'pass'}
    (ROOT/'reverse-engineering/battle-menu-state-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
