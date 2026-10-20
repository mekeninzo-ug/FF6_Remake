#!/usr/bin/env python3
"""Extract fixed, source-mapped battle tables after checking both input identities."""
from pathlib import Path
import hashlib, json, re

ROOT = Path(__file__).resolve().parents[1]
TABLES = [('event_groups', 0xF5000, 1024), ('formations', 0xF5900, 2304),
          ('formation_monsters', 0xF6200, 8640), ('monster_properties', 0xF0000, 0x3000),
          ('rng_table',0xFD00,256),('ai_pointers',0xF8400,768),('ai_script',0xF8700,0x3950),
          ('command_delays',0x2067b,32),('command_properties',0xFFE00,64),('ai_command_sizes',0x21daf,16),('ai_attack_thresholds',0x21dd8,11),('ai_attack_commands',0x21de3,11),('command_target_modes',0x2278a,30),('throw_tool_items',0x22778,5),('throw_tool_offsets',0x2277d,5),('monster_special_animations',0xF37C0,384)]

def main():
    images = []
    for name, size, digest in [
        ('Unpatched', 3145728, '0f51b4fca41b7fd509e4b8f9d543151f68efa5e97b08493e4b2a0c06f5d8d5e2'),
        ('Patched', 4194304, 'f385c715b3088d7d697171ec76b93b8f32623d63b3672a364121269a9e41dbb9')]:
        data = (ROOT / f'Final Fantasy III (USA) [{name}].sfc').read_bytes()
        assert len(data) == size and hashlib.sha256(data).hexdigest() == digest
        images.append(data)
    clean, target = images
    source = (ROOT/'extracted/reference/ff6-disassembly-battle/data--monster_prop.asm').read_text()
    # Validate numeric fields of all source records against clean, including defaults.
    fields = {'speed': (0,1,0), 'attack_power': (1,1,0), 'hit_rate': (2,1,100),
              'evade': (3,1,0), 'mblock': (4,1,0), 'defense': (5,1,0),
              'mag_def': (6,1,0), 'mag_pwr': (7,1,0), 'hp': (8,2,0),
              'mp': (10,2,0), 'exp': (12,2,0), 'gil': (14,2,0), 'level': (16,1,0)}
    blocks = re.findall(r'^\s*monster_prop (\w+)\s*\n(.*?)^\s*end_monster_prop', source, re.M|re.S)
    assert len(blocks) == 384
    checked = 0
    for index, (_, block) in enumerate(blocks):
        for field, (offset, width, default) in fields.items():
            match = re.search(r'^\s*'+field+r'\s+([\d$A-Fa-f]+)\s*$', block, re.M)
            value = int(match[1][1:],16) if match and match[1].startswith('$') else int(match[1]) if match else default
            assert int.from_bytes(clean[0xF0000+index*32+offset:0xF0000+index*32+offset+width], 'little') == value, (index,field)
            checked += 1
    out = ROOT/'extracted/twue-3.07/build/native_battle'
    out.mkdir(parents=True, exist_ok=True)
    report = {'source_commit': '813013276c952fdd27edcf7b8b86f17291542cbf',
              'numeric_source_fields_checked': checked, 'tables': []}
    for name, offset, size in TABLES:
        data = target[offset:offset+size]
        assert len(data) == size
        (out/f'{name}.bin').write_bytes(data)
        changed = [i for i,(a,b) in enumerate(zip(clean[offset:offset+size],data)) if a != b]
        record_size={'event_groups':4,'formations':4,'formation_monsters':15,'monster_properties':32,'rng_table':1,'ai_pointers':2,'ai_script':1,'command_delays':1,'command_properties':2,'ai_command_sizes':1,'ai_attack_thresholds':1,'ai_attack_commands':1,'command_target_modes':1,'throw_tool_items':1,'throw_tool_offsets':1,'monster_special_animations':1}[name]
        report['tables'].append({'name': name, 'cpu': f'{offset+0xC00000:06X}',
            'file_offset': f'{offset:06X}', 'length': size, 'sha256': hashlib.sha256(data).hexdigest(),
            'changed_bytes': len(changed), 'record_size':record_size,'changed_records': sorted({i//record_size for i in changed}),
            'changes':[{'file_offset':f'{offset+i:06X}','cpu':f'{offset+i+0xc00000:06X}',
                        'clean':clean[offset+i],'target':data[i]} for i in changed]})
    assert clean[0x21daf:0x21dbf]==target[0x21daf:0x21dbf]==bytes([4,2,4,3,4,4,4,2,3,4,4,3,4,1,1,1])
    assert clean[0x21dd8:0x21dee]==target[0x21dd8:0x21dee]==bytes.fromhex('36 51 55 5d 65 7d 82 83 8b ee f0 19 02 07 0a 02 09 1b 1d 0c 00 02')
    assert clean[0x22778:0x22782]==target[0x22778:0x22782]==bytes.fromhex('a4 a5 ab ac ad 27 27 5a 5a 5a')
    assert clean[0x2278a:0x227a8]==target[0x2278a:0x227a8]==bytes.fromhex('20 1a 04 18 18 00 20 24 06 06 04 18 04 80 80 04 04 80 18 04 18 18 21 04 01 04 04 04 81 04')
    assert len(set(target[0xfd00:0xfe00]))==256
    assert clean[0xf8700:0xf8706]==target[0xf8700:0xf8706]==bytes.fromhex('f0 ee ee ef ff ff')
    assert all(int.from_bytes(target[i:i+2],'little')<0x3950 for i in range(0xf8400,0xf8700,2))
    delay_source=(ROOT/'extracted/reference/ff6-disassembly-battle/battle--action_1.asm').read_text().split('CmdDelayTbl:',1)[1].split('AfterAction1:',1)[0]
    delays=bytes(int(v) for v in re.findall(r'^\s*(?:@[0-9a-f]+:\s*)?\.byte\s+(\d+)\s*$',delay_source,re.M))
    assert len(delays)==32 and clean[0x2067b:0x2069b]==target[0x2067b:0x2069b]==delays
    command_source=(ROOT/'extracted/reference/ff6-disassembly-battle/data--battle_cmd_prop.asm').read_text()
    command_flags=re.findall(r'^battle_cmd_prop \w+, (\{[^}]+\}|\w+),',command_source,re.M)
    assert len(command_flags)==32
    flag_values={'NONE':0,'GOGO':1,'MIMIC':2,'IMP':4,'UNKNOWN':8}
    for index,flags in enumerate(command_flags):
        expected=0
        for flag in flags.strip('{}').split(','):expected|=flag_values[flag.strip()]
        assert clean[0xffe00+index*2]==target[0xffe00+index*2]==expected
    assert target[0x202a7:0x202ab]==bytes.fromhex('20 e1 67 ea')
    assert target[0x267e1:0x267f2]==bytes.fromhex('a5 b5 c9 19 d0 08 bd 18 30 1c 2e 3f a5 b5 c9 1e 60')
    report['queued_action_code']=[{'cpu':f'{start+0xc00000:06X}','file_offset':f'{start:06X}','length':end-start,'changed_bytes':sum(a!=b for a,b in zip(clean[start:end],target[start:end])),'sha256':hashlib.sha256(target[start:end]).hexdigest()} for start,end in [(0x20276,0x202dc),(0x267e1,0x267f2)]]
    report['unchanged_scheduler_code_regions']=[]
    for name,start,end in [('atb_update',0x2111b,0x21211),('queue_enqueue',0x24e66,0x24e91),('dispatch_selector',0x20033,0x200c5),('command_delay',0x203e4,0x20420),('clear_flag1',0x20a43,0x20a4a)]:
        assert clean[start:end]==target[start:end],name
        report['unchanged_scheduler_code_regions'].append({'name':name,'cpu':f'{start+0xc00000:06X}','file_offset':f'{start:06X}','length':end-start,'sha256':hashlib.sha256(target[start:end]).hexdigest()})
    report['unchanged_rng_code_regions']=[]
    for name,start,end in [('battle_loop_rng_step',0x20019,0x2001b),('battle_seed',0x22440,0x22447),('battle_rng',0x24b53,0x24b7b),('bit_selection',0x251f0,0x25269),('initial_gauges',0x22575,0x22602),('ai_pointer_load',0x22d71,0x22d82),('ai_fetch',0x21a5f,0x21a74),('ai_triple_selection',0x21a80,0x21a91)]:
        assert clean[start:end]==target[start:end],name
        report['unchanged_rng_code_regions'].append({'name':name,'cpu':f'{start+0xc00000:06X}','file_offset':f'{start:06X}','length':end-start,'sha256':hashlib.sha256(target[start:end]).hexdigest()})
    (out/'provenance.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(report,indent=2))

if __name__ == '__main__': main()
