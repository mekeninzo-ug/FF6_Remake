#!/usr/bin/env python3
"""Compare Rust table decoding and ATB arithmetic with target bytes.

The bounded CalcSpeed interpreter only supports C2/09D2 and the verified
hardware-multiply subroutine; this is not SNES battle/runtime validation.
"""
from pathlib import Path
import csv, hashlib, json, subprocess
ROOT=Path(__file__).resolve().parents[1]

def calc_speed(rom, speed, status, monster, setting):
    assert rom[0x24781:0x24792]==bytes.fromhex('08 c2 20 8f 02 42 00 ea ea ea ea af 16 42 00 28 60')
    pc=0x209d2; a=0; y=0; x=8 if monster else 0
    memory={0x3ef8+x:status,0x3b19+x:speed,0x3a90:255-setting*24}
    stack=[]; carry=False; zero=False; wide=False; saved=[]
    for _ in range(80):
        op=rom[pc];pc+=1;mask=65535 if wide else 255
        if op==8:saved.append((carry,zero,wide))
        elif op==0x28:carry,zero,wide=saved.pop()
        elif op==0xa0:y=rom[pc];pc+=1;zero=y==0
        elif op in (0xbd,0xad,0x9d):
            address=int.from_bytes(rom[pc:pc+2],'little')+(x if op!=0xad else 0);pc+=2
            if op==0x9d:
                memory[address]=a&255
                if wide:memory[address+1]=a>>8
            else:a=(a&0xff00)|memory.get(address,0);zero=(a&255)==0
        elif op==0x89:zero=(a&rom[pc])==0;pc+=1
        elif op in (0xd0,0xf0,0x90):
            rel=rom[pc];pc+=1
            if (op==0xd0 and not zero) or (op==0xf0 and zero) or (op==0x90 and not carry):pc+=rel if rel<128 else rel-256
        elif op==0x98:a=(a&0xff00)|y;zero=y==0
        elif op==0x48:stack.append(a&255)
        elif op==0x68:a=(a&0xff00)|stack.pop();zero=(a&255)==0
        elif op==0x18:carry=False
        elif op==0x4a:
            low=a&mask;carry=bool(low&1);low>>=1;a=(a&~mask)|low;zero=low==0
        elif op in (0x63,0x69):
            value=stack[-rom[pc]] if op==0x63 else rom[pc];pc+=1
            result=(a&mask)+value+int(carry);carry=result>mask
            a=(a&~mask)|(result&mask);zero=(a&mask)==0
        elif op==0x83:stack[-rom[pc]]=a&255;pc+=1
        elif op==0xeb:a=((a&255)<<8)|(a>>8);zero=(a&255)==0
        elif op==0xe0:carry=x>=rom[pc];zero=x==rom[pc];pc+=1
        elif op==0x20:
            assert rom[pc:pc+2]==bytes.fromhex('81 47');pc+=2
            a=(a&255)*(a>>8) # WRMPYA/WRMPYB -> RDMPY, P restored
        elif op==0xc2:assert rom[pc]==0x20;pc+=1;wide=True
        elif op==0x60:
            assert not stack and not saved
            return memory[0x3ac8+x]|memory[0x3ac9+x]<<8
        else:raise AssertionError((hex(pc-1),hex(op)))
    raise AssertionError('Instruction budget exceeded')

def read(path):
    with path.open(newline='') as stream:return list(csv.DictReader(stream))

def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes()
    assert hashlib.sha256(rom).hexdigest()=='f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9'
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-battle-data'
    subprocess.run([str(exe),'--capture-battle-data',str(out)],cwd=ROOT,check=True)
    groups=read(out/'event-groups.csv')
    for row in groups:
        group,rng,formation=map(int,[row['group'],row['rng'],row['formation']])
        offset=0xf5000+4*group+(2 if rng>=192 else 0)
        assert formation==int.from_bytes(rom[offset:offset+2],'little')
    expected=[]
    for formation in range(576):
        data=rom[0xf6200+15*formation:0xf6200+15*(formation+1)]
        for slot in range(6):
            monster=data[2+slot]+256*bool(data[14]&(1<<slot))
            if monster==511:continue
            prop=rom[0xf0000+monster*32:0xf0020+monster*32]
            expected.append((formation,slot,monster,int(bool(data[1]&(1<<slot))),data[8+slot],
                *(int.from_bytes(prop[i:i+2],'little') for i in [8,10,12,14]),prop[0],prop[16]))
    rows=read(out/'formation-monsters.csv')
    assert [tuple(map(int,row.values())) for row in rows]==expected
    animations=read(out/'special-animations.csv')
    assert [tuple(map(int,row.values())) for row in animations]==list(enumerate(rom[0xf37c0:0xf3940]))
    speeds=read(out/'gauge-increments.csv')
    for row in speeds:
        assert int(row['increment'])==calc_speed(rom,*(int(row[k]) for k in ['speed','status','monster','battle_speed'])),row
    clock_rows=read(out/'clock-gauge.csv');previous=turns=0;gauge=0xf000;advance=0x1234;duration=7
    increment=calc_speed(rom,30,0,1,0)
    for row in clock_rows:
        frame=int(row['frame']);current=frame&255;tick=False;ready=False;enqueue=False
        if 40<=frame<60:previous=current
        elif current not in (previous,(previous+1)&255):
            tick=True;turns=(turns+1)&65535;previous=current
            if gauge>=256:
                total=gauge+increment;gauge=total&65535
                if total>65535:ready=True;gauge&=255;advance&=255;duration=255
            if frame>=400:
                total=advance+(increment>>1);advance=total&65535
                if total>65535 or advance//256>=duration:enqueue=True;duration=255
        assert tuple(map(int,row.values()))==(frame,turns,int(tick),gauge,int(ready),advance,int(enqueue)),row
    report={'special_animation_rows':len(animations),'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),
        'event_group_selections':len(groups),'formation_count':576,'decoded_monster_slots':len(rows),
        'calc_speed_target_instruction_cases':len(speeds),'clock_gauge_frames':len(clock_rows),
        'first_guard':{'formation':2,'monster':0,'slots':[2,3],'hp_each':40,'mp_each':15},
        'reference_kind':'raw ROM tables and bounded 65816 arithmetic, not emulator capture',
        'result':'pass'}
    (ROOT/'reverse-engineering/battle-data-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(report,indent=2))

if __name__=='__main__':main()
