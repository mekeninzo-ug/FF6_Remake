#!/usr/bin/env python3
"""Actual target Condemned, Quetzalli and near-fatal relic routines."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row,kind):
        super().__init__(rom,0,0,[255]*6);self.pc={'condemn':0x209b4,'quetzalli':0x20b4a,'near-fatal':0x20a4a}[kind];self.p=0x30;self.kind=kind;self.x=int(row.get('slot',0))*2
        if kind=='condemn':
            self.write(0x7e11af,int(row['level']));self.write(0xbe,int(row['seed']));return
        self.write(0x7e3aa0+self.x,int(row['flags']));self.write(0x7e3205+self.x,int(row['update']))
        if kind=='quetzalli':
            self.write(0x7e3018+self.x,1<<(int(row['slot']) if int(row['slot'])<4 else int(row['slot'])+4),16)
            for addr,key,width in [(0x3f2c,'jumping',16),(0x3ab4+self.x,'advance',16),(0x322c+self.x,'duration',8),(0x3a66,'head',8),(0x3a67,'tail',8)]:self.write(0x7e0000+addr,int(row[key]),width)
            for i,v in enumerate(bytes.fromhex(row['queue'])):self.write(0x7e3820+i,v)
        else:
            self.write(0x7e3ee5+self.x,int(row['status2']));self.write(0x7e3c59+self.x,int(row['relic']));self.write(0xb8,int(row['b8']),16);self.write(0xb1,int(row['b1']));self.write(0x7e340a,int(row['immediate']))
            for i,v in enumerate(bytes.fromhex(row['links'])):self.write(0x7e3184+i,v)
            raw=bytes.fromhex(row['payloads'])
            for i in range(128):
                p=raw[i*5:i*5+5];self.write(0x7e3420+i*2,int.from_bytes(p[:2],'little'),16);self.write(0x7e3520+i*2,int.from_bytes(p[2:4],'little'),16);self.write(0x7e3620+i*2,p[4])
    def write(self,address,value,width=8):
        super().write(address,value,width)
        if address==0x4202 and width==16:super().write(0x4216,(value&255)*(value>>8),16)
    def allowed(self):return 0x24781<=self.pc<0x24792 or 0x209b4<=self.pc<0x209d2 or 0x20b4a<=self.pc<0x20b83 or 0x20a4a<=self.pc<0x20a91 or 0x24b53<=self.pc<0x24b7b or 0x24e54<=self.pc<0x24f57
    def hook(self,dest):return False
    def result(self):
        h=lambda a,n:bytes(self.read(0x7e0000+a+i) for i in range(n)).hex().upper()
        if self.kind=='condemn':return [self.read(0x7e3b05+self.x),self.read(0xbe)]
        if self.kind=='quetzalli':return [self.read(0x7e3f2c,16),self.read(0x7e3aa0+self.x),self.read(0x7e3205+self.x),self.read(0x7e3ab4+self.x,16),self.read(0x7e322c+self.x),h(0x3820,256)]
        payloads=bytearray()
        for i in range(128):payloads.extend(self.read(0x7e3420+i*2,16).to_bytes(2,'little'));payloads.extend(self.read(0x7e3520+i*2,16).to_bytes(2,'little'));payloads.append(self.read(0x7e3620+i*2))
        return [self.read(0x7e3205+self.x),self.read(0xb8,16),self.read(0xb1),h(0x3184,128),self.read(0x7e340a),payloads.hex().upper()]
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes();assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    for lo,hi in [(0x209b4,0x209d2),(0x20a4a,0x20a91),(0x20b4a,0x20b83)]:assert rom[lo:hi]==clean[lo:hi]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-after-effects';subprocess.run([str(exe),'--capture-battle-after-effects',str(out)],cwd=ROOT,check=True)
    steps=0;counts={};traces=[]
    for kind,keys,count in [('condemn',['countdown','index'],65536),('quetzalli',['jumping_out','flags_out','update_out','advance_out','duration_out','queue_out'],2048),('near-fatal',['update_out','b8_out','b1_out','links_out','immediate_out','payloads_out'],4096)]:
        with (out/(kind+'.csv')).open(newline='') as f:rows=list(csv.DictReader(f))
        assert len(rows)==count;counts[kind]=count
        for i,row in enumerate(rows):
            cpu=Cpu(rom,row,kind)
            if i in [0,100,1000]:cpu.trace=[]
            actual=cpu.run();steps+=cpu.steps;expected=[row[k] if k in ['queue_out','links_out','payloads_out'] else int(row[k]) for k in keys]
            assert actual==expected,(kind,i,[(k,a,b) for k,a,b in zip(keys,actual,expected) if a!=b],row)
            if cpu.trace is not None:traces.append({'kind':kind,'case':i,'instructions':cpu.trace})
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'cases':counts,'target_instructions':steps,'reference_kind':'actual target StartCondemn, QuetzEffect and NearFatalEffect including RNG, command allocation and immediate-action storage; no hooks; not full AfterAction2','result':'pass'}
    (ROOT/'reverse-engineering/battle-after-effects-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
