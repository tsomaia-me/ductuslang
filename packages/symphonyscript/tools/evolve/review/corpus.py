import argparse
import os
from pathlib import Path

from programs import holds_secret, key, read_jsonl, write_jsonl_gz

SOURCE_PATH = "main.sym"
SINGLE_PROGRAM_SUFFIXES = (".sym", ".fuzz")
TARGETED_PROGRAMS = "programs.jsonl"


def found(path, root):
    files = sorted(file for file in path.rglob("*") if file.is_file()) if path.is_dir() else [path]
    for file in files:
        origin = Path(os.path.relpath(file, root)).as_posix()
        if file.suffix in SINGLE_PROGRAM_SUFFIXES:
            text = file.read_text(encoding="utf-8", errors="replace")
            yield {"origin": origin, "sources": [{"path": SOURCE_PATH, "text": text}]}
        elif file.name == TARGETED_PROGRAMS:
            for record in read_jsonl(file):
                yield {"origin": f"{origin}#{record['id']}", "stated": record["stated"],
                       "sources": [{"path": SOURCE_PATH, "text": record["program"]}]}


def add(corpus, paths, root):
    records = read_jsonl(corpus) if corpus.exists() else []
    known = {record["key"] for record in records}
    added = held = secret = 0
    for path in paths:
        for program in found(path, root):
            if holds_secret(program):
                secret += 1
                continue
            program_key = key(program)
            if program_key in known:
                held += 1
                continue
            known.add(program_key)
            records.append({"key": program_key, **program})
            added += 1
    write_jsonl_gz(corpus, records)
    return {"added": added, "held": held, "secret": secret, "total": len(records)}


def main():
    parser = argparse.ArgumentParser(
        description="Append .sym and .fuzz programs and targeted programs.jsonl files to a corpus, "
                    "one program per text, without programs holding a secret.")
    parser.add_argument("corpus", type=Path, help="tools/evolve/corpora/<name>.jsonl.gz")
    parser.add_argument("paths", type=Path, nargs="+", help="files or directories to walk")
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="origins are relative to this directory")
    arguments = parser.parse_args()
    counts = add(arguments.corpus, arguments.paths, arguments.root)
    print(f"{arguments.corpus.name}: {counts['added']} added, {counts['held']} already held, "
          f"{counts['secret']} dropped for holding a secret, {counts['total']} in all")


if __name__ == "__main__":
    main()
