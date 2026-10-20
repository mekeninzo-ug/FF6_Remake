#!/usr/bin/env python3
"""Bounded 65816 patch checks plus native player motion and pixel comparisons."""
from pathlib import Path
import csv
import hashlib
import json
import subprocess
import sys
from PIL import Image,ImageChops,ImageDraw
from verify_magitek import tiles,palette,blit
from verify_field_traversal import exits_from_rom
ROOT=Path(__file__).resolve().parents[1]

def read(path):
    with path.open(newline='',encoding='utf-8') as stream:return list(csv.DictReader(stream))

def execute_patch(rom,pc,a,memory):
    """Only the mapped A8/Y16/DBR0 routines; no CPU/system emulation elsewhere."""
    stack=[];zero=False;trace=[]
    for _ in range(128):
        start=pc;op=rom[pc];pc+=1
        if op==0xa9:a=rom[pc];pc+=1;zero=a==0
        elif op==0xa5:a=memory.get(rom[pc],0);pc+=1;zero=a==0
        elif op in (0xad,0xb9):
            address=int.from_bytes(rom[pc:pc+2],'little');pc+=2
            a=memory.get(address,0);zero=a==0 # Y=0 selects the supplied object
        elif op==0x48:stack.append(a)
        elif op==0x68:a=stack.pop();zero=a==0
        elif op==0xea:pass
        elif op==0x29:a&=rom[pc];pc+=1;zero=a==0
        elif op==0x1a:a=(a+1)&255;zero=a==0
        elif op==0x3a:a=(a-1)&255;zero=a==0
        elif op==0x4a:a>>=1;zero=a==0
        elif op in (0xf0,0xd0):
            relative=rom[pc];pc+=1
            if (op==0xf0 and zero) or (op==0xd0 and not zero):pc+=relative if relative<128 else relative-256
        elif op==0x60:
            assert not stack
            trace.append({'file_offset':f'0x{start:06X}','cpu':f'0x{start+0xc00000:06X}','opcode':f'0x{op:02X}'})
            return a,trace
        else:raise AssertionError(f'Unexpected 65816 opcode {op:02X} at {start:06X}')
        trace.append({'file_offset':f'0x{start:06X}','cpu':f'0x{start+0xc00000:06X}','opcode':f'0x{op:02X}'})
    raise AssertionError('Patch instruction budget exceeded')

