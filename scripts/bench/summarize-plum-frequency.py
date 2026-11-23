"""Fixed family weights, batch confidence intervals, sensitivity, adoption gates.

Usage: python3 scripts/bench/summarize-plum-frequency.py RUN_ROOT
Screen data is never eligible for adoption. Separate environments stay separate.
"""
import json
import math
import statistics
import sys
from pathlib import Path
repo=Path(__file__).resolve().parents[2];root=Path(sys.argv[1])
m=json.loads((repo/'fixtures/frequency-v1/manifest.json').read_text())
def interval(xs):
 mean=statistics.mean(xs);n=len(xs)
 t={3:4.303,5:2.776}.get(n,2.776)
 err=t*statistics.stdev(xs)/math.sqrt(n) if n>1 else math.inf
 return dict(ratio=mean,ci95=[mean-err,mean+err],batches=xs)
def summarize(data):
 rows=data['rows'];batches=sorted(set(r['batch'] for r in rows));cases=[]
 for c in m['cases']:
  ratios=[]
  for b in batches:
   pair={r['stage']:r['median_ms'] for r in rows if r['case']==c['id'] and r['batch']==b}
   if len(pair)!=2:raise ValueError('Incomplete batch')
   ratios.append(pair[1]/pair[0])
  cases.append(dict(name=c['name'],family=c['family'],**interval(ratios)))
 families={f:interval([sum(c['case_weight']*cases[c['id']]['batches'][i] for c in m['cases'] if c['family']==f) for i in range(len(batches))]) for f in sorted(set(c['family'] for c in m['cases']))}
 overall=interval([sum(.2*f['batches'][i] for f in families.values()) for i in range(len(batches))])
 sensitivity={}
 for f,v in families.items():
  other=statistics.mean(x['ratio'] for k,x in families.items() if k!=f)
  sensitivity[f]=dict(weight_60_percent=.6*v['ratio']+.4*other,
     breakeven_weight=(1-other)/(v['ratio']-other) if v['ratio']!=other else None)
 eligible=not data['metadata'].get('screen',True) and len(batches)>=5 and all(len(r['samples_ms'])>=9 for r in rows)
 return dict(overall=overall,families=families,cases=cases,sensitivity=sensitivity,adoption_samples=eligible)
reports={}
for p in sorted(root.glob('*.json')):
 parts=p.stem.split('-',1)
 if len(parts)!=2 or parts[0] not in ['native','browser','node']:continue
 data=json.loads(p.read_text())
 if 'rows' not in data:continue
 reports.setdefault(parts[1],{})[parts[0]]=summarize(data)
frequency=json.loads((root/'profiles/baseline/frequency.json').read_text()) if (root/'profiles/baseline/frequency.json').exists() else None
mapping={'bd1':'bd','bd2':'bd','ext1':'extend','ext2':'extend','ext4':'extend','add':'add','sib2':'sib2','bd2-sib2':'sib2'}
for name,envs in reports.items():
 gates=[]
 for primary,other in [('native','browser'),('browser','native')]:
  if primary not in envs or other not in envs:continue
  a,b=envs[primary],envs[other]
  common=a['adoption_samples'] and b['adoption_samples'] and b['overall']['ci95'][1]<=1.02 and max(c['ci95'][1] for c in b['cases'])<=1.05
  A=common and a['overall']['ci95'][1]<=.99 and max(c['ci95'][1] for c in a['cases'])<=1.05
  natural=set()
  if frequency:
   natural={c['case'].rsplit('-',1)[0] for c in frequency['cases'] if c['candidates'][mapping.get(name,'extend')]['applicable']>0}
  gains=[c['name'].rsplit('-',1)[0] for c in a['cases'] if c['ci95'][1]<=.97 and c['name'].rsplit('-',1)[0] in natural]
  independent_gains={('updates' if x.startswith('updates') else x) for x in gains}
  B=common and len(independent_gains)>=2 and a['overall']['ci95'][1]<=1 and max(c['ci95'][1] for c in a['cases'])<=1.03
  natural_families={('updates' if x.startswith('updates') else x) for x in natural}
  gates.append(dict(primary=primary,route_A=A,route_B=B,
    conditions=dict(adoption_sample_count=a['adoption_samples'] and b['adoption_samples'],
      A1_primary_R_upper_le_099=a['overall']['ci95'][1]<=.99,
      A2_other_R_upper_le_102=b['overall']['ci95'][1]<=1.02,
      A3_all_representative_cases_upper_le_105=max(c['ci95'][1] for c in a['cases']+b['cases'])<=1.05,
      B1_two_independent_applicable_workloads=len(natural_families)>=2,
      B2_two_improved_workloads_and_primary_R_upper_le_100=len(independent_gains)>=2 and a['overall']['ci95'][1]<=1,
      B3_primary_cases_upper_le_103_and_other_limits=max(c['ci95'][1] for c in a['cases'])<=1.03 and b['overall']['ci95'][1]<=1.02 and max(c['ci95'][1] for c in b['cases'])<=1.05),
    independent_natural_workloads=sorted(natural),improved_workloads=sorted(set(gains)),independent_improved_families=sorted(independent_gains)))
 envs['decision']=dict(timing_gates=gates,eligible_by_timing=any(g['route_A'] or g['route_B'] for g in gates),
   requires='Matching guest counts/checksums, semantic tests and layout must also pass before adoption')
(root/'summary.json').write_text(json.dumps(reports,indent=2)+'\n')
for name,envs in reports.items():
 print(name,{k:round(v['overall']['ratio'],5) for k,v in envs.items() if k!='decision'},envs['decision']['eligible_by_timing'])
