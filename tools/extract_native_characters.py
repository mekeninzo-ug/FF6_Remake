#!/usr/bin/env python3
"""Read source-mapped character tables from verified clean/target ROMs."""
from pathlib import Path
import hashlib,json,re
ROOT=Path(__file__).resolve().parents[1]

def main():
    images=[]
    for name,size,digest in [('Unpatched',3145728,'0f51b4fca41b7fd509e4b8f9d543151f68efa5e97b08493e4b2a0c06f5d8d5e2'),('Patched',4194304,'f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9')]:
        data=(ROOT/f'Final Fantasy III (USA) [{name}].sfc').read_bytes()
        assert len(data)==size and hashlib.sha256(data).hexdigest()==digest;images.append(data)
    clean,target=images
    source=(ROOT/'extracted/reference/ff6-disassembly-battle/field--char_prop.asm').read_text()
    index=checks=0
    for match in re.finditer(r'^\s*empty_char_prop (\d+)|^\s*char_prop (\w+)\s*\n(.*?)^\s*end_char_prop',source,re.M|re.S):
        if match[1]:
            count=int(match[1]);assert clean[0x2d7ca0+22*index:0x2d7ca0+22*(index+count)]==bytes(22*count);index+=count;continue
        block=match[3];raw=clean[0x2d7ca0+22*index:0x2d7ca0+22*(index+1)]
        hpmp=re.search(r'set_char_prop_hp_mp (\d+), (\d+)',block)
        stats=re.search(r'set_char_prop_stats ([\d, ]+)',block)
        numeric=list(map(int,hpmp.groups())) if hpmp else [0,0]
        assert raw[:2]==bytes(numeric),(index,'hpmp');checks+=2
        numeric=list(map(int,stats[1].split(','))) if stats else [0]*9
        assert raw[6:15]==bytes(numeric),(index,'stats');checks+=9;index+=1
    assert index==64
    growth=(ROOT/'extracted/reference/ff6-disassembly-battle/data--level_up.asm').read_text()
    japanese_mp=growth.split('.else',1)[1].split('.endif',1)[0]
    japanese_values=[]
    for line in japanese_mp.splitlines():
        if m:=re.search(r'\.byte\s+([\d, ]+)',line.split(';')[0]):japanese_values.extend(map(int,m[1].split(',')))
    assert len(japanese_values)==98
    japanese_differences=[{'level':i+2,'japanese':a,'target':b} for i,(a,b) in enumerate(zip(japanese_values,target[0x26f502:0x26f564])) if a!=b]
    growth=re.sub(r'\.if LANG_EN\n(.*?)\.else\n.*?\.endif',r'\1',growth,flags=re.S)
    tables=[('character_properties',0x2d7ca0,1408,22),('level_up_exp',0x2d8220,196,2),('level_up_hp',0x26f4a0,98,1),('level_up_mp',0x26f502,98,1)]
    out=ROOT/'extracted/twue-3.07/build/native_characters';out.mkdir(parents=True,exist_ok=True)
    report={'source_commit':'813013276c952fdd27edcf7b8b86f17291542cbf','numeric_character_fields_checked':checks,'target_mp_growth_japanese_differences':japanese_differences,'tables':[]}
    for name,offset,length,stride in tables:
        if name!='character_properties':
            label={'level_up_exp':'LevelUpExp','level_up_hp':'LevelUpHP','level_up_mp':'LevelUpMP'}[name]
            body=growth.split(label+':',1)[1].split('.segment',1)[0]
            values=[]
            for line in body.splitlines():
                line=line.split(';')[0]
                if m:=re.search(r'\.(byte|word)\s+([\d, ]+)',line):values.extend(map(int,m[2].split(',')))
            expected=b''.join(v.to_bytes(stride,'little') for v in values)
            assert len(values)==98 and clean[offset:offset+length]==expected,name
        data=target[offset:offset+length];(out/f'{name}.bin').write_bytes(data)
        changes=[{'cpu':f'{0xc00000+offset+i:06X}','file_offset':f'{offset+i:06X}','clean':a,'target':b} for i,(a,b) in enumerate(zip(clean[offset:offset+length],data)) if a!=b]
        report['tables'].append({'name':name,'cpu':f'{0xc00000+offset:06X}','file_offset':f'{offset:06X}','length':length,'record_size':stride,'sha256':hashlib.sha256(data).hexdigest(),'changes':changes})
    # Exact GameStart bindings: slot/property pairs, not actor sprite identities.
    for slot,prop in [(0,0),(14,32),(15,33)]:
        assert target[0xa5e33:0xa5e8e].count(bytes([0x40,slot,prop]))==1
    report['opening_bindings']=[[0,0],[14,32],[15,33]]
    report['unchanged_initialization_code_regions']=[]
    for name,start,end in [('CalcAverageLevel region',0x9f78,0x9fdc),('EventCmd_40 region',0xa07c,0xa180),('UpdateExp tail region',0xa23e,0xa27e),('InitMaxHP and InitMaxMP',0xa27e,0xa2f9)]:
        assert clean[start:end]==target[start:end],name
        report['unchanged_initialization_code_regions'].append({'name':name,'cpu':f'{0xc00000+start:06X}','file_offset':f'{start:06X}','length':end-start,'sha256':hashlib.sha256(target[start:end]).hexdigest()})
    (out/'provenance.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(f'Character tables: {checks} numeric fields, 294 growth entries and three GameStart bindings checked; {sum(len(t["changes"]) for t in report["tables"])} target byte changes.')

if __name__=='__main__':main()
