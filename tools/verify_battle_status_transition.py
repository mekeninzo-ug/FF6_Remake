#!/usr/bin/env python3
"""Execute status-variable setup, controlled post-handler store and pending reset."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row,phase='prepare'):
        super().__init__(rom,0,0,[255]*6);self.pc={'prepare':0x2450d,'store':0x24585,'reset':0x244ff}[phase];self.p=0x10;self.x=12;self.y=int(row['case'])%10*2;self.phase=phase;self.row=row
        for address,key in [(0x3ee4,'current12'),(0x3ef8,'current34'),(0x3dd4,'set12'),(0x3de8,'set34'),(0x3dfc,'clear12'),(0x3e10,'clear34'),(0x331c,'allowed12'),(0x3330,'allowed34'),(0x3bf4,'hp'),(0x3c1c,'max_hp'),(0x3e4d,'extra'),(0x32df,'marker'),(0x3e60,'retal12'),(0x3e74,'retal34')]:self.write(0x7e0000+address+self.y,int(row[key]),16)
        if phase=='store':self.write(0xfc,int(row['resolved12']),16);self.write(0xfe,int(row['resolved34']),16)
    def allowed(self):return 0x244ff<=self.pc<0x24598 or 0x2447f<=self.pc<0x24490
    def result(self):
        assert self.x==12
        if self.phase=='store':return self.read(0x7e3ee4+self.y,16),self.read(0x7e3ef8+self.y,16)
        assert self.read(0x7e3ee4+self.y,16)==int(self.row['current12']) and self.read(0x7e3ef8+self.y,16)==int(self.row['current34'])
        if self.phase=='reset':return ''.join(f'{self.read(0x7e0000+a+self.y,16):04X}' for a in [0x3dd4,0x3de8,0x3dfc,0x3e10])
        assert [self.read(0x7e0000+a+self.y,16) for a in [0x3dd4,0x3de8,0x3dfc,0x3e10]]==[int(self.row[k]) for k in ['set12','set34','clear12','clear34']]
        return [self.read(a,16) for a in [0xfc,0xfe,0xf4,0xf6]]+[self.read(0x7e3e60+self.y,16),self.read(0x7e3e74+self.y,16)]
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert rom[0x244ff:0x24598]==clean[0x244ff:0x24598]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-status-transition'
    subprocess.run([str(exe),'--capture-battle-status-transition',str(out)],cwd=ROOT,check=True)
    with (out/'preparation.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==16384;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,100,1000,10000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        assert actual==[int(row[k]) for k in ['out_set12','out_set34','out_clear12','out_clear34','out_retal12','out_retal34']],(i,row,actual)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
        store=Cpu(rom,row,'store');actual=store.run();steps+=store.steps
        assert actual==(int(row['stored12']),int(row['stored34'])),(i,row,actual)
        reset=Cpu(rom,row,'reset');actual=reset.run();steps+=reset.steps
        assert actual==row['reset'],(i,row,actual)
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases_per_helper':len(rows),'target_instructions':steps,'reference_kind':'actual target InitStatusVars, CalcStatus snapshot, controlled post-handler store and ResetStatusMod; no hooks; individual status handlers are excluded','result':'pass'}
    (ROOT/'reverse-engineering/battle-status-transition-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
