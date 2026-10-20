#!/usr/bin/env python3
"""Actual monster-special and common attack preparation; graphics interfaces hooked."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x232ec;self.p=0x10;self.x=12;self.stop_pc=0x233a3;self.effects=[]
        for i,v in enumerate(bytes.fromhex(row['magic'])):self.write(0x7e11a0+i,v)
        for address,key in [(0xb2,'b2'),(0xb3,'b3'),(0xa0,'graphics'),(0xb7,'animation'),(0xbc,'multiplier')]:self.write(address,int(row[key]))
        self.write(0x7e11b0,int(row['base']),16)
        for address,key in [(0x3412,'special_index'),(0x322d+12,'property'),(0x3c81+12,'special_animation'),(0x3ee5+12,'status2'),(0x3ef9+12,'status4'),(0x3ec9,'targets'),(0x3aa1+12,'row')]:self.write(0x7e0000+address,int(row[key]))
    def allowed(self):return 0x232ec<=self.pc<=0x233a3 or 0x251f0<=self.pc<0x25269
    def hook(self,dest):
        if dest==0x62bf:assert self.a==0x2506;self.effects.append('runic');return True
        if dest==0x63db:self.effects.append('copy');return True
        return super().hook(dest)
    def result(self):
        assert self.x==12
        return bytes(self.read(0x7e11a0+i) for i in range(14)),self.read(0xb2),self.read(0xa0),self.read(0xb7),self.read(0xbc),self.read(0x7e11b0,16),'|'.join(self.effects)
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert rom[0x232ec:0x233a3]==clean[0x232ec:0x233a3]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-attack-prep'
    subprocess.run([str(exe),'--capture-battle-attack-prep',str(out)],cwd=ROOT,check=True)
    with (out/'preparation.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==16384;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,100,1000,10000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=(bytes.fromhex(row['magic_out']),)+tuple(int(row[k]) for k in ['b2_out','graphics_out','animation_out','multiplier_out','base_out'])+(row['effects'],)
        assert actual==expected,(row,actual,expected)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'reference_kind':'actual target C2/32EC..33A2 and GetBitPtr; Runic graphics/copy interfaces hooked; excludes facing, attacker effects and target execution','result':'pass'}
    (ROOT/'reverse-engineering/battle-attack-prep-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
