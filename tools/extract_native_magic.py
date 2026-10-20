#!/usr/bin/env python3
"""Export source-mapped target attack properties and retain patched damage evidence."""
from pathlib import Path
import hashlib,json,re
ROOT=Path(__file__).resolve().parents[1]
def main():
    images=[]
    for name,size,digest in [('Unpatched',3145728,'0f51b4fca41b7fd509e4b8f9d543151f68efa5e97b08493e4b2a0c06f5d8d5e2'),('Patched',4194304,'f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9')]:
        data=(ROOT/f'Final Fantasy III (USA) [{name}].sfc').read_bytes()
        assert len(data)==size and hashlib.sha256(data).hexdigest()==digest;images.append(data)
    clean,target=images
    source=(ROOT/'extracted/reference/ff6-disassembly-battle/data--magic_prop.asm').read_text()
    blocks=re.findall(r'^\s*magic_prop (\w+)\s*\n(.*?)^\s*end_magic_prop',source,re.M|re.S)
    assert len(blocks)==256;checks=0
    for index,(_,block) in enumerate(blocks):
        for name,offset in [('mp_cost',5),('spell_power',6),('hit_rate',8)]:
            match=re.search(r'^\s*'+name+r'\s+(\d+)\s*$',block,re.M);value=int(match[1]) if match else 0
            assert clean[0x46ac0+index*14+offset]==value,(index,name);checks+=1
    out=ROOT/'extracted/twue-3.07/build/native_magic';out.mkdir(parents=True,exist_ok=True)
    offset=0x46ac0;data=target[offset:offset+3584];(out/'attack_properties.bin').write_bytes(data)
    changes=[{'cpu':f'{0xc00000+offset+i:06X}','file_offset':f'{offset+i:06X}','attack':i//14,'byte':i%14,'clean':a,'target':b} for i,(a,b) in enumerate(zip(clean[offset:offset+3584],data)) if a!=b]
    report={'source_commit':'813013276c952fdd27edcf7b8b86f17291542cbf','cpu':'C46AC0','file_offset':'046AC0','length':3584,'record_size':14,'numeric_source_fields':checks,'sha256':hashlib.sha256(data).hexdigest(),'changes':changes,'code_regions':[]}
    for name,start,end in [('patched_base_damage',0x22b69,0x22c21),('patched_damage_helpers',0x20d4a,0x20d87),('target_damage_modifiers',0x20c9e,0x20d4a),('multiply24',0x247b7,0x247d6)]:
        code=target[start:end];(out/f'{name}.bin').write_bytes(code)
        report['code_regions'].append({'name':name,'cpu':f'{start+0xc00000:06X}','file_offset':f'{start:06X}','length':end-start,'changed_bytes':sum(a!=b for a,b in zip(clean[start:end],code)),'sha256':hashlib.sha256(code).hexdigest()})
    assert target[0x2322b:0x23231]==bytes.fromhex('20 69 2b ea ea ea')
    report['main_attack_callsite']={'cpu':'C2322B','file_offset':'02322B','target_bytes':'20 69 2B EA EA EA','meaning':'shared base damage entry; old separate RelicDmgEffect call replaced by NOPs'}
    (out/'provenance.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(f'Attack extraction: {checks} clean numeric fields checked, {len(changes)} target attack-property changes; patched damage regions retained.')
if __name__=='__main__':main()
