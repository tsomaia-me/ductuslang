# For single-fault cases that drew no diagnostic: group by class and the edited text,
# and print the edited line before and after, so each can be judged valid or not.
import sys, re, collections
text = open(sys.argv[1], encoding='utf-8').read()
cls = sys.argv[2]
per = int(sys.argv[3])
groups = collections.defaultdict(list)
for c in text.split('=== case ')[1:]:
    head, rest = c.split('\n', 1)
    m = re.match(r'(\d+) category (\S+) faults (\d+) diags (\d+) lexdiags (\d+) edits \[\("([^"]+)", (\d+), (\d+), (.*)\)\]', head)
    if not m or m.group(2) != cls or int(m.group(4)) != 0:
        continue
    at, dl = int(m.group(7)), int(m.group(8))
    ins = m.group(9).strip('"\'')
    orig = rest.split('--- orig\n',1)[1].split('\n--- mutated\n',1)[0]
    mut = rest.split('\n--- mutated\n',1)[1].split('\n--- diags',1)[0]
    s = orig.rfind('\n', 0, at) + 1
    e = orig.find('\n', at); e = len(orig) if e < 0 else e
    line = orig[s:e]
    edited = orig[at:at+dl] if dl else ins
    key = edited.strip() or repr(edited)
    ms = mut.rfind('\n', 0, at) + 1
    me = mut.find('\n', at + len(ins)); me = len(mut) if me < 0 else me
    groups[key].append((line, mut[ms:me]))
tot = sum(len(v) for v in groups.values())
print('total', tot)
for k in sorted(groups, key=lambda k: -len(groups[k])):
    v = groups[k]
    print(f'## {k!r} x{len(v)}')
    seen = set()
    n = 0
    for a, b in v:
        if (a, b) in seen: continue
        seen.add((a, b)); n += 1
        print(f'    {a!r}\n -> {b!r}')
        if n >= per: break
