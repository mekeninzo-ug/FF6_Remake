#!/usr/bin/env python3
"""Target-instruction checks for battle RNG, bit selection and initial gauges."""
from pathlib import Path
import csv,hashlib,json,subprocess,sys
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]

class RngCpu(EquipmentCpu):
    def __init__(self,rom,seed,entry=0x24b65,a=0,wide=False):
        super().__init__(rom,0,0,[255]*6);self.pc=entry;self.a=a;self.p=0x10 if wide else 0x30
        self.write(0xbe,seed)
    def allowed(self):return 0x24b53<=self.pc<0x24b7b or 0x251f0<=self.pc<0x25269 or 0x22575<=self.pc<0x22602
    def result(self):return self.a&65535,self.read(0xbe)

class GaugeCpu(RngCpu):
    def __init__(self,rom,row):
        super().__init__(rom,int(row['seed']),0x22575)
        self.write(0xb0,int(row['battle_flags']))
        for address,key in [(0x201f,'battle_type'),(0x3a40,'enemy_characters'),(0x3ee1,'scrolling')]:self.write(0x7e0000+address,int(row[key]))
        speeds=bytes.fromhex(row['speeds']);present=int(row['present'])
        self.before=[int(row['before'][i:i+4],16) for i in range(0,40,4)]
        for slot in range(10):
            x=slot*2
            self.write(0x7e3218+x,self.before[slot],16)
            self.write(0x7e3aa0+x,int(bool(present&(1<<slot))))
            self.write(0x7e3b19+x,speeds[slot])
            self.write(0x7e3018+x,(1<<slot) if slot<4 else (1<<(slot+4)),16)
    def result(self):return ''.join(f'{self.read(0x7e3218+slot*2,16):04X}' for slot in range(10)),self.read(0xbe)

class GuardCpu(RngCpu):
    def __init__(self,rom,seed):
        super().__init__(rom,seed,0x21a80)
        for i,value in enumerate(rom[0xf8701:0xf8704]):self.write(0x7e3a2d+i,value)
    def allowed(self):return super().allowed() or 0x21a80<=self.pc<0x21a91 or self.pc==0x21a42

def read(path):
    with path.open(newline='') as stream:return list(csv.DictReader(stream))
def main():
    result=subprocess.run([sys.executable,str(ROOT/'tools/extract_native_battle.py')],cwd=ROOT,check=True,capture_output=True,text=True)
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-rng'
    subprocess.run([str(exe),'--capture-battle-rng',str(out)],cwd=ROOT,check=True)
    rows=read(out/'bounded.csv');assert len(rows)==65536;steps=0
    for row in rows:
        cpu=RngCpu(rom,int(row['seed']),a=int(row['bound']));result,index=cpu.run();steps+=cpu.steps
        assert (result,index)==(int(row['result']),int(row['index'])),row
    bits=read(out/'bits.csv');assert len(bits)==2048
    for row in bits:
        cpu=RngCpu(rom,int(row['seed']),0x2522a,int(row['mask']),True);result,index=cpu.run();steps+=cpu.steps
        assert (result,index)==(int(row['result']),int(row['index'])),row
    sequence=read(out/'sequences.csv');assert len(sequence)==2560
    for row in sequence:
        seed=(int(row['frame'])<<2)&255;index=(seed+int(row['step'])+1)&255
        assert (int(row['seed']),int(row['index']),int(row['value']))==(seed,index,rom[0xfd00+index])
    gauges=read(out/'initial-gauges.csv');assert len(gauges)==1024;gauge_steps=0;traces=[]
    for case,row in enumerate(gauges):
        cpu=GaugeCpu(rom,row)
        if case in [1,2,3,1000]:cpu.trace=[]
        actual,index=cpu.run();gauge_steps+=cpu.steps
        assert (actual,index)==(row['after'],int(row['index'])),(row,actual,index)
        if cpu.trace is not None:traces.append({'inputs':row,'instructions':cpu.trace})
    guard=read(out/'guard-ai.csv');assert len(guard)==256;counts={}
    for row in guard:
        cpu=GuardCpu(rom,int(row['seed']));attack,index=cpu.run()
        assert (attack,index)==(int(row['attack']),int(row['index'])),row
        counts[attack]=counts.get(attack,0)+1
    assert counts=={0xee:171,0xef:85}
    (out/'initial-gauge-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    provenance=json.loads((ROOT/'extracted/twue-3.07/build/native_battle/provenance.json').read_text())
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'bounded_rng_cases':len(rows),'bit_selection_cases':len(bits),'sequence_rows':len(sequence),'initial_gauge_cases':len(gauges),'guard_ai_cases':len(guard),'guard_choice_counts':counts,'rng_bit_target_instructions':steps,'initial_gauge_target_instructions':gauge_steps,'table_comparison':[{'name':t['name'],'cpu':t['cpu'],'file_offset':t['file_offset'],'length':t['length'],'changed_bytes':t['changed_bytes'],'sha256':t['sha256']} for t in provenance['tables'] if t['name'] in ['rng_table','ai_pointers','ai_script']],'unchanged_code_regions':provenance['unchanged_rng_code_regions'],'reference_kind':'bounded actual target RNG/bit-selection/InitGauge/AIRand3 instructions and raw table; no complete battle runtime capture','result':'pass'}
    (ROOT/'reverse-engineering/battle-rng-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
