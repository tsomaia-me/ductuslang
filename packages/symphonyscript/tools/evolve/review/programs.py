import gzip
import hashlib
import json

MAIN = "main"
SECRETS = ("sk-ant", "api_key")
PROBES = "probes"
SEPARATOR = "----"
TAG = "#"
EXPECT = "#expect"
DEFAULT_TAG = "example"
PROBE_VOCABULARIES = {"example": MAIN, "native": "native", "bounded": "bounded", "envelope": "envelope",
                      "bar": "bar", "signature": "signature", "to": "to"}


def key(program):
    sources = sorted([source["path"], source["text"]] for source in program["sources"])
    encoded = json.dumps([program.get("vocabulary", MAIN), sources], ensure_ascii=False)
    return hashlib.sha256(encoded.encode()).hexdigest()[:16]


def holds_secret(program):
    return any(secret in source["text"] for source in program["sources"] for secret in SECRETS)


def probe_lines(text):
    lines = text.split("\n")
    if lines[-1] == "":
        lines.pop()
    return [line.removesuffix("\r") for line in lines]


def probe_program(lines):
    filled = [index for index, (_, line) in enumerate(lines) if line.strip()]
    if not filled:
        return []
    body, tag = lines[filled[0]:], DEFAULT_TAG
    if body[0][1].strip().startswith(TAG):
        tag, body = body[0][1].strip().removeprefix(TAG).strip(), body[1:]
    last = [index for index, (_, line) in enumerate(body) if line.strip()]
    if last:
        stated = body[last[-1]][1].strip()
        rest = stated.removeprefix(EXPECT)
        if stated.startswith(EXPECT) and (not rest or rest[0].isspace()):
            body = body[:last[-1]]
    first = body[0][0] if body else lines[filled[0]][0]
    return [(first, tag, "".join(f"{line}\n" for _, line in body))]


def probe_programs(text):
    programs, part = [], []
    for number, line in enumerate(probe_lines(text), 1):
        if line.rstrip() == SEPARATOR:
            programs += probe_program(part)
            part = []
        else:
            part.append((number, line))
    return programs + probe_program(part)


def probe_records(record):
    [source] = record["sources"]
    if not any(line.rstrip() == SEPARATOR or line.lstrip().startswith(TAG) for line in probe_lines(source["text"])):
        return [record]
    records = []
    for line, tag, text in probe_programs(source["text"]):
        vocabulary = PROBE_VOCABULARIES.get(tag)
        records.append({"origin": f"{record['origin']}:{line}", "sources": [{"path": source["path"], "text": text}],
                        "vocabulary": vocabulary, **({} if vocabulary else {"tag": tag})})
    return records


def size(program):
    return sum(len(source["text"]) for source in program["sources"])


def read_jsonl(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt", encoding="utf-8") as lines:
        return [json.loads(line) for line in lines if line.strip()]


def write_jsonl_gz(path, records):
    with open(path, "wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as packed:
        for record in records:
            packed.write((json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n").encode())
