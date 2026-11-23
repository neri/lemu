"""Collect actual entry/dispatch counts; validate independent reference checksums.

Counters are a separate target directory. Never use their elapsed time.
"""
import collections
import csv
import hashlib
import io
import json
import re
import subprocess
import sys
from pathlib import Path
repo=Path(__file__).resolve().parents[2]
root=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();out.mkdir(exist_ok=True,parents=True)
manifest=json.loads((repo/'fixtures/frequency-v1/manifest.json').read_text())
reuse='--reuse' in sys.argv
if not reuse: subprocess.run(['cargo','build','--offline','--release','--features','uop-profile','--example','frequency_bench',
                '--target-dir',str(root/'target-profile'),'--manifest-path',str(root/'Cargo.toml')],check=True)
data=bytes(((i*37+(i>>3)*11)^(i>>5))&255 for i in range(65536))
u32=lambda n:n&0xffffffff
word=lambda i:int.from_bytes(data[4096+2*i:4098+2*i],'little')
sign=lambda x,b:x-(1<<b) if x&(1<<(b-1)) else x
mix=lambda x:u32((x^(x>>7))*33+17)
def operands(form):
 parts=form.split('_')
 mnemonic=parts[1] if parts[0] in ['REP','REPZ','REPNZ'] else parts[0]
 if mnemonic in ['MOVSB','MOVSW','MOVSD','CMPSB','CMPSW','CMPSD']:
  width={'B':8,'W':16,'D':32}[mnemonic[-1]]
  return [dict(kind='memory',role='destination' if mnemonic.startswith('MOVS') else 'read',width=width,segment='ES',index='EDI'),
          dict(kind='memory',role='source',width=width,segment='DS',index='ESI')]
 result=[]
 for i,t in enumerate(parts[1:]):
  kind='memory' if re.fullmatch(r'M[bwd]',t) else 'register' if re.match(r'R[bwd]',t) else 'immediate' if re.match(r'I[bwdv]',t) else 'relative' if t.startswith('J') else 'attribute'
  role='source' if mnemonic=='PUSH' else 'target' if mnemonic in ['CALL','JMP','JCC'] else 'read' if mnemonic in ['CMP','TEST'] else 'destination' if i==0 else 'source'
  result.append(dict(kind=kind,role=role,width={'b':8,'w':16,'d':32,'v':32}.get(t[1:2])))
 return result

def reference(family):
 s=0
 if family=='integer':
  for i in range(1024):
   x=int.from_bytes(data[4096+4*i:4100+4*i],'little')
   s=u32(s+((x>>3)+i if x&1 else x^(i*13)));s=u32((s<<3)|(s>>29))
 elif family=='byte-word':
  for i in range(2048):
   c=data[4096+i];s=u32(s+(c-48 if 48<=c<=57 else c^32)+sign(word((i+c)&2047),16)+sign(c,8))
 elif family.startswith('updates'):
  counts=list(range(256))
  for i in range(2048):
   x=data[4096+i]
   if family!='updates-register':counts[x]=u32(counts[x]+3)
   if family!='updates-immediate':counts[(x+17)&255]=u32(counts[(x+17)&255]+word((i+x)&2047))
   s=u32(s*33)^counts[(x+31)&255]
  s=u32(s+sum(counts))
 elif family=='calls':
  rec=7
  for _ in range(4):rec=mix(rec)
  for i in range(1024):
   x=int.from_bytes(data[4096+4*i:4100+4*i],'little')
   s=u32(s+mix(x)) if (x&15)<7 else s^mix(i)
   if i&63==0:s=u32(s+rec)
 elif family=='copy-compare':
  for n in [16,128,1024]:
   s+=sum(data[4096+i]^data[4096+((i+1)&2047)] for i in range(n))
 return u32(s)
