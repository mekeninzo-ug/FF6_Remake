#!/usr/bin/env python3
"""Bounded target C2/0E77 execution versus native equipment aggregation.

Only the mapped equipment routine and its verified multiply helper are allowed.
This checker has no PPU, audio, input, cartridge or general SNES runtime.
"""
from pathlib import Path
import csv,hashlib,json,subprocess,sys
ROOT=Path(__file__).resolve().parents[1]

class EquipmentCpu:
    def __init__(self,rom,prop,status,equipment):
        self.rom=rom;self.memory={};self.pc=0x20e77;self.a=self.x=self.y=0
        self.p=0x20;self.bank=0x7e;self.sp=0x1ff;self.steps=0;self.trace=None;self.stop_pc=None
        raw=rom[0x2d7ca0+prop*22:0x2d7cb6+prop*22]
        self.memory[0x7e1600]=prop;self.memory[0x7e1614]=status
        for i,v in enumerate(raw[6:10]):self.memory[0x7e161a+i]=v
        for i,v in enumerate(equipment):self.memory[0x7e161f+i]=v
    def read(self,address,width=8):
        if 0xc00000<=address<=0xffffff:value=self.rom[address-0xc00000]
        else:value=self.memory.get(address,0)
        if width==16:return value|(self.read(address+1)<<8)
        return value
    def write(self,address,value,width=8):
        self.memory[address]=value&255
        if width==16:self.memory[address+1]=(value>>8)&255
    def fetch(self,width=8):
        v=self.read(0xc00000+self.pc,width);self.pc+=width//8;return v
    def push(self,v,width=8):
        if width==16:self.push(v>>8)
        self.write(self.sp,v);self.sp=(self.sp-1)&65535
    def pop(self,width=8):
        self.sp=(self.sp+1)&65535;v=self.read(self.sp)
        return v|(self.pop()<<8) if width==16 else v
    def flag(self,mask,enabled):self.p=(self.p|mask) if enabled else (self.p&~mask)
    def nz(self,v,width):
        self.flag(2,v==0);self.flag(0x80,bool(v&(1<<(width-1))));return v
    def seta(self,v,width):
        v&=(1<<width)-1;self.a=(self.a&0xff00)|v if width==8 else v;self.nz(v,width)
    def allowed(self):return 0x20e77<=self.pc<0x21105
    def result(self):return bytes(self.read(0x7e11a0+i) for i in range(64)),self.read(0x7e160c)&0xc0,self.read(0x7e1610)&0xc0
    def hook(self,dest):
        if dest!=0x4781:return False
        assert self.rom[0x24781:0x24792]==bytes.fromhex('08 c2 20 8f 02 42 00 ea ea ea ea af 16 42 00 28 60')
        self.a=(self.a&255)*(self.a>>8);return True
    def run(self):
        for _ in range(getattr(self,"instruction_budget",20000)):
            if self.pc==self.stop_pc:return self.result()
            self.steps+=1;start=self.pc;op=self.fetch();w=8 if self.p&0x20 else 16;iw=8 if self.p&0x10 else 16
            if self.trace is not None:self.trace.append({'cpu':f'{start+0xc00000:06X}','file_offset':f'{start:06X}','opcode':f'{op:02X}','a':self.a,'x':self.x,'y':self.y,'p':self.p,'sp':self.sp})
            mask=(1<<w)-1;low=self.a&mask
            if op in (0xda,0x5a):self.push(self.x if op==0xda else self.y,iw)
            elif op==0xea:pass
            elif op==0x8b:self.push(self.bank)
            elif op==8:self.push(self.p)
            elif op==0x48:self.push(low,w)
            elif op==0xab:self.bank=self.nz(self.pop(),8)
            elif op==0x68:self.seta(self.pop(w),w)
            elif op in (0xfa,0x7a):
                value=self.nz(self.pop(iw),iw)
                if op==0xfa:self.x=value
                else:self.y=value
            elif op==0x28:
                self.p=self.pop()
                if self.p&0x10:self.x&=255;self.y&=255
            elif op in (0xc2,0xe2):
                bits=self.fetch();self.p=self.p&~bits if op==0xc2 else self.p|bits
                if self.p&0x10:self.x&=255;self.y&=255
            elif op==0xa9:self.seta(self.fetch(w),w)
            elif op in (0xa2,0xa0):
                value=self.nz(self.fetch(iw),iw)
                if op==0xa2:self.x=value
                else:self.y=value
            elif op in (0xad,0xbd,0xb9,0xaf,0xbf,0xa5,0xb5,0xa3,0xae,0xbe,0xac,0xbc,0xa6):
                if op in (0xaf,0xbf):address=self.fetch(16)|(self.fetch()<<16);address+=self.x if op==0xbf else 0
                elif op in (0xa5,0xb5,0xa6):address=self.fetch()+(self.x if op==0xb5 else 0)
                elif op==0xa3:address=self.sp+self.fetch()
                else:address=(self.bank<<16)|((self.fetch(16)+(self.x if op in (0xbd,0xbc) else self.y if op in (0xb9,0xbe) else 0))&65535)
                if op in (0xae,0xbe,0xa6):self.x=self.nz(self.read(address,iw),iw)
                elif op in (0xac,0xbc):self.y=self.nz(self.read(address,iw),iw)
                else:self.seta(self.read(address,w),w)
            elif op in (0xdf,0x7f,0x3f,0xff):
                address=(self.fetch(16)|(self.fetch()<<16))+self.x;v=self.read(address,w)
                if op==0xdf:self.flag(1,low>=v);self.nz((low-v)&mask,w)
                elif op==0x3f:self.seta(low&v,w)
                elif op==0xff:
                    result=low-v-(1-(self.p&1));self.flag(1,result>=0);self.seta(result,w)
                else:
                    result=low+v+(self.p&1);self.flag(1,result>mask);self.seta(result,w)
            elif op in (0x84,0x86):self.write(self.fetch(),self.y if op==0x84 else self.x,iw)
            elif op in (0x8e,0x8c):self.write((self.bank<<16)|self.fetch(16),self.x if op==0x8e else self.y,iw)
            elif op==0xa4:self.y=self.nz(self.read(self.fetch(),iw),iw)
            elif op==0x8f:self.write(self.fetch(16)|(self.fetch()<<16),low,w)
            elif op in (0x8d,0x9d,0x99,0x85,0x95):
                address=self.fetch()+(self.x if op==0x95 else 0) if op in (0x85,0x95) else (self.bank<<16)|((self.fetch(16)+(self.x if op==0x9d else self.y if op==0x99 else 0))&65535)
                self.write(address,low,w)
            elif op in (0x9c,0x9e,0x64,0x74):
                address=self.fetch()+(self.x if op==0x74 else 0) if op in (0x64,0x74) else (self.bank<<16)|((self.fetch(16)+(self.x if op==0x9e else 0))&65535);self.write(address,0,w)
            elif op in (0x29,0x25,0x2d,0x3d,0x39,0x49,0x45,0x4d,0x5d,0x59,0x09,0x89,0x2c,0x3c,0x24,0x0d,0x1d,0x19,0x05,0x0c,0x1c,0x14,0x04):
                address=None
                if op in (0x2d,0x3d,0x2c,0x3c,0x0d,0x1d,0x19,0x39,0x59,0x4d,0x5d,0x0c,0x1c):address=(self.bank<<16)|((self.fetch(16)+(self.x if op in (0x3d,0x3c,0x1d,0x5d) else self.y if op in (0x19,0x39,0x59) else 0))&65535);v=self.read(address,w)
                elif op in (0x05,0x25,0x45,0x24,0x14,0x04):address=self.fetch();v=self.read(address,w)
                else:v=self.fetch(w)
                if op in (0x29,0x25,0x2d,0x3d,0x39):self.seta(low&v,w)
                elif op in (0x49,0x45,0x4d,0x5d,0x59):self.seta(low^v,w)
                elif op in (0x09,0x0d,0x1d,0x19,0x05):self.seta(low|v,w)
                else:
                    self.flag(2,(low&v)==0)
                    if op in (0x2c,0x3c,0x24):self.flag(0x80,bool(v&(1<<(w-1))))
                    if op in (0x0c,0x1c,0x14,0x04):self.write(address,v|low if op in (0x0c,0x04) else v&~low,w)
            elif op in (0x69,0x6d,0x7d,0x79,0x63,0x65):
                if op==0x69:v=self.fetch(w)
                elif op==0x63:v=self.read(self.sp+self.fetch(),w)
                elif op==0x65:v=self.read(self.fetch(),w)
                else:v=self.read((self.bank<<16)|((self.fetch(16)+(self.x if op==0x7d else self.y if op==0x79 else 0))&65535),w)
                result=low+v+(self.p&1);self.flag(1,result>mask);self.seta(result,w)
            elif op in (0x03,0x23,0x83):
                address=self.sp+self.fetch()
                if op in (0x03,0x23):self.seta(low|self.read(address,w) if op==0x03 else low&self.read(address,w),w)
                else:self.write(address,low,w)
            elif op in (0xe5,0xe9,0xed):
                v=self.read(self.fetch(),w) if op==0xe5 else self.read((self.bank<<16)|self.fetch(16),w) if op==0xed else self.fetch(w);result=low-v-(1-(self.p&1));self.flag(1,result>=0);self.seta(result,w)
            elif op in (0xc9,0xe0,0xc0,0xc5,0xcd,0xdd,0xd9,0xec,0xcc):
                width=iw if op in (0xe0,0xc0,0xec,0xcc) else w
                if op==0xc5:v=self.read(self.fetch(),width)
                elif op in (0xcd,0xdd,0xd9,0xec,0xcc):v=self.read((self.bank<<16)|((self.fetch(16)+(self.x if op==0xdd else self.y if op==0xd9 else 0))&65535),width)
                else:v=self.fetch(width)
                value=self.x if op in (0xe0,0xec) else self.y if op in (0xc0,0xcc) else low
                self.flag(1,value>=v);self.nz((value-v)&((1<<width)-1),width)
            elif op in (0x0a,0x4a,0x2a,0x26,0x06,0x0e,0x1e,0x4e,0x5e,0x6e,0x7e,0x46,0x66,0x6a):
                address=((self.bank<<16)|((self.fetch(16)+(self.x if op in (0x1e,0x5e,0x7e) else 0))&65535)) if op in (0x0e,0x1e,0x4e,0x5e,0x6e,0x7e) else self.fetch() if op in (0x26,0x06,0x46,0x66) else None;v=self.read(address,w) if address is not None else low;carry=self.p&1
                if op in (0x4a,0x4e,0x5e,0x6e,0x7e,0x46,0x66,0x6a):self.flag(1,bool(v&1));result=(v>>1)|((carry<<(w-1)) if op in (0x66,0x6a,0x6e,0x7e) else 0)
                else:self.flag(1,bool(v&(1<<(w-1))));result=((v<<1)+(carry if op in (0x2a,0x26) else 0))&mask
                if address is None:self.seta(result,w)
                else:self.write(address,result,w);self.nz(result,w)
            elif op in (0x1a,0x3a):self.seta(low+(1 if op==0x1a else -1),w)
            elif op in (0xe6,0xc6,0xee,0xfe,0xde):
                address=self.fetch() if op in (0xe6,0xc6) else (self.bank<<16)|((self.fetch(16)+(self.x if op in (0xfe,0xde) else 0))&65535)
                v=(self.read(address,w)+(-1 if op in (0xde,0xc6) else 1))&mask;self.write(address,v,w);self.nz(v,w)
            elif op==0x54:
                destination=self.fetch();source=self.fetch();count=self.a+1
                for offset in range(count):self.write((destination<<16)|((self.y+offset)&65535),self.read((source<<16)|((self.x+offset)&65535)))
                self.x=(self.x+count)&65535;self.y=(self.y+count)&65535;self.a=65535;self.bank=destination
            elif op==0xf4:self.push(self.fetch(16),16)
            elif op in (0xe8,0xca,0xc8,0x88):
                value=(self.x if op in (0xe8,0xca) else self.y)+(1 if op in (0xe8,0xc8) else -1);value&=(1<<iw)-1;self.nz(value,iw)
                if op in (0xe8,0xca):self.x=value
                else:self.y=value
            elif op in (0x8a,0x98):self.seta(self.x if op==0x8a else self.y,w)
            elif op in (0xaa,0xa8):
                value=self.a&((1<<iw)-1);self.nz(value,iw)
                if op==0xaa:self.x=value
                else:self.y=value
            elif op in (0x9b,0xbb):
                if op==0x9b:self.y=self.nz(self.x&((1<<iw)-1),iw)
                else:self.x=self.nz(self.y&((1<<iw)-1),iw)
            elif op==0xeb:self.a=((self.a&255)<<8)|(self.a>>8);self.nz(self.a&255,8)
            elif op==0x7b:self.a=0;self.nz(0,16) # direct page is fixed at zero
            elif op in (0x18,0x38):self.flag(1,op==0x38)
            elif op in (0xd0,0xf0,0x10,0x30,0xb0,0x90,0x80):
                relative=self.fetch();take={0xd0:not self.p&2,0xf0:bool(self.p&2),0x10:not self.p&0x80,0x30:bool(self.p&0x80),0xb0:bool(self.p&1),0x90:not self.p&1,0x80:True}[op]
                if take:self.pc+=relative if relative<128 else relative-256
            elif op in (0x20,0xfc,0x4c,0x7c):
                dest=self.fetch(16)
                if op in (0xfc,0x7c):dest=self.read(0xc20000+((dest+self.x)&65535),16)
                if op in (0x20,0x4c) and self.hook(dest):
                    if op==0x4c:
                        if self.sp==0x1ff:return self.result()
                        self.pc=0x20000+((self.pop(16)+1)&65535)
                else:
                    if op not in (0x4c,0x7c):self.push((self.pc-1)&65535,16)
                    self.pc=0x20000+dest
            elif op==0x60:
                if self.sp==0x1ff:return self.result()
                self.pc=0x20000+((self.pop(16)+1)&65535)
            elif op==0x6b:
                assert self.sp==0x1ff
                return self.result()
            else:raise AssertionError(f'Unsupported opcode {op:02X} at C{start:05X}')
            assert self.allowed(),hex(self.pc)
        raise AssertionError('Equipment instruction budget exceeded')

