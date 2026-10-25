import hashlib
import json
import os
import subprocess
import tempfile
from pathlib import Path

from Fuzz4All.target.target import FResult

WARNING_CODES = {"integer-input", "unreachable-instance", "no-entry-instance"}
SOURCE_PATH = "main.sym"


def program(program_id, text):
    return {"id": program_id, "sources": [{"path": SOURCE_PATH, "text": text}]}


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()[:16]


class Oracle:
    def __init__(self, check, shrink, findings):
        self.check = str(check)
        self.shrink = str(shrink)
        self.findings = Path(findings)
        self.findings.mkdir(parents=True, exist_ok=True)
        self.spec_judge = None

    def keep(self, name, record):
        path = self.findings / name
        if path.exists():
            return False
        with tempfile.NamedTemporaryFile("w", dir=self.findings, suffix=".tmp", delete=False) as out:
            json.dump(record, out, indent=1)
        try:
            os.link(out.name, path)
            return True
        except FileExistsError:
            return False
        finally:
            os.unlink(out.name)

    def record(self, work, entry, signal):
        program_path = work / "program.json"
        program_path.write_text(json.dumps(entry))
        shrunk = subprocess.run([self.shrink, str(program_path), signal], capture_output=True, text=True)
        original = entry["sources"]
        if shrunk.returncode != 0:
            return self.keep(f"unshrunk-{signal}-{digest(original[0]['text'])}.json",
                             {"signal": signal, "id": entry["id"], "sources": original, "shrink_error": shrunk.stderr})
        result = json.loads(shrunk.stdout)
        return self.keep(f"{signal}-{digest(result['key'])}.json", {**result, "original_sources": original})

    def judge(self, program_id, text):
        result, message, errors, parses = self.verdict_of(program_id, text)
        if self.spec_judge is not None:
            self.spec_judge.consider(program_id, text, errors, parses)
        return result, message

    def verdict_of(self, program_id, text):
        entry = program(program_id, text)
        with tempfile.TemporaryDirectory(dir=self.findings.parent) as directory:
            work = Path(directory)
            (work / "programs.jsonl").write_text(json.dumps(entry) + "\n")
            checked = subprocess.run([self.check, str(work / "programs.jsonl"), str(work / "verdicts.jsonl")],
                                     capture_output=True, text=True)
            if checked.returncode < 0:
                self.keep(f"process-{-checked.returncode}-{digest(text)}.json",
                          {"signal": f"process exited on signal {-checked.returncode}", "id": program_id,
                           "sources": entry["sources"]})
                return FResult.ERROR, f"evolve-check exited on signal {-checked.returncode}", None, False
            if checked.returncode != 0:
                return FResult.FAILURE, checked.stderr, None, False
            verdict = json.loads((work / "verdicts.jsonl").read_text())
            if verdict["signals"]:
                for signal in verdict["signals"]:
                    self.record(work, entry, signal)
                return FResult.ERROR, json.dumps(verdict["findings"]), None, False
        errors = [d for d in verdict["diagnostics"] if d["code"] not in WARNING_CODES]
        if errors:
            return FResult.FAILURE, " ".join(d["code"] for d in errors), errors, verdict["parses"]
        return FResult.SAFE, "", errors, verdict["parses"]
