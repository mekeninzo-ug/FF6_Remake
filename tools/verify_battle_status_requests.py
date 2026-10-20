#!/usr/bin/env python3
"""Execute MagicStatusEffect and CalcStatus for native pending status requests."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row,resolve=False):
        super().__init__(rom,0,0,[255]*6);self.pc=0x2447f if resolve else 0x24406;self.p=0x10;self.x=18;self.y=(int(row['case'])%10)*2;self.resolve=resolve;self.row=row
        for i,v in enumerate(bytes.fromhex(row['magic'])):self.write(0x7e11a0+i,v)
        self.write(0xb3,int(row['b3']))
        for address,key in [(0x3ee4,'current12'),(0x3ef8,'current34'),(0x3dd4,'set12'),(0x3de8,'set34'),(0x3dfc,'clear12'),(0x3e10,'clear34'),(0x331c,'allowed12'),(0x3330,'allowed34'),(0x3018,'mask')]:self.write(0x7e0000+address+self.y,int(row[key]),16)
        self.write(0x7e3c95+self.y,int(row['monster']));self.write(0x7e3a48,int(row['failed']),16);self.write(0x7e3a5a,int(row['miss']),16)
        if resolve:
            for address,key in [(0xf8,'current12'),(0xfa,'current34'),(0xfc,'delta_set12'),(0xfe,'delta_set34'),(0xf4,'delta_clear12'),(0xf6,'delta_clear34')]:self.write(address,int(row[key]),16)
    def allowed(self):return 0x24406<=self.pc<0x244ff or 0x20e32<=self.pc<0x20e39
    def result(self):
        assert self.x==18
        if self.resolve:return self.read(0xfc,16),self.read(0xfe,16)
        assert self.read(0x7e3ee4+self.y,16)==int(self.row['current12'])
        assert self.read(0x7e3ef8+self.y,16)==int(self.row['current34'])
        return [self.read(0x7e0000+a+self.y,16) for a in [0x3dd4,0x3de8,0x3dfc,0x3e10]]+[self.read(0x7e3a48,16),self.read(0x7e3a5a,16)]+[self.read(a,16) for a in [0xfc,0xfe,0xf4,0xf6]]
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert rom[0x24406:0x244ff]==clean[0x24406:0x244ff]
    assert all(rom[0x46ac0+i*14+4]&12!=12 for i in range(256))
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-status-requests'
    subprocess.run([str(exe),'--capture-battle-status-requests',str(out)],cwd=ROOT,check=True)
    with (out/'requests.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==16384;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,100,1000,10000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=[int(row[k]) for k in ['set12_out','set34_out','clear12_out','clear34_out','failed_out','miss_out','delta_set12','delta_set34','delta_clear12','delta_clear34']]
        assert actual==expected,(i,row,actual,expected)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
        resolve=Cpu(rom,row,True);actual=resolve.run();steps+=resolve.steps
        assert actual==(int(row['resolved12']),int(row['resolved34'])),(i,row,actual)
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'request_cases':len(rows),'resolve_cases':len(rows),'target_instructions':steps,'reference_kind':'actual target MagicStatusEffect, EquipStatusMod, SetStatus1 and CalcStatus; no hooks; does not execute per-status handlers or commit','result':'pass'}
    (ROOT/'reverse-engineering/battle-status-requests-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
