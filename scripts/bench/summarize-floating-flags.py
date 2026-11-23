"""Summarize case-wise median ratios, then equal-weight geometric means."""
import argparse,json,math,statistics
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('directory',type=Path);a=p.parse_args()
summary={}
for target in ['native','wasm','native-control','wasm-control']:
    path=a.directory/f'{target}.json'
    if not path.exists():continue
    raw=json.loads(path.read_text());rows=raw['rows'];variants=raw['metadata']['variants']
    if len(rows) != (42 if '-control' in target else 96): continue
    cases=[]
    for mode,case in sorted({(r['mode'],r['case']) for r in rows}):
        rs=[r for r in rows if r['mode']==mode and r['case']==case]
        assert len(rs)==3,(target,mode,case,len(rs))
        ratios=[statistics.median(r['medians_ms'][i]/r['medians_ms'][0] for r in rs) for i in range(len(variants))]
        cases.append(dict(mode=mode,case=case,name=rs[0]['name'],backend=rs[0]['backend'],family=rs[0]['family'],ratios=ratios,round_ratios=[[r['medians_ms'][i]/r['medians_ms'][0] for i in range(len(variants))] for r in rs]))
    groups=[]
    for mode,backend in sorted({(r['mode'],r['backend']) for r in cases}):
        xs=[r for r in cases if r['mode']==mode and r['backend']==backend]
        ratios=[math.exp(statistics.mean(math.log(r['ratios'][i]) for r in xs)) for i in range(len(variants))]
        ranges=[[math.exp(statistics.mean(math.log(r['round_ratios'][round_index][i]) for r in xs)) for round_index in range(3)] for i in range(len(variants))]
        groups.append(dict(mode=mode,backend=backend,ratios=ratios,round_ratios=ranges))
        print(target, 'Flat' if mode else 'Harvard',backend,dict(zip(variants,[round(100*(r-1),2) for r in ratios])))
    summary[target]=dict(variants=variants,cases=cases,groups=groups)
(a.directory/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
lines=['| Target | Engine | Guest | Cheap time change | Deferred time change |','|---|---|---|---:|---:|']
for target,s in summary.items():
    for g in s['groups']:
        lines.append('| '+ ' | '.join([target,'Flat' if g['mode'] else 'Harvard',g['backend'],*[f'{100*(r-1):+.2f}%' for r in g['ratios'][1:]]])+' |')
(a.directory/'table.md').write_text('\n'.join(lines)+'\n')
