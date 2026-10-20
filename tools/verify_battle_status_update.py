#!/usr/bin/env python3
"""Complete actual patched UpdateStatus and all set/remove handlers, no hooks."""
from pathlib import Path
import csv,hashlib,json,subprocess
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
FIELDS=[(0x3ee4,2),(0x3ef8,2),(0x3dd4,2),(0x3de8,2),(0x3dfc,2),(0x3e10,2),(0x331c,2),(0x3330,2),(0x3bf4,2),(0x3c1c,2),(0x3e4d,2),(0x32df,2),(0x3e60,2),(0x3e74,2),(0x3aa0,2),(0x3204,2),(0x3cf9,1),(0x3af1,1),(0x3f0c,1),(0x3f0d,1)]
GLOBAL=[0x2f53,0x3a4a,0x2f44,0x2f2f,0x3a3a,0x3a56]
class Cpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x24391;self.p=0x30;self.x=12;self.handler_entries=set()
        raw=bytes.fromhex(row['actors']);assert len(raw)==360
        for slot in range(10):
            offset=slot*36;self.write(0x7e3018+slot*2,1<<(slot if slot<4 else slot+4),16)
            for address,size in FIELDS:
                value=int.from_bytes(raw[offset:offset+size],'little');offset+=size
                self.write(0x7e0000+address+slot*2,value,8 if address==0x3e4d else size*8)
        raw=bytes.fromhex(row['globals']);assert len(raw)==17
        for i,address in enumerate(GLOBAL):self.write(0x7e0000+address,int.from_bytes(raw[i*2:i*2+2],'little'),8 if address==0x2f2f else 16)
        for i,v in enumerate(raw[12:]):self.write(0x7e2f30+i,v)
    def read(self,address,width=8):
        if width==16 and 0xc246b0<=address<0xc24730 and hasattr(self,'handler_entries'):self.handler_entries.add((address-0xc246b0)//2)
        return super().read(address,width)
    def allowed(self):return 0x24391<=self.pc<0x24406 or 0x2447f<=self.pc<0x246b0
    def result(self):
        assert self.x==12
        actors=bytearray()
        for slot in range(10):
            for address,size in FIELDS:actors.extend(self.read(0x7e0000+address+slot*2,8 if address==0x3e4d else size*8).to_bytes(size,'little'))
        globals=bytearray()
        for address in GLOBAL:globals.extend(self.read(0x7e0000+address,8 if address==0x2f2f else 16).to_bytes(2,'little'))
        globals.extend(self.read(0x7e2f30+i) for i in range(5))
        return actors.hex(),globals.hex()
def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    assert hashlib.sha256(clean).hexdigest()=='0f51b4fca41b7fd509e4b8f9d543151f68efa5e97b08493e4b2a0c06f5d8d5e2'
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-status-update'
    subprocess.run([str(exe),'--capture-battle-status-update',str(out)],cwd=ROOT,check=True)
    with (out/'update.csv').open(newline='') as f:rows=list(csv.DictReader(f))
    assert len(rows)==2688;steps=0;traces=[];handlers=set();baseline_differences=[]
    for i,row in enumerate(rows):
        cpu=Cpu(rom,row)
        if i in [0,100,1000]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps;handlers.update(cpu.handler_entries)
        assert actual==(row['actors_out'].lower(),row['globals_out'].lower()),(i,actual,row['actors_out'],row['globals_out'])
        if i>=2048:
            baseline=Cpu(clean,row).run()
            if baseline!=actual:baseline_differences.append({'case':i,'slot':(i-2048)//64,'status_id':((i-2048)%64)//2,'set':bool((i-2048)%2),'clean_actors':baseline[0],'target_actors':actual[0],'clean_globals':baseline[1],'target_globals':actual[1]})
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    assert handlers==set(range(64)),sorted(set(range(64))-handlers)
    (out/'version-handler-differences.json').write_text(json.dumps(baseline_differences,indent=2)+'\n',encoding='utf-8')
    (out/'target-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    changed=[f'{i:06X}' for i in range(0x24391,0x24730) if clean[i]!=rom[i]]
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'ten_actor_cases':len(rows),'controlled_single_status_cases':640,'controlled_cases_different_from_clean':len(baseline_differences),'dispatch_entries_exercised':len(handlers),'target_instructions':steps,'changed_code_or_table_offsets':changed,'reference_kind':'complete actual patched UpdateStatus and all dispatched status handlers, preparation, resolution and reset; no hooks; excludes downstream consumers of flags/timers','result':'pass'}
    (ROOT/'reverse-engineering/battle-status-update-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
