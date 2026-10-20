#!/usr/bin/env python3
"""Copy mapped field event tables from verified ROMs; no patch application."""
from pathlib import Path
import csv,json,re,hashlib,zlib
ROOT=Path(__file__).resolve().parents[1]
def main():
 target=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes();clean=(ROOT/'Final Fantasy III (USA) [Unpatched].sfc').read_bytes()
 assert len(target)==4194304 and zlib.crc32(target)==0xfb8a31b0
 assert len(clean)==3145728 and zlib.crc32(clean)==0xa27f1c7a
 output=ROOT/'extracted/twue-3.07/build/native_events';output.mkdir(parents=True,exist_ok=True)
 records=[]
 for name,base,size in [('event_triggers',0x40000,5),('npc_properties',0x41a10,9)]:
  for rom in [clean,target]:
   ptr=[int.from_bytes(rom[base+i*2:base+i*2+2],'little') for i in range(417)]
   assert ptr[0]==834 and all(a<=b for a,b in zip(ptr,ptr[1:]))
   assert all((v-ptr[0])%size==0 for v in ptr)
  end=base+int.from_bytes(target[base+832:base+834],'little');data=target[base:end]
  (output/f'{name}.bin').write_bytes(data)
  records.append({'file':f'{name}.bin','cpu':f'0x{base+0xc00000:06X}','file_offset':f'0x{base:06X}',
                  'length':len(data),'maps':416,'records':(len(data)-834)//size,'record_size':size,
                  'changed_bytes':sum(a!=b for a,b in zip(data,clean[base:end])),
                  'sha256':hashlib.sha256(data).hexdigest().upper()})
 # Check every trigger's source coordinate/order; resolve hexadecimal source labels.
 source=(ROOT/'extracted/reference/ff6-disassembly-event/event_trigger.asm').read_text()
 map_id=None;ordinal=0;checked=resolved=0
 for line in source.splitlines():
  if match:=re.search(r'array_label EVENT_TRIGGER, (\d+)',line):map_id=int(match[1]);ordinal=0
  if match:=re.search(r'event_trigger \{(\d+), (\d+)\}, (\w+)',line):
   start=0x40000+int.from_bytes(target[0x40000+map_id*2:0x40002+map_id*2],'little')+ordinal*5
   raw=target[start:start+5];assert raw[:2]==bytes([int(match[1]),int(match[2])]);checked+=1;ordinal+=1
   if re.fullmatch(r'_c[abc][0-9a-f]{4}',match[3]):
    assert int.from_bytes(raw[2:],'little')==int(match[3][1:],16)-0xca0000;resolved+=1
 catalog=[]
 with (ROOT/'extracted/twue-3.07/provenance/source_asset_map.csv').open(newline='') as stream:
  for row in csv.DictReader(stream):
   if row['asset'].startswith('assets/gfx/map_sprite_gfx/') or row['asset'].startswith('assets/gfx/map_sprite_pal/'):
    catalog.append(('gfx' if '/map_sprite_gfx/' in row['asset'] else 'pal',row['item_index'],row['asset']))
 with (output/'sprite_catalog.csv').open('w',newline='') as stream:
  writer=csv.writer(stream);writer.writerow(['kind','id','asset']);writer.writerows(catalog)
 initial=bytearray(128)
 source=(ROOT/'extracted/reference/ff6-disassembly-field/init_npc_switch.asm').read_text()
 for bit in map(int,re.findall(r'bitlist_set (\d+)',source)):initial[bit//8]|=1<<(bit%8)
 assert clean[0xe0a0:0xe120]==target[0xe0a0:0xe120]==initial
 (output/'initial_npc_switches.bin').write_bytes(initial)
 data=target[0x11fa00:0x120000];(output/'map_init_events.bin').write_bytes(data)
 manifest={'source_commit':'813013276c952fdd27edcf7b8b86f17291542cbf','tables':records,
           'initial_npc_switches':{'cpu':'0xC0E0A0','file_offset':'0x00E0A0','length':128,'bits_set':sum(byte.bit_count() for byte in initial),'changed_bytes':0},
           'trigger_source_coordinates_checked':checked,'trigger_hexadecimal_targets_checked':resolved,
           'map_init':{'cpu':'0xD1FA00','file_offset':'0x11FA00','records':512,'length':1536},
           'limitations':'Map init data retained; special NPC, movement and full trigger script coverage remain incomplete.'}
 (output/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
 print(f'Field event tables: {checked} source trigger coordinates; {resolved} hexadecimal targets; 416 map ranges per table.')
if __name__=='__main__':main()
