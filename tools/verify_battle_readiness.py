#!/usr/bin/env python3
"""Actual target readiness, player input eligibility, cancel and ATB disable."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=[0x208c6,0x2091f,0x20986,0x20977][int(row['mode'])];self.p=0x30;self.x=int(row['slot'])*2;self.effects=[]
        self.write(0xb1,int(row['b1']))
        for address,key,width in [(0x3404,'quick',8),(0x3a97,'colosseum',8),(0x2f4c,'untargetable',16),(0x3a65,'advance_tail',8),(0x3a67,'action_tail',8)]:self.write(0x7e0000+address,int(row[key]),width)
        for address,key,width in [(0x3ed8,'property',8),(0x3255,'ai',8),(0x32b9,'control',8),(0x3359,'seize',8),(0x3395,'charm',8),(0x3ee4,'status1',8),(0x3ee5,'status2',8),(0x3ef8,'status3',8),(0x3ef9,'status4',8),(0x3205,'update_high',8),(0x3e4d,'extra',8),(0x3aa0,'flags0',8),(0x3aa1,'flags1',8),(0x3218,'gauge',16),(0x32cc,'head',8)]:self.write(0x7e0000+address+self.x,int(row[key]),width)
        self.write(0x7e3018+self.x,1<<(int(row['slot']) if int(row['slot'])<4 else int(row['slot'])+4),16)
        for i in range(256):self.write(0x7e3720+i,255);self.write(0x7e3820+i,255)
    def allowed(self):return 0x208c5<=self.pc<0x209b4 or self.pc==0x209cd or 0x211ef<=self.pc<0x21211 or 0x211b4<=self.pc<0x211bb or 0x20792<=self.pc<0x20799 or 0x20a43<=self.pc<0x20a4a or 0x24e66<=self.pc<0x24e84 or 0x25864<=self.pc<0x2587e
    def hook(self,dest):
        if dest==0x6411:
            assert self.a&255 in [2,3]
            self.effects.append(f'{"open" if self.a&255==2 else "close"}:{self.read(0x10)}');return True
        return super().hook(dest)
    def result(self):
        hexmem=lambda address:bytes(self.read(0x7e0000+address+i) for i in range(256)).hex().upper()
        return [self.read(0x7e3aa0+self.x),self.read(0x7e3aa1+self.x),self.read(0x7e3218+self.x,16),self.read(0x7e3a65),self.read(0x7e3a67),hexmem(0x3720),hexmem(0x3820),'|'.join(self.effects)]
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert rom[0x208c5:0x209b4]==clean[0x208c5:0x209b4]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-readiness'
    subprocess.run([str(exe),'--capture-battle-readiness',str(out)],cwd=ROOT,check=True)
    with (out/'readiness.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==16384;steps=0;traces=[];events={}
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,100,1000,10000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        expected=[int(row[k]) for k in ['flags0_out','flags1_out','gauge_out','advance_tail_out','action_tail_out']]+[row[k] for k in ['advance','action','effects']]
        assert actual==expected,(i,[(k,a,b) for k,a,b in zip(['f0','f1','gauge','advance_tail','action_tail','advance','action','effects'],actual,expected) if a!=b],row)
        for effect in cpu.effects:events[effect.split(':')[0]]=events.get(effect.split(':')[0],0)+1
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    assert events.get('open',0)>0 and events.get('close',0)>0,events
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':len(rows),'target_instructions':steps,'menu_effect_counts':events,'reference_kind':'actual target pending/player readiness, CancelAction, DisableATB, queue helpers and OpenMenu gates; ExecBtlGfx intercepted; not full AfterAction2 or live menu','result':'pass'}
    (ROOT/'reverse-engineering/battle-readiness-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
