"""Dynamic instruction mix, subtracting initialization and the first invocation."""
import json,re,subprocess,collections,argparse
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('binary',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
repo=Path(__file__).resolve().parents[2]
guest=repo/'plum/examples/support/bench/floating-flags/guest'
manifest=json.loads((guest/'manifest.json').read_text())
rows=[]
for c in manifest['cases']:
    raw=subprocess.check_output([str(a.binary.resolve()),str(c['id']),'1','1'],text=True)
    (a.output.parent/(c['name']+'-profile.tsv')).write_text(raw)
    before,after=raw.split('# after\n')
    def parse(s):
        return {int(v[0]):int(v[1]) for line in s.splitlines() if (v:=line.split('\t'))[0].isdigit()}
    x,y=parse(before),parse(after)
    asm={int(m[1],16):(m[2],m[3]) for line in (guest/(c['name']+'.disasm')).read_text().splitlines() if (m:=re.match(r'^\s*([0-9a-f]+):\s+(?:[0-9a-f]{2}\s+)+\s*([a-z][a-z0-9]*)\s*(.*)$',line))}
    counts=collections.Counter();mnemonics=collections.Counter()
    for eip,count in y.items():
        count-=x.get(eip,0)
        if not count:continue
        mn,operands=asm[eip]
        mnemonics[mn]+=count
        if mn.startswith(('fcom','fucom','fcmov')) or 'comis' in mn or mn.startswith(('min','max')):category='floating_compare_select'
        elif mn.startswith(('fadd','fsub','fmul','fdiv','fsqrt','addss','addsd','addps','addpd','subss','subsd','subps','subpd','mulss','mulsd','mulps','mulpd','divss','divsd','divps','divpd','sqrt')):category='floating_arithmetic'
        elif mn.startswith(('cvt','fild','fist')):category='floating_conversion'
        elif mn.startswith('f') or re.match(r'^(mov|xor|and|or|unpck|shuf).*(ss|sd|ps|pd)$',mn) or mn.startswith(('pxor','movd')):category='floating_move_other'
        elif mn.startswith(('j','loop')) or mn in ['call','ret']:category='branch_call'
        elif mn.startswith(('mov','push','pop','lea')):category='integer_move_address'
        else:category='integer_compute_other'
        counts[category]+=count
    total=sum(counts.values())
    rows.append(dict(name=c['name'],backend=c['backend'],family=c['family'],instructions=total,counts=dict(counts),percent={k:100*v/total for k,v in counts.items()},mnemonics=dict(mnemonics.most_common())))
    print(c['name'],total,dict(counts))
a.output.write_text(json.dumps(rows,indent=2)+'\n')