def main():
    subprocess.run([sys.executable,str(ROOT/'tools/extract_native_items.py')],cwd=ROOT,check=True)
    rom=(ROOT/'Final Fantasy III (USA) [Patched].sfc').read_bytes()
    exe=ROOT/'target/release/ff6-native.exe';out=ROOT/'reverse-engineering/captures/native-equipment'
    subprocess.run([str(exe),'--capture-equipment',str(out)],cwd=ROOT,check=True)
    with (out/'equipment.csv').open(newline='') as stream:rows=list(csv.DictReader(stream))
    assert len(rows)==3080
    total=0
    for row in rows:
        cpu=EquipmentCpu(rom,int(row['property']),int(row['status']),bytes.fromhex(row['equipment']))
        scratch,hp,mp=cpu.run();total+=cpu.steps
        expected=bytes.fromhex(row['scratch'])
        assert scratch==expected,(row['case'],[(hex(0x11a0+i),a,b) for i,(a,b) in enumerate(zip(scratch,expected)) if a!=b])
        assert (hp,mp)==(int(row['hp_boost']),int(row['mp_boost'])),row['case']
        assert int(row['battle_strength'])==min(scratch[6]*2,255)
    with (out/'native-party-equipment.csv').open(newline='') as stream:party=list(csv.DictReader(stream))
    for actual,reference in zip(party,rows[:3]):
        raw=bytes.fromhex(reference['scratch'])
        assert tuple(map(int,list(actual.values())[1:]))==tuple(raw[i] for i in [6,4,2,0,26,27,8,10,12,13,63])
    assert len(party)==3
    report={'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest().upper(),'equipment_cases':len(rows),'target_instructions_executed':total,
        'native_party_rows':len(party),'reference_kind':'bounded execution of actual target C2/0E77 equipment routine; no complete battle/emulator capture','result':'pass'}
    (ROOT/'reverse-engineering/equipment-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(report,indent=2))
if __name__=='__main__':main()
