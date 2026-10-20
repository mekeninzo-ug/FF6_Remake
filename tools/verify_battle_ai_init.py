#!/usr/bin/env python3
"""Execute target InitAI and FindAIScriptEnd against native script boundaries."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row,scan=False):
        super().__init__(rom,0,0,[255]*6);self.scan=scan;self.row=row
        self.pc=0x21a43 if scan else 0x22d71;self.p=0x20 if scan else 0
        self.x=int(row['start']) if scan else 0x1234;self.y=(int(row['monster'])%10)*2;self.a=0 if scan else int(row['monster'])
        if not scan:self.write(0x7e3268+self.y,int(row['previous']),16)
    def allowed(self):return 0x21a42<=self.pc<0x21a5f or 0x22d71<=self.pc<0x22d99
    def result(self):
        if self.scan:return self.a&255,self.x
        assert self.x==0x1234 and self.a==int(self.row['monster'])
        return self.read(0x7e3254+self.y,16),self.read(0x7e3268+self.y,16)
def rows(path):
    with path.open(newline='') as f:return list(csv.DictReader(f))
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    for lo,hi in [(0x21a42,0x21a5f),(0x22d71,0x22d99),(0x21daf,0x21dbf)]:assert rom[lo:hi]==clean[lo:hi]
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-ai-init'
    subprocess.run([str(exe),'--capture-battle-ai-init',str(out)],cwd=ROOT,check=True)
    init=rows(out/'initialization.csv');blocks=rows(out/'normal-blocks.csv');assert len(init)==768
    steps=0;traces=[];without=0
    for i,row in enumerate(init):
        cpu=Cpu(rom,row)
        if i in [0,20,100,700]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        assert actual==(int(row['normal']),int(row['retaliation'])),(i,row,actual)
        if int(row['previous'])==65535 and actual[1]==65535:without+=1
        if cpu.trace is not None:traces.append({'kind':'init','case':i,'instructions':cpu.trace})
    for i,row in enumerate(blocks):
        cpu=Cpu(rom,row,True);actual=cpu.run();steps+=cpu.steps
        assert actual==(int(row['terminator']),int(row['next'])),(i,row,actual)
    # Compare complete decoded normal and retaliation streams, separating
    # changed script content from pointers merely moved by patch layout.
    def stream(image,start):
        ptr=start
        while True:
            end,next_ptr=Cpu(image,{'monster':0,'start':ptr},True).run();ptr=next_ptr
            if end==255:return image[0xf8700+start:0xf8700+ptr]
    changed_normal=[];changed_retaliation=[];relocated=[];comparisons=[]
    for monster in range(384):
        clean_normal,clean_retal=Cpu(clean,{'monster':monster,'previous':65535}).run()
        target_normal,target_retal=Cpu(rom,{'monster':monster,'previous':65535}).run()
        normal_a,normal_b=stream(clean,clean_normal),stream(rom,target_normal)
        retal_a=b'' if clean_retal==65535 else stream(clean,clean_retal)
        retal_b=b'' if target_retal==65535 else stream(rom,target_retal)
        if normal_a!=normal_b:changed_normal.append(monster)
        if retal_a!=retal_b:changed_retaliation.append(monster)
        if clean_normal!=target_normal:relocated.append(monster)
        comparisons.append({'monster':monster,'clean_normal_cpu':f'{0xcf8700+clean_normal:06X}','target_normal_cpu':f'{0xcf8700+target_normal:06X}','clean_normal_file':f'{0xf8700+clean_normal:06X}','target_normal_file':f'{0xf8700+target_normal:06X}','clean_retaliation_offset':clean_retal,'target_retaliation_offset':target_retal,'normal_changed':normal_a!=normal_b,'retaliation_changed':retal_a!=retal_b,'clean_normal_sha256':hashlib.sha256(normal_a).hexdigest(),'target_normal_sha256':hashlib.sha256(normal_b).hexdigest(),'clean_retaliation_sha256':hashlib.sha256(retal_a).hexdigest(),'target_retaliation_sha256':hashlib.sha256(retal_b).hexdigest()})
    (out/'version-script-comparison.json').write_text(json.dumps(comparisons,indent=2)+'\n',encoding='utf-8')
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'initialization_cases':len(init),'block_scan_cases':len(blocks),'monsters_without_retaliation':without,'target_instructions':steps,'normal_script_content_changed':changed_normal,'retaliation_script_content_changed':changed_retaliation,'normal_pointer_changed_count':len(relocated),'reference_kind':'actual target InitAI and FindAIScriptEnd, all 384 monster pointers; no hooks; decoded stream bytes compared to clean; does not execute AI conditions or actions','result':'pass'}
    (ROOT/'reverse-engineering/battle-ai-init-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
