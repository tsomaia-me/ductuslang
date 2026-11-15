import re, sys
spec = open(sys.argv[1], encoding='utf-8').read().split('\n')
out = []
section = '0'
counts = {}
i = 0
while i < len(spec):
    line = spec[i]
    m = re.match(r'^#{2,3} (\d+(?:\.\d+)?)\.? ', line)
    if m:
        section = m.group(1)
    m = re.match(r'^```(sym|sym-error|sym-package)\s*$', line)
    if m:
        tag = m.group(1)
        j = i + 1
        body = []
        while not spec[j].startswith('```'):
            body.append(spec[j]); j += 1
        key = (section, tag)
        counts[key] = counts.get(key, 0) + 1
        # reason text: next non-empty line after the block
        k = j + 1
        while k < len(spec) and not spec[k].strip():
            k += 1
        reason = spec[k] if tag == 'sym-error' else ''
        out.append((f'§{section} {counts[key]} {tag}', '\n'.join(body) + '\n', reason, i + 1))
        i = j
    i += 1
want = sys.argv[2]
with open(sys.argv[3], 'w', encoding='utf-8') as f:
    for name, body, reason, line in out:
        if name.endswith(' ' + want):
            f.write(f'#### {name} L{line}\n{body}')
if want == 'sym-error':
    for name, body, reason, line in out:
        if name.endswith(' sym-error'):
            print(name, f'L{line}', '::', reason[:160])
print(len([1 for n,_,_,_ in out if n.endswith(' ' + want)]), want, 'examples', file=sys.stderr)
