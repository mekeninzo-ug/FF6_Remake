#!/usr/bin/env python3
"""Actual target casting gate and MP payment, with graphics interfaces hooked."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class CastCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x23201;self.p=0x30;self.x=int(row['offset']);self.reset=None
        self.write(0x7e11a5,int(row['cost']));self.write(0x7e3ee4+self.x,int(row['status1']));self.write(0x7e3ee5+self.x,int(row['status2']));self.write(0x7e3410,int(row['spell']))
    def allowed(self):
        if self.pc in [0x23225,0x23275]:self.stop_pc=self.pc
        return 0x23201<=self.pc<=0x23225 or self.pc==0x23275
    def hook(self,dest):
        if dest==0x62bf:assert self.a&255==14;self.reset=self.a>>8;return True
        return super().hook(dest)
    def result(self):return int(self.pc==0x23225),self.reset
class PayCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x232b1;self.p=0x10;self.x=int(row['offset']);self.stop_pc=0x232ec;self.effects=[];self.initial_cost=int(row['cost'])
        for address,key,width in [(0x7e3c08+self.x,'mp',16),(0x7e3a4c,'cost',16),(0x7e3204+self.x,'update',16),(0xb5,'command',8),(0x7e11a7,'message',16),(0xa2,'gfx_a2',16),(0xa4,'gfx_a4',16)]:self.write(address,int(row[key]),width)
    def allowed(self):return 0x232b1<=self.pc<=0x232ec
    def hook(self,dest):
        if dest==0x35ad:self.effects.append('save');return True
        if dest==0x629b:assert self.a==0x2802;self.effects.append('mp');return True
        if dest==0x63db:self.effects.append('copy');return True
        return super().hook(dest)
    def result(self):return [0 if self.initial_cost==0 else 2 if self.effects else 1]+[self.read(a,w) for a,w in [(0x7e3c08+self.x,16),(0x7e3a4c,16),(0x7e3204+self.x,16),(0xb5,8),(0x7e11a7,16),(0xa2,16),(0xa4,16)]]+['|'.join(self.effects)]
def read(path):
    with path.open(newline='') as f:return list(csv.DictReader(f))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    for lo,hi in [(0x23201,0x23225),(0x232b1,0x232ec)]:assert rom[lo:hi]==clean[lo:hi]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-resources'
    subprocess.run([str(exe),'--capture-battle-resources',str(out)],cwd=ROOT,check=True)
    casting=read(out/'casting.csv');payments=read(out/'payment.csv');assert len(casting)==810 and len(payments)==16384
    steps=0;traces=[]
    for row in casting:
        cpu=CastCpu(rom,row);actual=cpu.run();steps+=cpu.steps
        assert actual==(int(row['allowed']),int(row['reset']) if row['reset'] else None),(row,actual)
    for i,row in enumerate(payments):
        cpu=PayCpu(rom,row)
        if i in [0,1,20,100]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=[int(row[k]) for k in ['result','mp_out','cost_out','update_out','command_out','message_out','gfx_a2_out','gfx_a4_out']]+[row['effects']]
        assert actual==expected,(row,actual,expected)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'casting_cases':len(casting),'payment_cases':len(payments),'target_instructions':steps,'reference_kind':'actual target casting gate and cached MP payment; graphics queue/setup/copy interfaces intercepted; no complete attack execution','result':'pass'}
    (ROOT/'reverse-engineering/battle-resources-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
