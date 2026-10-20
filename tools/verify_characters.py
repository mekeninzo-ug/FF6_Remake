#!/usr/bin/env python3
"""Compare native base character state with source-mapped target ROM tables."""
from pathlib import Path
import csv,hashlib,json,subprocess,sys
ROOT=Path(__file__).resolve().parents[1]
def read(path):
    with path.open(newline='') as stream:return list(csv.DictReader(stream))
def main():
    subprocess.run([sys.executable,str(ROOT/'tools/extract_native_characters.py')],cwd=ROOT,check=True)
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes()
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-characters'
    subprocess.run([str(exe),'--capture-characters',str(out)],cwd=ROOT,check=True)
    rows=read(out/'levels.csv');assert len(rows)==64*99
    for row in rows:
        prop,level=int(row['property']),int(row['level']);offset=0x2d7ca0+22*prop
        raw=rom[offset:offset+22];hp=raw[0];mp=raw[1];exp=0
        # Source InitMaxHP/MP and UpdateExp add one entry per gained level.
        for gain in range(level-1):
            hp+=rom[0x26f4a0+gain];mp+=rom[0x26f502+gain]
            exp+=8*int.from_bytes(rom[0x2d8220+2*gain:0x2d8222+2*gain],'little')
        assert (int(row['hp']),int(row['mp']),int(row['exp']),int(row['speed']))==(hp,mp,exp,raw[7]),row
        assert bytes.fromhex(row['commands'])==raw[2:6] and bytes.fromhex(row['equipment'])==raw[15:21]
    expected=[(0,0,3,63,27,96),(14,32,1,68,0,0),(15,33,1,70,0,0)]
    assert [tuple(map(int,r.values())) for r in read(out/'opening-party.csv')]==expected
    game=read(out/'native-game-party.csv')
    assert [tuple(map(int,list(r.values())[1:])) for r in game if r['state']=='new']==expected
    restored=[(0,0,3,17,9,96),(14,32,1,29,0,0),(15,33,1,70,0,0)]
    assert [tuple(map(int,list(r.values())[1:])) for r in game if r['state']=='restored']==restored
    assert (out/'character-save.dat').read_text().startswith('FF6NATIVE9\ncharacters=')
    provenance=json.loads((ROOT/'extracted/twue-3.07/build/native_characters/provenance.json').read_text())
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),
        'character_level_cases':len(rows),'numeric_clean_source_fields':provenance['numeric_character_fields_checked'],
        'growth_entries':294,'opening_party':expected,'native_game_and_save_rows':len(game),
        'mp_growth_changed_bytes':86,'japanese_source_mp_difference':provenance['target_mp_growth_japanese_differences'],
        'reference_kind':'raw target tables and static field initialization source; no original runtime capture',
        'result':'pass'}
    (ROOT/'reverse-engineering/characters-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(report,indent=2))
if __name__=='__main__':main()
