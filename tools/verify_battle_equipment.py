#!/usr/bin/env python3
"""Bounded UpdateEquipBattle projection with source's direct page at 1100."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
FIELDS=[(0x3b2c,1),(0x3b2d,1),(0x3b19,1),(0x3b40,1),(0x3b41,1),(0x3b54,1),(0x3b55,1),(0x3c6c,2),(0x3d71,1),(0x3c95,1),(0x3c44,2),(0x3b68,2),(0x3b7c,2),(0x3d34,2),(0x3b90,2),(0x3ba4,2),(0x3bb8,2),(0x331c,2),(0x3bcc,2),(0x3be0,2),(0x3cbc,2),(0x3ca8,2),(0x3cd0,2),(0x3ce4,2),(0x3c58,2)]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x22883;self.p=0x30;self.x=2*(int(row['case'])%4);self.stop_pc=0x22944
        for i,v in enumerate(bytes.fromhex(row['scratch'])):self.write(0x7e11a0+i,v)
        self.write(0x7e3a6d,int(row['party']))
    def read(self,address,width=8):return super().read(0x7e1100+address if address<256 else address,width)
    def write(self,address,value,width=8):return super().write(0x7e1100+address if address<256 else address,value,width)
    def allowed(self):return 0x22861<=self.pc<=0x22944
    def result(self):
        out=bytearray()
        for address,size in FIELDS:out.extend(self.read(0x7e0000+address+self.x,size*8).to_bytes(size,'little'))
        out.extend([self.read(0x7e2e6e+self.x//2),self.read(0x7e3a6d)])
        return out,bytes(self.read(0x7e11a0+i) for i in range(64))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert rom[0x22861:0x22944]==clean[0x22861:0x22944]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-equipment'
    subprocess.run([str(exe),'--capture-battle-equipment',str(out)],cwd=ROOT,check=True)
    with (out/'projection.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==8195;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,100,1000]:cpu.trace=[]
        actual,scratch=cpu.run();steps+=cpu.steps
        assert actual==bytes.fromhex(row['result']),(i,actual.hex(),row['result'])
        assert scratch==bytes.fromhex(row['scratch_out']),(i,scratch.hex(),row['scratch_out'])
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'reference_kind':'actual target C2/2883..2943 and InvertEvade with direct page 1100; excludes sprite/visual flags and FixImmuneStatus; no hooks','result':'pass'}
    (ROOT/'reverse-engineering/battle-equipment-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
