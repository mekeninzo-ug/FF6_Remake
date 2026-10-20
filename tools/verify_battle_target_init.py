#!/usr/bin/env python3
"""Execute complete target InitTarget including actual property block copy."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x226d3;self.p=0x30;self.a=int(row['command'])|(int(row['attack'])<<8);self.x=12;self.y=16
        self.write(0xba,255);self.write(0xbb,255);self.write(0xb1,0x61);self.write(0x7e3410,0x55);self.write(0x7e3411,0x66)
        for i in range(14):self.write(0x7e11a0+i,170)
    def allowed(self):return 0x226d3<=self.pc<0x22778 or 0x22966<=self.pc<0x2298a or 0x22b63<=self.pc<0x22b69
    def result(self):
        assert (self.x,self.y)==(12,16)
        return self.read(0xba),self.read(0xbb),self.read(0xb1),self.read(0x7e3410),self.read(0x7e3411),bytes(self.read(0x7e11a0+i) for i in range(14))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    for lo,hi in [(0x226d3,0x227a8),(0x22966,0x2298a),(0x22b63,0x22b69)]:assert rom[lo:hi]==clean[lo:hi]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-target-init'
    subprocess.run([str(exe),'--capture-battle-target-init',str(out)],cwd=ROOT,check=True)
    with (out/'initialization.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==65536;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,511,2230,7555]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=tuple(int(row[k]) for k in ['restrictions','cursor','flags','last_spell','last_item'])+(bytes.fromhex(row['magic']),)
        assert actual==expected,(i,row,actual,expected)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'reference_kind':'actual target InitTarget, item pointer and LoadMagicProp including block move; hardware multiply helper modeled; all command/attack byte pairs, no full battle runtime','result':'pass'}
    (ROOT/'reverse-engineering/battle-target-init-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
