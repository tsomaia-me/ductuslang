import gzip
import hashlib
import json

MAIN = "main"
SECRETS = ("sk-ant", "api_key")


def key(program):
    sources = sorted([source["path"], source["text"]] for source in program["sources"])
    encoded = json.dumps([program.get("vocabulary", MAIN), sources], ensure_ascii=False)
    return hashlib.sha256(encoded.encode()).hexdigest()[:16]


def holds_secret(program):
    return any(secret in source["text"] for source in program["sources"] for secret in SECRETS)


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
