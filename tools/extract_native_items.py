#!/usr/bin/env python3
"""Extract the target ItemProp table, validating source layout and identities."""
from pathlib import Path
import hashlib,json,re
ROOT=Path(__file__).resolve().parents[1]
def main():
    images=[]
    for name,size,digest in [('Unpatched',3145728,'0f51b4fca41b7fd509e4b8f9d543151f68efa5e97b08493e4b2a0c06f5d8d5e2'),('Patched',4194304,'f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9')]:
        data=(ROOT/f'Final Fantasy III (USA) [{name}].sfc').read_bytes()
        assert len(data)==size and hashlib.sha256(data).hexdigest()==digest;images.append(data)
    clean,target=images;offset=0x185000;size=30*256
    source=(ROOT/'extracted/reference/ff6-disassembly-battle/data--item_prop.asm').read_text()
    blocks=re.findall(r'^\s*item_prop (\w+), (\w+)\s*\n(.*?)^\s*end_item_prop',source,re.M|re.S)
    assert len(blocks)==256;checks=0
    for index,(_,kind,block) in enumerate(blocks):
        raw=clean[offset+index*30:offset+(index+1)*30]
        for names,field,width in [('price',28,2),('attack_power|def_power|item_power',20,1),('hit_rate|mag_def',21,1)]:
            if kind=='CONSUMABLE' and field==21:continue
            match=re.search(r'^\s*(?:'+names+r')\s+(\d+)\s*$',block,re.M)
            value=int(match[1]) if match else 0
            assert int.from_bytes(raw[field:field+width],'little')==value,(index,names,value,raw[field]);checks+=1
        for field,name in enumerate(['strength','speed','stamina','mag_pwr']):
            match=re.search(r'^\s*'+name+r'\s+([+-]?\d+)\s*$',block,re.M)
            value=int(match[1]) if match else 0
            encoded=value if value>=0 else 8-value
            assert ((int.from_bytes(raw[16:18],'little')>>(field*4))&15)==encoded,(index,name);checks+=1
    assert clean[0x20e77:0x2111b]==target[0x20e77:0x2111b]
    field_code=bytes.fromhex('9c df 11 a4 00 64 1b b9 67 08 29 40 f0 13 b9 67 08 29 07 cd 6d 1a d0 09 5a a5 1b 22 06 00 c2 7b 7a c2 21 98 69 29 00 a8 7b e2 20 e6 1b c0 90 02 d0 d5 60')
    assert clean[0x6cf3:0x6cf3+len(field_code)]==target[0x6cf3:0x6cf3+len(field_code)]==field_code
    evade=[0,10,20,30,40,50,-10,-20,-30,-40,-50]
    assert target[0x21105:0x2111b]==b''.join((v&65535).to_bytes(2,'little') for v in evade)
    data=target[offset:offset+size]
    out=ROOT/'extracted/twue-3.07/build/native_items';out.mkdir(parents=True,exist_ok=True)
    (out/'item_properties.bin').write_bytes(data)
    report={'source_commit':'813013276c952fdd27edcf7b8b86f17291542cbf','cpu':'D85000','file_offset':'185000','length':size,'record_size':30,'numeric_source_fields_checked':checks,'sha256':hashlib.sha256(data).hexdigest(),
        'changes':[{'cpu':f'{0xc00000+offset+i:06X}','file_offset':f'{offset+i:06X}','item':i//30,'byte':i%30,'clean':a,'target':b} for i,(a,b) in enumerate(zip(clean[offset:offset+size],data)) if a!=b],
        'unchanged_equipment_code_region':{'cpu':'C20E77','file_offset':'020E77','length':0x2111b-0x20e77},
        'unchanged_field_aggregation':{'cpu':'C06CF3','file_offset':'006CF3','length':len(field_code)}}
    (out/'provenance.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(f'Item extraction: {checks} numeric source fields; {len(report["changes"])} changed target bytes; equipment routine unchanged.')
if __name__=='__main__':main()
