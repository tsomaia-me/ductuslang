import glob
import gzip
import json
import os
import struct
import sys

ROOT = sys.argv[1]
OUT = sys.argv[2]
os.makedirs(OUT, exist_ok=True)


def pack(name, texts):
    with open(os.path.join(OUT, name + ".pack"), "wb") as out:
        for text in texts:
            data = text.encode("utf-8")
            out.write(struct.pack("<I", len(data)))
            out.write(data)


def placements(i):
    head = f"    Lamp as wash{i} /0.7 | "
    pad = " " * len(head)
    return (
        f"Rig rig{i}:\n"
        f"    Lamp/0.5 | hue=0.2 strobe\n"
        f"{head}level=0.3\n"
        f"{pad}hue=0.9\n"
        f"    Lamp*/0.25 Lamp/0.5 Lamp/0.75\n"
        f"    [Lamp/0.2 Lamp/0.5 Lamp/0.8]\n"
        f"    Scene as dusk{i} | hold=8:\n"
        f"        Cue/0 Cue/2\n"
        f"        Cue/4 | fade=500ms\n"
        f"        Dims | depth=0.5: wash{i}\n"
    )


def expressions(i):
    return (
        f"state a{i}(1)\n"
        f"state b{i}(0.5)\n"
        f"state c{i} = (a{i} + 3 * a{i} div 2 - 7 % 4) ** 2 > 10 and not (b{i} < 0.25) or a{i} is 3\n"
        f"state d{i} = if c{i} then b{i} * 0.8 + 1.0 / 3 else -b{i} ** 2\n"
        f"let k{i} = [1, 2, 3, 4][{i} % 4] + max(3, 4)\n"
        f"state e{i} = [a{i}, a{i} + 1, a{i} * 2][a{i} % 3]\n"
    )


def strings(i):
    return (
        f'state n{i}(""): str[..32]\n'
        f'state t{i} = "item {{a{i}}} of {{b{i} + 1}}: {{n{i}}} // no comment \\{{ok}}"\n'
        f'state w{i} = "kit/{{k{i}}}.wav" + t{i}[1..^1]\n'
    )


def functions(i):
    return (
        f"fn fun{i}(x, by: i32 = 2) -> i32:\n"
        f"    let twice: i32 = x * 2\n"
        f"    let rest = twice div by\n"
        f"    if rest > 3 then rest + by else rest - by\n"
        f"pattern P{i}(default root, steps = 4) -> (Note*):\n"
        f"    for j in 0..steps:\n"
        f"        Note/(root + j * 2) | velocity=0.5\n"
    )


def plainfns(i):
    return (
        f"fn fun{i}(x, by = 2):\n"
        f"    let twice = x * 2\n"
        f"    let rest = twice div by\n"
        f"    if rest > 3 then rest + by else rest - by\n"
        f"pattern P{i}(root, steps = 4):\n"
        f"    for j in 0..steps:\n"
        f"        Note/(root + j * 2) | velocity=0.5\n"
    )


def blocks(i):
    return (
        f"Plant p{i}:\n"
        f"    Valve/0.1\n"
        f"    when alarm{i}:\n"
        f"        Valve/0.3 Valve/0.5\n"
        f"    else:\n"
        f"        Valve/0.3\n"
        f"    when:\n"
        f"        phase is 0: Valve/0.6\n"
        f"        phase is 1: Valve/0.7\n"
        f"        else: Valve/0.9\n"
        f"    for q in 0..4:\n"
        f"        Valve/(0.1 + q) when q > 1\n"
        f"state m{i} = match mode:\n"
        f"    Auto: 1.0\n"
        f"    Manual: 0.8\n"
        f"    else: 0.6\n"
    )


def documents(i):
    return (
        f'state body{i}(json {{"items": [1, 2.5, "x", true, null], "user": {{"name": "bob", "tags": ["a", "b"]}}}}): json[..4096]\n'
        f'state r{i} = json {{"ok": true, "count": count, "echo": (a + 1), "who": body{i}}}\n'
    )


def unicode(i):
    return (
        f"state größe{i}(1)\n"
        f"state 日本{i} = größe{i} + 1\n"
        f"Rig bühne{i}:\n"
        f"    Lamp/0.5 | hue=0.2\n"
    )


FAULTS = [
    "    Lamp ) | =\n",
    "    Lamp/ | hue=\n",
    "    [Lamp/0.2 Lamp\n",
    "    Lamp/0.5 | hue=(1 + \n",
    '    Lamp | hue="open\n',
]


def faulty(i):
    return placements(i) + FAULTS[i % len(FAULTS)]


SHAPES = {
    "placements": placements,
    "expressions": expressions,
    "strings": strings,
    "functions": functions,
    "plainfns": plainfns,
    "blocks": blocks,
    "documents": documents,
    "unicode": unicode,
    "faulty": faulty,
}

for name, shape in SHAPES.items():
    per = shape(0).count("\n")
    for lines in (10000, 20000, 40000, 80000):
        text = "".join(shape(i) for i in range(lines // per))
        pack(f"gen_{name}_{lines}", [text])

large = sorted(glob.glob(os.path.join(ROOT, "crates/fixtures/projects/large/*.sym")))
pack("large", [open(path, encoding="utf-8").read() for path in large])
pack("large_one_file", ["".join(open(path, encoding="utf-8").read() for path in large)])

texts = []
with gzip.open(os.path.join(ROOT, "tools/evolve/corpora/breakers.jsonl.gz"), "rt", encoding="utf-8") as lines:
    for line in lines:
        for source in json.loads(line)["sources"]:
            texts.append(source["text"])
pack("corpus", texts)
print(len(large), "large files;", len(texts), "corpus sources")
