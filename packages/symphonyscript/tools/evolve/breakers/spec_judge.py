import json
import os
import re
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from oracle import SOURCE_PATH, digest
from proxy_llm import ProxyRefused, text_of

JUDGE_MODEL = "claude-opus-5-5"
JUDGE_EFFORT = "high"
JUDGE_MAX_TOKENS = 16000
JUDGE_THREADS = 4
KEYWORDS = re.compile(r"^### 2\.5 .*?^```text\n(.*?)^```", re.MULTILINE | re.DOTALL)
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
ANSWER = {
    "type": "object",
    "properties": {"verdict": {"type": "string", "enum": ["accept", "reject"]}, "rule": {"type": "string"}},
    "required": ["verdict", "rule"],
    "additionalProperties": False,
}
TASK = ("Judge the SymphonyScript program below against the specification in the system prompt. "
        f"The program is one file, {SOURCE_PATH}, compiled against the example vocabulary of section 1.3. "
        "It is accepted when it compiles with warnings at most, and rejected when the specification "
        "requires an error for it.")
ANSWER_FORM = ("Reply with JSON only: verdict is accept or reject, and rule is the SPEC section and the "
               "sentence of it that decides the verdict.")
ARGUE_OPPOSITE = ("Before you decide, argue for the verdict opposite to the one you first reach, "
                  "then decide.")
INSTRUCTIONS = (f"{TASK} {ANSWER_FORM}", f"{TASK} {ARGUE_OPPOSITE} {ANSWER_FORM}")


def spec_keywords(spec):
    return set(KEYWORDS.search(spec).group(1).split())


def novelty(text, errors, keywords):
    if errors:
        return "codes:" + ",".join(sorted({error["code"] for error in errors}))
    kinds = sorted({word.group(0) for line in text.splitlines()
                    if line.strip() and not line[0].isspace() and (word := WORD.match(line))})
    constructs = sorted(keywords & set(WORD.findall(text)))
    return "clean:" + ",".join(kinds) + "|" + ",".join(constructs)


def answer_of(reply):
    try:
        answer = json.loads(text_of(reply))
        if answer["verdict"] in ("accept", "reject") and isinstance(answer["rule"], str):
            return answer
    except (ValueError, KeyError, TypeError):
        pass
    return None


class SpecJudge:
    def __init__(self, client, spec, keep, claims, log, every, generated=0):
        self.client = client
        self.spec = spec
        self.keywords = spec_keywords(spec)
        self.keep = keep
        self.claims = Path(claims)
        self.claims.mkdir(parents=True, exist_ok=True)
        self.log = Path(log)
        self.every = every
        self.generated = generated
        self.judged = len(self.log.read_text().splitlines()) if self.log.exists() else 0
        self.pool = ThreadPoolExecutor(max_workers=JUDGE_THREADS)
        self.pending = []

    def claim(self, key):
        try:
            os.close(os.open(self.claims / digest(key), os.O_CREAT | os.O_EXCL | os.O_WRONLY))
            return True
        except FileExistsError:
            return False

    def consider(self, program_id, text, errors):
        self.generated += 1
        if errors is None or self.judged * self.every >= self.generated:
            return False
        key = novelty(text, errors, self.keywords)
        if not self.claim(key):
            return False
        self.judged += 1
        with open(self.log, "a") as log:
            log.write(json.dumps({"id": program_id, "key": key}) + "\n")
        self.pending.append(self.pool.submit(self.decide, program_id, text, errors, key))
        return True

    def ask(self, instruction, text):
        return answer_of(self.client.create({
            "model": JUDGE_MODEL, "max_tokens": JUDGE_MAX_TOKENS, "system": self.spec,
            "output_config": {"effort": JUDGE_EFFORT, "format": {"type": "json_schema", "schema": ANSWER}},
            "messages": [{"role": "user", "content": f"{instruction}\n\n```sym\n{text}\n```"}]}))

    def decide(self, program_id, text, errors, key):
        try:
            with ThreadPoolExecutor(max_workers=len(INSTRUCTIONS)) as both:
                answers = list(both.map(lambda instruction: self.ask(instruction, text), INSTRUCTIONS))
        except ProxyRefused as refused:
            print(f"the proxy refused a judge call: {refused}", file=sys.stderr, flush=True)
            return False
        compiler = "reject" if errors else "accept"
        if None in answers or answers[0]["verdict"] != answers[1]["verdict"] or answers[0]["verdict"] == compiler:
            return False
        finding_key = f"verdict|{answers[0]['verdict']}|{key}"
        return self.keep(f"verdict-{digest(finding_key)}.json", {
            "signal": "verdict", "key": finding_key, "id": program_id, "compiler": compiler,
            "judges": answers[0]["verdict"], "rules": [answer["rule"] for answer in answers],
            "codes": sorted({error["code"] for error in errors}), "diagnostics": errors,
            "sources": [{"path": SOURCE_PATH, "text": text}]})

    def close(self):
        self.pool.shutdown(wait=True)
        for future in self.pending:
            future.result()
