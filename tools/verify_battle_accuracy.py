#!/usr/bin/env python3
"""Execute actual patched ordinary accuracy path, stopping before miss graphics."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x222fb;self.p=0x30;self.x=0;self.y=8
        self.write(0xbe,int(row['seed']));self.write(0x7e11a2,int(row['physical']));self.write(0x7e11a8,int(row['rate']))
        for address,key in [(0x3b54,'evade'),(0x3b55,'magic_evade'),(0x3ee4,'status12'),(0x3ef8,'status34')]:self.write(0x7e0000+address+self.y,int(row[key]),16 if key.startswith('status') else 8)
        self.write(0x7e3ee4,int(row['blind']));self.write(0x7e3c58+self.y,4*int(row['beads']));self.write(0x7e3e4c+self.y,int(row['retort']))
        self.write(0x7e3018+self.y,0x100,16);self.write(0x7e3a54,0x100*int(row['guaranteed']),16)
    def allowed(self):return 0x222fb<=self.pc<0x2239c or 0x25864<=self.pc<0x25880 or 0x24b53<=self.pc<0x24b7b or self.pc in [0x222b3,0x222b5,0x222d1,0x222e8]
    def result(self):return int(self.pc==0x222e8),int(bool(self.read(0x7e3dfd+self.y)&4)),self.read(0xbe)
    def run(self):
        # The shared checker already supports a single explicit stop PC. Route
        # the four exits through that stop without executing miss presentation.
        self.stop_pc=0x222e8
        return super().run()
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-accuracy'
    subprocess.run([str(exe),'--capture-battle-accuracy',str(out)],cwd=ROOT,check=True)
    with (out/'accuracy.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        # Branches to the miss exit bypass the final JMP hook. Stop all exits.
        original=cpu.allowed
        def allowed():
            if cpu.pc in [0x222b5,0x222d1,0x222e8]:cpu.stop_pc=cpu.pc
            return original()
        cpu.allowed=allowed
        if i in [100,120,160,1001]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        assert actual==tuple(int(row[k]) for k in ['hit','remove_image','index']),(i,row,actual)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    assert len(rows)==16384
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'changed_accuracy_bytes':[f'{i:06X}' for i in range(0x222fb,0x2239c) if rom[i]!=clean[i]],'reference_kind':'actual patched ordinary accuracy, CheckStatus and RNG; hardware multiply helper intercepted; actual final hit/miss branch executed; excludes initial immunity gates and miss graphics','result':'pass'}
    (ROOT/'reverse-engineering/battle-accuracy-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
