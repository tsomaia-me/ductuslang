# Converts a jsonl.gz corpus (one JSON object a line, texts under sources[].text)
# into a length-prefixed binary: u32 LE length, then UTF-8 bytes, per source text.
import gzip, json, struct, sys
src, dst = sys.argv[1], sys.argv[2]
n = 0
with gzip.open(src, 'rt', encoding='utf-8') as f, open(dst, 'wb') as out:
    for line in f:
        line = line.strip()
        if not line:
            continue
        obj = json.loads(line)
        for s in obj.get('sources', []):
            b = s['text'].encode('utf-8')
            out.write(struct.pack('<I', len(b)))
            out.write(b)
            n += 1
print(dst, n)
