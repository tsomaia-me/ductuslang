# Groups the cases a `single`/`multi` run wrote, and prints the smallest of each group.
import sys, re, collections
path = sys.argv[1]
per = int(sys.argv[2]) if len(sys.argv) > 2 else 2
filt = sys.argv[3] if len(sys.argv) > 3 else None
text = open(path, encoding='utf-8').read()
cases = text.split('=== case ')[1:]
groups = collections.defaultdict(list)
for c in cases:
    head, rest = c.split('\n', 1)
    m = re.match(r'(\d+) category (\S+) faults (\d+) diags (\d+) lexdiags (\d+) edits (.*)', head)
    cat, faults, diags, lexd, edits = m.group(2), int(m.group(3)), int(m.group(4)), int(m.group(5)), m.group(6)
    orig = rest.split('--- orig\n',1)[1].split('\n--- mutated\n',1)[0]
    mut = rest.split('\n--- mutated\n',1)[1].split('\n--- diags',1)[0]
    dl = rest.split('\n--- diags\n',1)[1].split('--- error nodes',1)[0].strip().split('\n') if diags else []
    codes = tuple(sorted(set(d.split()[0] for d in dl if d.strip())))
    # inserted / deleted text
    em = re.findall(r"\('([^']*)', (\d+), (\d+), \"?'?(.*?)'?\"?\)", edits)
    key = (cat, 'fewer' if diags < faults else 'more', codes)
    groups[key].append((len(mut), mut, dl, edits, orig))
for key in sorted(groups, key=lambda k: -len(groups[k])):
    if filt and filt not in str(key):
        continue
    v = sorted(groups[key], key=lambda x: x[0])
    print(f'##### {key} count={len(v)}')
    for n, mut, dl, edits, orig in v[:per]:
        print('  edits', edits)
        print('  mutated:', repr(mut))
        for d in dl: print('   ', d)
