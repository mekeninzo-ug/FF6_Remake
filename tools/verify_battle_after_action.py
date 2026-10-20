#!/usr/bin/env python3
"""Combined actual-target AfterAction2 comparison; only graphics submission hooked."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
def decode(raw):return {int(raw[i:i+6],16):int(raw[i+6:i+8],16) for i in range(0,len(raw),8)}
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x2083f;self.p=0x30;self.instruction_budget=200000;self.effects=[];self.expected=decode(row['after']);self.memory.update(decode(row['before']));self.visited=set()
    def write(self,address,value,width=8):
        super().write(address,value,width)
        if address==0x4202 and width==16:super().write(0x4216,(value&255)*(value>>8),16)
        if address==0x4206:
            dividend=self.read(0x4204,16);divisor=value&255
            super().write(0x4214,dividend//divisor if divisor else 65535,16);super().write(0x4216,dividend%divisor if divisor else dividend,16)
    def allowed(self):
        self.visited.add(self.pc)
        return 0x20301<=self.pc<0x2031c or 0x2083f<=self.pc<0x20b83 or 0x211b4<=self.pc<0x211bb or 0x211ef<=self.pc<0x21211 or 0x20792<=self.pc<0x20799 or 0x24e54<=self.pc<0x24f57 or 0x25864<=self.pc<0x2587e or 0x24781<=self.pc<0x247b7 or 0x24b53<=self.pc<0x24b7b or 0x2527d<=self.pc<0x2532c or 0x25763<=self.pc<0x257c2
    def hook(self,dest):
        if dest==0x6411:
            assert self.a&255 in [2,3]
            self.effects.append(f'{"open" if self.a&255==2 else "close"}:{self.read(0x10)}');return True
        return False
    def result(self):return [(f'{a:06X}',self.read(a),v) for a,v in self.expected.items() if self.read(a)!=v]
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes();assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9';assert rom[0x2083f:0x20b83]==clean[0x2083f:0x20b83]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-after-action';subprocess.run([str(exe),'--capture-battle-after-action',str(out)],cwd=ROOT,check=True)
    csv.field_size_limit(1000000)
    with (out/'after-action.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==1024;steps=0;traces=[];visited=set();effects={'open':0,'close':0}
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,255,1000]:cpu.trace=[]
        diff=cpu.run();steps+=cpu.steps;visited|=cpu.visited
        assert not diff,(i,diff[:30]);assert '|'.join(cpu.effects)==row['effects'],(i,cpu.effects,row['effects'])
        for e in cpu.effects:effects[e.split(':')[0]]+=1
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    entries={'quetzalli':0x20b4a,'spells':0x25763,'remove':0x20a0f,'start_condemn':0x209b4,'stop_condemn':0x209ce,'commands':0x2527d,'speed':0x209d2,'morph':0x20aa8,'player':0x2091f,'pending':0x208c6,'clear_def':0x20a41,'near_fatal':0x20a4a,'disable':0x20977}
    assert all(pc in visited for pc in entries.values()),[n for n,pc in entries.items() if pc not in visited]
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'handler_entries_executed':list(entries),'menu_effects':effects,'reference_kind':'complete actual target AfterAction2 and all called state/queue/menu handlers; only ExecBtlGfx submission intercepted; hardware arithmetic modeled without cycle timing; not live battle or rendered SNES comparison','result':'pass'}
    (ROOT/'reverse-engineering/battle-after-action-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
