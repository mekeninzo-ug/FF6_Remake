from __future__ import annotations
import argparse, csv, html, json, math
from pathlib import Path
from PIL import Image

BPP={'.1bpp':1,'.2bpp':2,'.3bpp':3,'.4bpp':4}

def decode_tile(data: bytes, bpp: int) -> list[int]:
    tile_bytes=8*bpp; pixels=[0]*64
    for y in range(8):
        for x in range(8):
            value=0; bit=7-x
            if bpp==1:
                value=(data[y]>>bit)&1
            else:
                for plane in range(bpp):
                    off=(16+y) if bpp==3 and plane==2 else (plane//2)*16+y*2+(plane%2)
                    value |= ((data[off]>>bit)&1)<<plane
            pixels[y*8+x]=value
    return pixels

def sheet_for_gfx(path: Path, out: Path, bpp: int) -> dict:
    raw=path.read_bytes(); size=8*bpp
    count=len(raw)//size
    if not count or len(raw)%size: return {'file':str(path),'issue':f'{len(raw)} bytes is not a whole number of {size}-byte {bpp}bpp tiles'}
    cols=min(32,max(1,math.ceil(math.sqrt(count))))
    rows=math.ceil(count/cols)
    im=Image.new('P',(cols*8,rows*8),0)
    pal=[]
    for i in range(256):
        pal.extend((i,i,i))
    im.putpalette(pal)
    pixels=im.load()
    for ti in range(count):
        tile=decode_tile(raw[ti*size:(ti+1)*size],bpp); tx=(ti%cols)*8; ty=(ti//cols)*8
        for j,v in enumerate(tile): pixels[tx+j%8,ty+j//8]=v
    out.parent.mkdir(parents=True,exist_ok=True); im.save(out,optimize=True)
    return {'file':str(path),'preview':str(out),'bpp':bpp,'tiles':count,'tile_columns':cols,'tile_rows':rows}

def snes_color(word: int) -> tuple[int,int,int]:
    r=word&31; g=(word>>5)&31; b=(word>>10)&31
    return (r*255//31,g*255//31,b*255//31)

def palette_preview(path: Path,out: Path) -> dict:
    raw=path.read_bytes(); n=len(raw)//2; colors=[snes_color(int.from_bytes(raw[i*2:i*2+2],'little')) for i in range(n)]
    if not colors: return {'file':str(path),'issue':'empty palette'}
    im=Image.new('RGB',(128,math.ceil(n/16)*16),(0,0,0)); px=im.load()
    for i,c in enumerate(colors):
        x=(i%16)*8; y=(i//16)*16
        for yy in range(16):
            for xx in range(8): px[x+xx,y+yy]=c
    out.parent.mkdir(parents=True,exist_ok=True); im.save(out,optimize=True)
    out.with_suffix('.json').write_text(json.dumps({'source':str(path),'format':'SNES BGR555 little-endian','colors':[{ 'index':i,'rgb':list(c),'hex':'#%02X%02X%02X'%c} for i,c in enumerate(colors)]},indent=2),encoding='utf8')
    return {'file':str(path),'preview':str(out),'colors':n}

def tilemap_preview(path: Path,out: Path,csvout: Path) -> dict:
    raw=path.read_bytes()
    if len(raw)%2: return {'file':str(path),'issue':'odd byte count'}
    words=[int.from_bytes(raw[i:i+2],'little') for i in range(0,len(raw),2)]
    if not words: return {'file':str(path),'issue':'empty tilemap'}
    width=32 if len(words)%32==0 else min((d for d in range(1,int(math.sqrt(len(words)))+1) if len(words)%d==0),key=lambda d:abs(d-math.sqrt(len(words))))
    height=math.ceil(len(words)/width)
    im=Image.new('L',(width,height),0); px=im.load(); csvout.parent.mkdir(parents=True,exist_ok=True)
    with csvout.open('w',newline='',encoding='utf8') as f:
        w=csv.writer(f); w.writerow(['x','y','raw','tile_index','palette','priority','hflip','vflip'])
        for i,word in enumerate(words):
            x=i%width;y=i//width;px[x,y]=word&255
            w.writerow([x,y,f'0x{word:04X}',word&0x3ff,(word>>10)&7,(word>>13)&1,(word>>14)&1,(word>>15)&1])
    out.parent.mkdir(parents=True,exist_ok=True); im.save(out,optimize=True)
    return {'file':str(path),'preview':str(out),'decoded_csv':str(csvout),'width_words':width,'height_words':height,'entries':len(words)}

def byte_tilemap_preview(path: Path,out: Path,csvout: Path) -> dict:
    raw=path.read_bytes()
    if path.name=='world_1_tilemap.bin' or path.name=='world_2_tilemap.bin': width=256
    elif path.name=='world_3_tilemap.bin': width=128
    else:
        width=next((w for w in (64,32,16) if len(raw)%w==0),16)
    height=math.ceil(len(raw)/width)
    im=Image.new('L',(width,height),0); px=im.load(); csvout.parent.mkdir(parents=True,exist_ok=True)
    with csvout.open('w',newline='',encoding='utf8') as f:
        w=csv.writer(f); w.writerow(['x','y','tile_index'])
        for i,value in enumerate(raw):
            x=i%width; y=i//width; px[x,y]=value; w.writerow([x,y,value])
    out.parent.mkdir(parents=True,exist_ok=True); im.save(out,optimize=True)
    return {'file':str(path),'preview':str(out),'decoded_csv':str(csvout),'width_tiles':width,'height_tiles':height,'entries':len(raw)}

def main():
    ap=argparse.ArgumentParser(description='Decode FF6 SNES planar tiles, palettes and tilemap entries into previews.')
    ap.add_argument('root',type=Path,help='one extraction root, e.g. extracted/vanilla')
    args=ap.parse_args(); root=args.root.resolve(); gfx=root/'assets'/'gfx'; prev=root/'previews'
    records=[]; issues=[]; data_maps=[]
    for p in sorted(gfx.rglob('*')):
        if not p.is_file(): continue
        rel=p.relative_to(gfx)
        if p.suffix.lower() in BPP:
            r=sheet_for_gfx(p,prev/'gfx'/rel.with_suffix('.png'),BPP[p.suffix.lower()]);records.append(r)
            if 'issue' in r: issues.append(r)
        elif p.suffix.lower()=='.pal': records.append(palette_preview(p,prev/'palettes'/rel.with_suffix('.png')))
        elif p.suffix.lower()=='.scr':
            records.append(tilemap_preview(p,prev/'tilemaps'/rel.with_suffix('.png'),prev/'tilemaps'/rel.with_suffix('.csv')))
    data_root=root/'assets'/'data'
    for p in sorted(data_root.rglob('*.bin')):
        rel=p.relative_to(data_root)
        if rel.parts[:2]==('field','sub_tilemap') or p.name in ('world_1_tilemap.bin','world_2_tilemap.bin','world_3_tilemap.bin'):
            data_maps.append(byte_tilemap_preview(p,prev/'data_tilemaps'/rel.with_suffix('.png'),prev/'data_tilemaps'/rel.with_suffix('.csv')))
    (prev/'graphics_index.json').write_text(json.dumps(records,indent=2),encoding='utf8')
    (prev/'graphics_issues.json').write_text(json.dumps(issues,indent=2),encoding='utf8')
    (prev/'data_tilemaps_index.json').write_text(json.dumps(data_maps,indent=2),encoding='utf8')
    cards=[]
    for r in records:
        if 'preview' not in r: continue
        p=Path(r['preview']); rel=p.relative_to(prev).as_posix(); title=html.escape(str(r['file']))
        cards.append(f'<figure><img loading="lazy" src="{html.escape(rel)}"><figcaption>{title}<br>{html.escape(str(r.get("bpp",r.get("colors",r.get("entries","")))))}</figcaption></figure>')
    for r in data_maps:
        p=Path(r['preview']); rel=p.relative_to(prev).as_posix(); title=html.escape(str(r['file']))
        cards.append(f'<figure><img loading="lazy" src="{html.escape(rel)}"><figcaption>{title}<br>{r["width_tiles"]}×{r["height_tiles"]} tile IDs</figcaption></figure>')
    page='<!doctype html><meta charset="utf-8"><title>FF6 graphics previews</title><style>body{font:14px system-ui;margin:1rem;background:#17191c;color:#eee}main{display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:12px}figure{margin:0;padding:8px;background:#25282d;overflow:hidden}img{width:100%;height:180px;object-fit:contain;image-rendering:pixelated;background:#0d0e10}figcaption{font-size:11px;overflow-wrap:anywhere}</style><h1>Graphics, palette and tilemap previews</h1><p>Planar tile PNGs use indexed grayscale. Palette PNGs decode SNES BGR555 colors. Screen maps include SNES tile attributes in CSV; field/world tilemaps show byte tile IDs in CSV.</p><main>'+''.join(cards)+'</main>'
    (prev/'index.html').write_text(page,encoding='utf8')
    print(f'{root}: {len(records)} graphics/palette/screen-map previews, {len(data_maps)} field/world map previews, {len(issues)} graphics decode issues')
if __name__=='__main__':main()