rows=[];summaries=[]
for case in manifest['cases']:
 raw=(out/f"{case['name']}.tsv").read_text() if reuse else subprocess.check_output([str(root/'target-profile/release/examples/frequency_bench'),str(case['id'])],text=True)
 (out/f"{case['name']}.tsv").write_text(raw)
 lines=raw.splitlines();checksum=int(lines[0].split('=')[1]);assert checksum==reference(case['name'].rsplit('-',1)[0]),(case['name'],checksum)
 totals=dict((k,int(v)) for k,v in re.findall(r'(\w+)=(\d+)',lines[1]));assert totals['faults']==0
 instructions=list(csv.DictReader(io.StringIO('\n'.join(lines[2:])),delimiter='\t'))
 guest=sum(int(r['entries']) for r in instructions)
 candidates=collections.Counter();family_counts=collections.Counter();forms=collections.Counter();narrow_loads=0;extension_forms=collections.Counter()
 for r in instructions:
  count=int(r['entries']);ir=r['ir'];uops=r['uops'];form=ir.split('(')[0];mnemonic=form.split('_')[0]
  if mnemonic in ['REP','REPZ','REPNZ']:mnemonic=form.split('_')[1]
  family_counts[mnemonic]+=count
  # Use actual lowered address operations, so SIB encoded with scale=1 is
  # distinguished from BD even when its pretty-printed EA looks equivalent.
  ea='none';a32=bool(re.search(r'_M[bwdp]',form)) and 'a16:' not in ir
  if a32:
   ea='SIB' if 'LeaSIB' in uops or 'LoadSIB' in uops or 'StoreSIB' in uops else ('absolute' if re.search(r'\[0x',ir) else ('BD' if '+' in ir[ir.find('['):] else 'base-only'))
  segment='CS' if 'CS:' in ir or 'CS(' in uops else ('SS' if re.search(r'\[(ESP|EBP)',ir) else 'DS') if ea!='none' else 'none'
  explicit_segment=re.search(r'\[(ES|CS|SS|DS|FS|GS):',ir)
  if explicit_segment:segment=explicit_segment[1]
  if 'StringOp' in uops:
   ea='implicit-index';segment='DS/ES'
  forms[(form,ea,segment)]+=count
  load=bool(re.search(r'LoadR(?:8|16)(?:CS)?\(',uops))
  narrow_loads+=count if load else 0
  if load and 'CS' not in uops:
   if 'LeaSIB' in uops:candidates['sib2']+=count
   elif 'LeaBD' in uops or 'LoadConst(MemAddr' in uops:candidates['bd']+=count
  if re.match(r'MOV[ZS]X_Rd_M[bw]$',form) and a32 and load and not any(x in uops for x in ['Lea','LoadConst','CS']):candidates['extend']+=count
  if re.match(r'MOV[ZS]X_Rd_',form):
   for suffix,key in [('_Mb','memory8'),('_Mw','memory16'),('_Rb','register8'),('_Rw','register16')]:
    if form.endswith(suffix) and (a32 or not key.startswith('memory')):
     extension_forms[key+'_total']+=count
     if key.startswith('memory') and load and not any(x in uops for x in ['Lea','LoadConst','CS']):
      extension_forms[key+'_eligible']+=count
  if form=='ADD_Md_Id' and a32:candidates['add']+=count
  r.update(case=case['name'],mnemonic=mnemonic,form=form,ea=ea,segment=segment)
  rows.append(r)
 summary=dict(case=case['name'],checksum=checksum,guest_instructions=guest,**totals,mnemonics=family_counts,extension_applicability={k:extension_forms[k] for k in ['memory8_total','memory8_eligible','memory16_total','memory16_eligible','register8_total','register16_total']},
              candidates={k:dict(applicable=candidates[k],fraction_guest=candidates[k]/guest,
                  family_fraction=candidates[k]/max(1,sum(family_counts[m] for m in ['MOVZX','MOVSX']) if k=='extend' else family_counts['ADD'] if k=='add' else narrow_loads),
                  saved_dispatches=candidates[k] if k!='add' else 0) for k in ['bd','sib2','extend','add']},
              branch_instructions=sum(v for k,v in family_counts.items() if k.startswith('J') or k.startswith('LOOP')),
              call_instructions=family_counts['CALL'],return_instructions=family_counts['RET'],
              forms=[dict(form=f,operands=operands(f),ea=ea,segment=seg,count=n) for (f,ea,seg),n in forms.items()])
 summaries.append(summary)
(out/'frequency.json').write_text(json.dumps(dict(counter_enabled=True,reference='independent Python reference',cases=summaries),indent=2)+'\n')
with (out/'instructions.csv').open('w') as f:
 writer=csv.DictWriter(f,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
print([(s['case'],{k:v['applicable'] for k,v in s['candidates'].items()}) for s in summaries])
