#!/usr/bin/env python3
"""Compare Rust base damage with the patched 65816 routine, not vanilla formulas."""
from pathlib import Path
import csv,hashlib,json,subprocess,sys
from verify_equipment import EquipmentCpu
ROOT=Path(__file__).resolve().parents[1]

class DamageCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x22b69;self.p=0x30;self.x=8 if int(row['monster']) else 0
        for address,value in [(0x11af,int(row['level'])),(0x11a6,int(row['power'])),(0x11ae,int(row['stat'])),
                              (0x11a2,int(row['physical'])),(0x11a3,0x80 if int(row['affects_mp']) else 0),
                              (0x11a4,int(row['restorative'])),(0x3c58+self.x,int(row['relic4']))]:
            self.write(0x7e0000+address,value)
        self.write(0xb2,int(row['attack_flags']),16);self.write(0xb5,int(row['command']))
        self.write(0x7e3c44+self.x,int(row['relic_damage']),16)
    def allowed(self):
        return 0x22b69<=self.pc<0x22c21 or 0x20d4a<=self.pc<0x20d87 or 0x20dd1<=self.pc<0x20ddd or 0x247b7<=self.pc<0x247d6
    def result(self):return self.read(0x7e11b0,16)

class ModifierCpu(EquipmentCpu):
    def __init__(self,rom,row):
        super().__init__(rom,0,0,[255]*6);self.pc=0x20c9e;self.p=0x30
        self.x=8 if int(row['attacker_monster']) else 0;self.y=8 if int(row['target_monster']) else 0
        self.rng=int(row['rng'])
        self.write(0x7e11b0,int(row['base']),16)
        for address,value in [(0x11a1,int(row['element'])),(0x11a2,int(row['flags0'])),(0x11a3,int(row['flags1'])),(0x11a4,int(row['flags2'])),
                              (0x3bb8+self.y,int(row['defense'])),(0x3bb9+self.y,int(row['magic_defense'])),
                              (0x3ef8+self.y,int(row['status3'])),(0x3ef9+self.y,int(row['status4'])),
                              (0x3aa1+self.y,int(row['target_flags'])),(0x3414,int(row['enabled'])),
                              (0x3a82,0x80 if int(row['normal_defense']) else 0),(0x3a83,0x80)]:self.write(0x7e0000+address,value)
        self.write(0xb2,int(row['attack_flags']),16);self.write(0xbc,int(row['multiplier']))
    def allowed(self):return 0x20c9e<=self.pc<0x20d4a or 0x2370b<=self.pc<0x2372f or 0x247b7<=self.pc<0x247d6
    def hook(self,dest):
        if dest==0x4b5a:
            assert self.rom[0x24b5a:0x24b65]==bytes.fromhex('da e6 be a6 be bf 00 fd c0 fa 60')
            self.seta(self.rng,8);return True
        return super().hook(dest)
    def result(self):return self.read(0xf0,16)

def main():
    subprocess.run([sys.executable,str(ROOT/'tools/extract_native_magic.py')],cwd=ROOT,check=True)
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes()
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-damage'
    subprocess.run([str(exe),'--capture-damage',str(out)],cwd=ROOT,check=True)
    with (out/'base-damage.csv').open(newline='') as stream:rows=list(csv.DictReader(stream))
    assert len(rows)==8768;steps=0;traces=[]
    for index,row in enumerate(rows):
        cpu=DamageCpu(rom,row)
        if index in [0,300,700,8767]:cpu.trace=[]
        actual=cpu.run();steps+=cpu.steps
        assert actual==int(row['damage']),(index,row,actual)
        if cpu.trace is not None:traces.append({'case':index,'inputs':row,'result':actual,'instructions':cpu.trace})
    with (out/'opening-magitek-base.csv').open(newline='') as stream:magitek=list(csv.DictReader(stream))
    assert len(magitek)==12
    for row in magitek:
        raw=rom[0x46ac0+int(row['attack'])*14:0x46ace+int(row['attack'])*14]
        context={'level':row['level'],'power':raw[6],'stat':row['magic'],'physical':raw[2]&1,'monster':0,'affects_mp':bool(raw[3]&128),'restorative':raw[4]&1,'relic4':0,'relic_damage':0,'command':29,'attack_flags':0}
        assert int(row['mp_cost'])==raw[5] and int(row['base_damage'])==DamageCpu(rom,context).run()
    with (out/'damage-modifiers.csv').open(newline='') as stream:modifiers=list(csv.DictReader(stream))
    assert len(modifiers)==4096;modifier_steps=0
    for row in modifiers:
        cpu=ModifierCpu(rom,row);actual=cpu.run();modifier_steps+=cpu.steps
        assert actual==int(row['damage']),(row,actual)
    with (out/'opening-magitek-modifiers.csv').open(newline='') as stream:opening_modifiers=list(csv.DictReader(stream))
    assert len(opening_modifiers)==3072
    for row in opening_modifiers:
        raw=rom[0x46ac0+int(row['attack'])*14:0x46ace+int(row['attack'])*14]
        context={'base':row['base'],'rng':row['rng'],'flags0':raw[2],'flags1':raw[3],'flags2':raw[4],'element':raw[1],
            'attacker_monster':0,'target_monster':1,'defense':100,'magic_defense':140,'status3':0,'status4':0,'target_flags':0,'normal_defense':1,'enabled':1,'attack_flags':0,'multiplier':0}
        assert ModifierCpu(rom,context).run()==int(row['modified'])
    (out/'patched-instruction-traces.json').write_text(json.dumps(traces,indent=2)+'\n',encoding='utf-8')
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'base_damage_cases':len(rows),'base_target_instructions_executed':steps,'opening_magitek_cases':len(magitek),'modifier_cases':len(modifiers),'modifier_target_instructions_executed':modifier_steps,'opening_modifier_cases':len(opening_modifiers),'reference_kind':'bounded execution of patched C2/2B69, C2/0C9E and helpers; excludes hit checks, elemental resolution and live battle runtime','result':'pass'}
    (ROOT/'reverse-engineering/damage-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(report,indent=2))
if __name__=='__main__':main()
