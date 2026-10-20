#!/usr/bin/env python3
"""Source-encode two bounded field scenes and compare actual native trigger traces."""
from pathlib import Path
import csv,hashlib,json,re,subprocess,sys
from PIL import Image,ImageDraw
import verify_opening_event as opening
ROOT=Path(__file__).resolve().parents[1]
opening.OBJECTS.update({'NPC_1':16,'NPC_2':17})

def read(path):
 with path.open(newline='',encoding='utf-8') as stream:return list(csv.DictReader(stream))
def encode(label,stop,battle_stop=False):
 source=(ROOT/'extracted/reference/ff6-disassembly-event/event_main.asm').read_text().splitlines()
 first=source.index(label+':');end=source.index(stop+':',first);entry=int(label[1:],16)-0xca0000
 encoded=bytearray();units=[];pending=None;body=[]
 def emit(data,line,obj=None):
  units.append({'offset':entry+len(encoded),'opcode':data[0],'length':len(data),'object':obj,'source_line':line});encoded.extend(data)
 for line_no in range(first+1,end):
  line=source[line_no].split(';',1)[0].strip()
  if not line:continue
  if pending:
   body.extend((line_no+1,data) for data in opening.object_command(line))
   if line=='end':
    obj,asynchronous,header=pending;length=sum(len(data) for _,data in body)
    emit(bytes([obj,length|(0 if asynchronous else 128)]),header)
    for line_no2,data in body:emit(data,line_no2,obj)
    body=[];pending=None
  elif match:=re.fullmatch(r'obj_script (\w+)(, ASYNC)?',line):pending=(opening.OBJECTS[match[1]],bool(match[2]),line_no+1)
  else:
   if match:=re.fullmatch(r'create_obj (\w+)',line):data=bytes([0x3d,opening.OBJECTS[match[1]]])
   elif match:=re.fullmatch(r'if_switch \$([0-9A-Fa-f]+)=(0|1), EventReturn',line):
    data=b'\xc0'+opening.word(int(match[1],16)|(int(match[2])<<15))+b'\xb3\x5e\x00'
   elif match:=re.fullmatch(r'dlg DLG_(\d+), BOTTOM',line):data=b'\x4b'+opening.word(int(match[1])|0x8000)
   elif match:=re.fullmatch(r'battle (\d+)',line):data=bytes([0x4d,int(match[1]),0x3f])
   else:data=opening.event_command(line,{})
   emit(data,line_no+1)
   if battle_stop and line.startswith('battle '):break
 assert pending is None
 return bytes(encoded),units