def main():
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes()
    clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
    assert rom[0x4e21:0x4e26]==bytes.fromhex('20 74 d6 80 0f')
    assert rom[0x4e28:0x4e2d]==bytes.fromhex('20 74 d6 80 08')
    assert rom[0x5885:0x5888]==rom[0x5892:0x5895]==bytes.fromhex('20 8d d6')
    assert rom[0x589d:0x589f]==bytes.fromhex('29 03')
    assert clean[0xd674:0xd69f]==b'\xff'*(0xd69f-0xd674)
    assert rom[0x7eca:0x7ed4]==clean[0x7eca:0x7ed4]==bytes([1,2,4,8,16,32,16,8,4,2])
    assert rom[0x7ed4:0x7edc]==clean[0x7ed4:0x7edc]==bytes.fromhex('00 00 40 00 00 00 40 00')
    assert rom[0x7f14:0x7f1c]==clean[0x7f14:0x7f1c]==bytes.fromhex('40 00 00 00 40 00 00 00')
    executable=ROOT/'target/release/ff6-native.exe'
    output=ROOT/'reverse-engineering/captures/native-player-motion'
    subprocess.run([str(executable),'--capture-player-motion',str(output)],cwd=ROOT,check=True)
    speed_rows=read(output/'speeds.csv')
    native_speeds={(int(row['cancel']),int(row['sprint'])):int(row['speed']) for row in speed_rows}
    examples=[]
    for buttons in range(256):
        for flags in (0,0x20):
            speed,trace=execute_patch(rom,0xd674,0,{7:buttons,0x11df:flags})
            assert speed==native_speeds[(int(bool(buttons&128)),int(bool(flags&32)))]
            if buttons in (0,128):examples.append({'buttons':buttons,'sprint_flags':flags,'speed':speed,'trace':trace})
    for row in speed_rows:
        speed,frame=int(row['speed']),int(row['frame']);rate=64*rom[0x7eca+speed]
        completed=frame*rate==4096
        assert int(row['completed'])==int(completed)
        assert int(row['progress'])==(0 if completed else frame*rate)
        assert int(row['offset_y'])==(0 if completed else -frame*rate)
    gait_rows=read(output/'gait.csv')
    assert len(gait_rows)==1024
    for row in gait_rows:
        a,_=execute_patch(rom,0xd68d,int(row['coordinate']),{0x87f:int(row['direction'])})
        assert (a&3)==int(row['phase'])
    game_rows=read(output/'game-motion.csv');scenarios={}
    for row in game_rows:scenarios.setdefault(row['scenario'],[]).append(row)
    for name,records in scenarios.items():
        initial=records[0];x,y=int(initial['x']),int(initial['y'])
        horizontal=name.endswith('right');blocked=name.startswith('blocked');dash=name.startswith('dash')
        rate=2 if dash else 1;frames=8 if dash else 16
        expected_final=12 if dash else 20
        assert [int(row['frame']) for row in records]==list(range(expected_final+1))
        for row in records:
            frame=int(row['frame']);distance=0 if blocked else min(max(frame-3,0)*rate,16)
            expected_x=x*16+(distance if horizontal else 0)
            expected_y=y*16-(distance if not horizontal else 0)
            if name=='exit-up' and frame==20:
                exits=[exit for exit in exits_from_rom(rom,19) if exit['y']==1 and exit['x']<=38<=exit['x']+exit['extent'] and exit['map']==39]
                assert len(exits)==1
                expected_x,expected_y=exits[0]['dest_x']*16,exits[0]['dest_y']*16
                assert int(row['map'])==39
            else:assert int(row['map'])==19
            assert (int(row['world_x']),int(row['world_y']))==(expected_x,expected_y),(name,frame)
            assert int(row['steps'])==int(frame>=4 and not blocked)
            assert int(row['active'])==int(not blocked and 4<=frame<3+frames)
            assert int(row['pending_exit'])==int(not blocked and frame==3+frames)
    vehicle=tiles(rom[0x183000:0x184c00]);vp=palette(rom[0x2680e0:0x268100])
    rider=tiles(rom[0x150000:0x1516a0]);rp=palette(rom[0x268040:0x268060])
    formations=[int.from_bytes(rom[p:p+2],'little') for p in range(0x5eb6,0x5f36,2)]
    frames=[int.from_bytes(rom[p:p+2],'little')//32 for p in range(0xce3a,0xd0f2,2)]
    comparisons=[]
    for row in game_rows:
        if not row['capture']:continue
        map_id=int(row['map']);wx,wy=int(row['world_x']),int(row['world_y']);facing=int(row['facing'])
        bundle=ROOT/f'extracted/twue-3.07/build/native_field/map-{map_id:04}'
        ref=output/(Path(row['capture']).stem+'-reference.png')
        subprocess.run([sys.executable,str(ROOT/'tools/render_native_field.py'),'--map',str(bundle),'--pixel-x',str(wx),'--pixel-y',str(wy),
                        '--scale','1','--output',str(ref)],cwd=ROOT,check=True,stdout=subprocess.DEVNULL)
        expected=Image.open(ref).convert('RGB');active=row['active']=='1';horizontal=active and facing in (1,3)
        coordinate=wx if horizontal else wy;phase=(coordinate>>3)&3
        cx=min(max(wx+8-128,0),64*16-256);cy=min(max(wy+8-112,0),64*16-224)
        x,y=wx-cx,wy-cy
        for piece in range(4):
            word=formations[piece*16+facing*4+phase];base=(word&511)-0x120
            blit(expected,vehicle,vp,[base,base+1,base+16,base+17],x+(piece%2)*16,y-24+(piece//2)*16,bool(word&0x4000),bool(word&0x8000))
        rider_phase=phase if active else 0
        frame_id=([4,5,4,3] if facing==0 else [1,2,1,0] if facing==2 else [0x2e,0x2f,0x2e,0x2f])[rider_phase]
        blit(expected,rider,rp,frames[frame_id*6:frame_id*6+4],x+8,y-30+((coordinate>>3)&1),facing==1)
        native=Image.open(output/row['capture']).convert('RGB')
        changed=sum(value!=(0,0,0) for value in ImageChops.difference(native,expected).getdata())
        assert changed==0,(row['capture'],changed)
        native.save(output/(Path(row['capture']).stem+'.png'));expected.save(ref)
        comparisons.append({'capture':row['capture'],'pixels':57344,'different_pixels':changed})
    sheet=Image.new('RGB',(512,480),'#151b25');draw=ImageDraw.Draw(sheet)
    for index,frame in enumerate((4,8,12,16)):
        x,y=index%2*256,index//2*240
        sheet.paste(Image.open(output/f'normal-up-{frame:02}.bmp'),(x,y));draw.text((x+4,y+226),f'Player walk / frame {frame}',fill='white')
    sheet.save(output/'contact-sheet.png')
    report={'executable_sha256':hashlib.sha256(executable.read_bytes()).hexdigest().upper(),
            'patched_rom_sha256':hashlib.sha256(rom).hexdigest().upper(),
            'dash_patch':{'cpu':'0xC0D674','file_offset':'0x00D674','bytes':rom[0xd674:0xd68d].hex(' '),'input_cases_checked':512,'examples':examples},
            'gait_patch':{'cpu':'0xC0D68D','file_offset':'0x00D68D','bytes':rom[0xd68d:0xd69f].hex(' '),'input_cases_checked':1024},
            'native_speed_rows':len(speed_rows),'game_motion_rows':len(game_rows),
            'scenarios':{name:len(rows) for name,rows in scenarios.items()},'frame_comparisons':comparisons,
            'emulator_reference':False,
            'limitations':['Bounded A8/Y16/DBR0 interpretation of mapped 65816 patch routines only','Nominal 60 Hz field clock',
                           'Sprint Shoes effect verified in core but not connected to equipment','No diagonal/bridge-complete collision, NPC/event/encounter or PPU timing validation']}
    (ROOT/'reverse-engineering/player-movement-validation.json').write_text(json.dumps(report,indent=2)+'\n')
    print(f'Player movement passed: 512 raw-ROM speed inputs, 1024 gait inputs, {len(game_rows)} game states and {len(comparisons)} full frame comparisons.')
if __name__=='__main__':main()
