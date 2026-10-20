#!/usr/bin/env python3
"""Actual target LoadRageProp, shared by monster setup and Rage."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
FIELDS=[(0x322d,1),(0x3be0,2),(0x331c,1),(0x331d,1),(0x3330,1),(0x3331,1),(0x3dd4,2),(0x3de8,2),(0x3c58,2),(0x3c6c,2),(0x3bcc,2),(0x3c94,2)]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x22dc1;self.p=0x20;self.x=int(row['id'])*32;self.y=2*(int(row['case'])%10)
        raw=bytes.fromhex(row['before']);offset=0
        for address,size in FIELDS:
            self.write(0x7e0000+address+self.y,int.from_bytes(raw[offset:offset+size],'little'),size*8);offset+=size
        assert offset==len(raw)
    def allowed(self):return 0x22dc1<=self.pc<0x22e3a
    def result(self):return b''.join(self.read(0x7e0000+a+self.y,size*8).to_bytes(size,'little') for a,size in FIELDS)
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert rom[0x22dc1:0x22e3a]==clean[0x22dc1:0x22e3a]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-rage'
    subprocess.run([str(exe),'--capture-battle-rage',str(out)],cwd=ROOT,check=True)
    with (out/'traits.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==6144;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,100,2000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        assert actual==bytes.fromhex(row['after']),(i,row,actual.hex())
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'reference_kind':'actual target LoadRageProp with all 384 records and existing actor traits; no hooks; pending status words not committed','result':'pass'}
    (ROOT/'reverse-engineering/battle-rage-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
