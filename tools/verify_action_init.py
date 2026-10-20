#!/usr/bin/env python3
"""Compare queued-action initialization with the actual patched Imp helper."""
from pathlib import Path
import csv,hashlib,json,subprocess,sys
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]
class ActionCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x20276;self.p=0x30|int(row['retarget']);self.x=int(row['slot'])*2;self.y=(int(row['case'])%128)*2
        self.write(0x7e3420+self.y,int(row['command'])|(int(row['attack'])<<8),16)
        self.write(0x7e3520+self.y,int(row['targets']),16);self.write(0x7e3620+self.y,int(row['cost']))
        self.write(0x7e3ee4+self.x,int(row['status1']));self.write(0x7e3a4a,int(row['changed']));self.write(0x7e3f2e,int(row['summon']));self.write(0x7e3a40,int(row['opponents']),16);self.write(0xbe,int(row['seed']))
        for slot in range(10):self.write(0x7e3018+slot*2,(1<<slot) if slot<4 else (1<<(slot+4)),16)
    def allowed(self):return 0x20276<=self.pc<0x202dc or 0x267e1<=self.pc<0x267f2 or 0x2520e<=self.pc<0x25217 or 0x2522a<=self.pc<0x25251 or 0x24b65<=self.pc<0x24b7b
    def result(self):
        assert self.read(0x7e3a7c,16)==self.read(0xb5,16)
        return self.a&255,self.read(0xb5),self.read(0xb6),self.read(0xb8,16),self.read(0x7e3a4c),self.read(0x7e3a4a),self.read(0x7e3f2e),self.read(0xbe)
def main():
    subprocess.run([sys.executable,str(ROOT/'tools/extract_native_battle.py')],cwd=ROOT,check=True,capture_output=True)
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert rom[0x202a7:0x202ab]==bytes.fromhex('20 e1 67 ea')
    assert rom[0x267e1:0x267f2]==bytes.fromhex('a5 b5 c9 19 d0 08 bd 18 30 1c 2e 3f a5 b5 c9 1e 60')
    assert clean[0x267e1:0x267f2]==b'\xff'*17
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-action-init'
    subprocess.run([str(exe),'--capture-action-init',str(out)],cwd=ROOT,check=True)
    with (out/'initialization.csv').open(newline='') as stream:rows=list(csv.DictReader(stream))
    assert len(rows)==8192;steps=0;traces=[]
    for i,row in enumerate(rows):
        cpu=ActionCpu(rom,row)
        if i in [0,25,281,1023]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        assert cpu.read(0x7e3420+cpu.y,16)==int(row['command'])|(int(row['attack'])<<8)
        assert cpu.read(0x7e3520+cpu.y,16)==int(row['targets']) and cpu.read(0x7e3620+cpu.y)==int(row['cost'])
        expected=tuple(int(row[k]) for k in ['returned','working_command','working_attack','working_targets','working_cost','after_changed','after_summon','index'])
        assert actual==expected,(i,row,actual)
        if cpu.trace is not None:traces.append({'case':i,'instructions':cpu.trace})
    (out/'target-action-init-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    provenance=json.loads((ROOT/'extracted/twue-3.07/build/native_battle/provenance.json').read_text())
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'action_initialization_cases':len(rows),'target_instructions':steps,'patched_helper_cpu':'C267E1','patched_helper_file_offset':'0267E1','code_regions':provenance['queued_action_code'],'command_property_flag_records':32,'reference_kind':'actual target queued-action/Imp helper/RNG bit-selection instructions; not full action execution','result':'pass'}
    (ROOT/'reverse-engineering/action-init-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
