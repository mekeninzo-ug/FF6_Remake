#!/usr/bin/env python3
"""Actual CheckHit prefix and special modes; stops before miss processing."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x2220d;self.p=0x30;self.y=int(row['offset'])
        for address,key in [(0xb3,'b3'),(0xb5,'command'),(0xbe,'seed')]:self.write(address,int(row[key]))
        for address,key in [(0x11a2,'flags0'),(0x11a3,'flags1'),(0x11a4,'flags2'),(0x11a9,'special'),(0x3ec9,'targets'),(0x11a8,'rate')]:self.write(0x7e0000+address,int(row[key]))
        for address,key in [(0x3ee4,'status1'),(0x3ef8,'status3'),(0x3ef9,'status4'),(0x3c95,'monster'),(0x3aa1,'protection'),(0x3b18,'level'),(0x3b40,'stamina'),(0x3b55,'magic_evade')]:self.write(0x7e0000+address+self.y,int(row[key]))
        self.write(0x7e3a36,int(row['golem']),16);self.write(0x7e3018+self.y,0x100,16)
    def allowed(self):
        if self.pc in [0x222fb,0x222e8,0x222b5,0x222d1,0x222e5]:self.stop_pc=self.pc
        return 0x2220d<=self.pc<=0x222fb or 0x2239c<=self.pc<0x223bf or 0x24b53<=self.pc<0x24b7b
    def hook(self,dest):
        if dest==0x4792:
            assert self.rom[0x24792:0x247b7]==bytes.fromhex('5a 08 c2 20 8f 04 42 00 e2 30 8a 8f 06 42 00 ea ea ea ea ea ea ea ea af 16 42 00 aa c2 20 af 14 42 00 28 7a 60')
            dividend=self.a;divisor=self.x&255;self.x=(dividend%divisor if divisor else dividend)&255;self.a=dividend//divisor if divisor else 65535
            return True
        return super().hook(dest)
    def result(self):return {0x222fb:0,0x222e8:1,0x222b5:2,0x222d1:3,0x222e5:4}[self.pc],int(bool(self.read(0x7e3dfc+self.y)&16)),self.read(0xfe),self.read(0xbe)
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert rom[0x2220d:0x222b5]==clean[0x2220d:0x222b5]
    assert rom[0x2239c:0x223bf]==clean[0x2239c:0x223bf]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-hit-gates'
    subprocess.run([str(exe),'--capture-battle-hit-gates',str(out)],cwd=ROOT,check=True)
    with (out/'gates.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==32768;steps=0;counts={};traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [100,121,500,1001]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        assert actual==tuple(int(row[k]) for k in ['route','vanish','block','index']),(i,row,actual)
        counts[actual[0]]=counts.get(actual[0],0)+1
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    assert set(counts)==set(range(5)),counts
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'route_counts':counts,'reference_kind':'actual target CheckHit prefix, special modes and RNG; hardware multiplication/division hooked; stops before ordinary accuracy or miss processing','result':'pass'}
    (ROOT/'reverse-engineering/battle-hit-gates-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