def main():
 subprocess.run([sys.executable,str(ROOT/'tools/extract_native_events.py')],cwd=ROOT,check=True)
 executable=ROOT/'target/release/ff6-native.exe';trace=ROOT/'reverse-engineering/captures/native-field-events';captures=ROOT/'reverse-engineering/captures/native-trigger-scenes'
 subprocess.run([str(executable),'--trace-field-events',str(trace)],cwd=ROOT,check=True)
 subprocess.run([str(executable),'--capture-trigger-scenes',str(captures)],cwd=ROOT,check=True)
 target=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
 commands=read(trace/'commands.csv');snapshots=read(trace/'snapshots.csv');records=[]
 actors=read(trace/'actors.csv');bindings=[]
 for actor,graphic,palette in [(0,0,2),(14,14,1),(15,14,1)]:
  pattern=bytes([0x3d,actor,0x3f,actor,1,0x37,actor,graphic,0x43,actor,palette])
  start=0xa5e33;block=target[start:0xa5e8e];assert block.count(pattern)==1 and clean[start:0xa5e8e].count(pattern)==1
  pc=start+block.index(pattern)
  bindings.append({'actor':actor,'graphic':graphic,'palette':palette,'party':1,'cpu':f'0x{pc+0xc00000:06X}','file_offset':f'0x{pc:06X}','bytes':pattern.hex(' ')})
  for row in actors:
   if int(row['object'])==actor:assert (int(row['graphic']),int(row['palette']),int(row['character_party']))==(graphic,palette,1)
 for name,label,stop,battle in [('backtrack','_cc9b1d','_cc9b71',False),('guards','_cc9b71','_cc9bb3',True)]:
  encoded,units=encode(label,stop,battle);entry=int(label[1:],16)-0xca0000;pc=entry+0xa0000
  assert target[pc:pc+len(encoded)]==clean[pc:pc+len(encoded)]==encoded
  expected={entry['offset']:entry for entry in units};actual=[entry for entry in commands if entry['scenario']==name]
  assert len(actual)==len(units)
  seen=set()
  for row in actual:
   address=int(row['event_offset'],16);unit=expected[address];assert address not in seen;seen.add(address)
   assert int(row['cpu'],16)==address+0xca0000 and int(row['file_offset'],16)==address+0xa0000
   assert (int(row['opcode'],16),int(row['length']))==(unit['opcode'],unit['length'])
   assert row['object']==('' if unit['object'] is None else str(unit['object']))
   if row['object']:assert int(row['frame'])%4==0
   if row['object'] and unit['opcode'] in (0xd5,0xd7):
    following=next(item for item in actual[actual.index(row)+1:] if item['object']==row['object'])
    assert int(following['frame'])-int(row['frame'])==4
  records.append({'scenario':name,'cpu':f'0x{entry+0xca0000:06X}','file_offset':f'0x{pc:06X}','bytes':len(encoded),'native_command_spans':len(actual),'source_units':units})
 gate=[row for row in commands if row['scenario']=='gated']
 assert [(int(row['event_offset'],16),int(row['opcode'],16)) for row in gate]==[(0x029b71,0xc0),(0x005eb3,0xfe)]
 assert target[0xa5eb3]==clean[0xa5eb3]==0xfe
 expected_stages=[('backtrack','dialogue','12',''),('backtrack','returned','',''),('guards','dialogue','13',''),('guards','battle','','1'),('gated','returned','','')]
 assert [(row['scenario'],row['stage'],row['dialogue'],row['battle']) for row in snapshots]==expected_stages
 assert (snapshots[0]['wedge_y'],snapshots[0]['vicks_x'],snapshots[0]['vicks_y'])==('52','40','51')
 assert (snapshots[1]['terra_y'],snapshots[1]['party'])==('49','0:255:255:255')
 assert (snapshots[2]['npc1_x'],snapshots[2]['npc1_y'],snapshots[2]['npc2_x'],snapshots[2]['npc2_y'])==('37','35','39','35')
 assert snapshots[3]['flag_0128']=='0' and snapshots[4]['flag_0128']=='1'
 game=read(captures/'states.csv');assert len(game)==4
 assert (game[0]['dialogue'],game[0]['party'],game[1]['y'])==('12','0:255:255:255','49')
 assert (game[2]['dialogue'],game[3]['battle'],game[3]['flag_0128'])==('13','1','0')
 assert game[3]['event_cpu']=='0xCC9BA4'
 diffs=[];base=0x41a10
 assert clean[base:base+834]==target[base:base+834]
 for map_id in range(416):
  start=base+int.from_bytes(target[base+map_id*2:base+map_id*2+2],'little');end=base+int.from_bytes(target[base+map_id*2+2:base+map_id*2+4],'little')
  for index,pc in enumerate(range(start,end,9)):
   changes=[{'field_byte':i,'clean':a,'patched':b} for i,(a,b) in enumerate(zip(clean[pc:pc+9],target[pc:pc+9])) if a!=b]
   if changes:diffs.append({'map':map_id,'npc':index+1,'cpu':f'0x{pc+0xc00000:06X}','file_offset':f'0x{pc:06X}','changes':changes})
 assert sum(len(row['changes']) for row in diffs)==6
 (ROOT/'reverse-engineering/npc-property-differences.json').write_text(json.dumps(diffs,indent=2)+'\n')
 sheet=Image.new('RGB',(512,480),'#151b25');draw=ImageDraw.Draw(sheet)
 for i,row in enumerate(game):
  image=Image.open(captures/f"{row['stage']}.bmp");image.save(captures/f"{row['stage']}.png")
  x,y=i%2*256,i//2*240;sheet.paste(image,(x,y));draw.text((x+4,y+226),row['stage'],fill='white')
 sheet.save(captures/'contact-sheet.png')
 report={'executable_sha256':hashlib.sha256(executable.read_bytes()).hexdigest().upper(),'source_commit':'813013276c952fdd27edcf7b8b86f17291542cbf',
         'game_start_actor_bindings':bindings,'actor_states':actors,'source_encodings':records,'gated_return_commands':gate,'vm_snapshots':snapshots,'actual_walk_trigger_captures':game,
         'snapshot_roundtrips':'Native capture asserts byte-identical VM/actor snapshots at DLG_12 and battle-group-1 boundary',
         'npc_property_changed_bytes':6,'npc_property_changed_records':len(diffs),'emulator_reference':False,
         'limitations':['First guard scene stops at battle request; no battle or victory is simulated','General script, startup, NPC interaction and collision coverage incomplete',
                        'Full game initialization/NPC switches, original timing and PPU presentation unvalidated']}
 (ROOT/'reverse-engineering/field-events-validation.json').write_text(json.dumps(report,indent=2)+'\n')
 print(f"Field events passed: {sum(row['bytes'] for row in records)} source-encoded bytes, {len(commands)} VM commands, four actual walk-trigger scene captures.")
if __name__=='__main__':main()
