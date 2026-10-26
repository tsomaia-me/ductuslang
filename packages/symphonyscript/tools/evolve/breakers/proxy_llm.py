import json
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from types import SimpleNamespace

from Fuzz4All.util.util import simple_parse

RETRY_SECONDS = 5
RETRIED_STATUSES = {429, 500, 502, 503, 504, 529}
THINKING = {"type": "adaptive"}
GENERATION_EFFORT = "high"
REQUIREMENTS_EFFORT = "high"
REQUIREMENTS_MAX_TOKENS = 32000


class ProxyRefused(Exception):
    pass


class ProxyClient:
    def __init__(self, url):
        self.url = url.rstrip("/") + "/v1/messages"
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def create(self, params):
        request = urllib.request.Request(self.url, data=json.dumps(params).encode(),
                                         headers={"content-type": "application/json"}, method="POST")
        while True:
            try:
                with self.opener.open(request) as response:
                    return json.loads(response.read())
            except urllib.error.HTTPError as error:
                body = error.read().decode(errors="replace")
                if error.code not in RETRIED_STATUSES:
                    raise ProxyRefused(f"{error.code}: {body}")
            except urllib.error.URLError:
                pass
            time.sleep(RETRY_SECONDS)


def text_of(message):
    return "".join(block.get("text", "") for block in message["content"] if block["type"] == "text")


def request_engine(client, models, config):
    system = "\n".join(m["content"] for m in config["messages"] if m["role"] == "system")
    messages = [m for m in config["messages"] if m["role"] != "system"]
    params = {"model": next(models), "max_tokens": max(config["max_tokens"], REQUIREMENTS_MAX_TOKENS),
              "thinking": THINKING, "output_config": {"effort": REQUIREMENTS_EFFORT}, "messages": messages}
    if system:
        params["system"] = system
    reply = client.create(params)
    return SimpleNamespace(choices=[SimpleNamespace(message=SimpleNamespace(content=text_of(reply)))])


class ProxyModel:
    def __init__(self, client, model, eos, system, max_tokens):
        self.client = client
        self.model = model
        self.system = system
        self.max_tokens = max_tokens
        self.stop_sequences = [stop for stop in dict.fromkeys(eos) if stop and stop.strip()]

    def complete(self, prompt):
        reply = self.client.create({
            "model": self.model, "max_tokens": self.max_tokens, "system": self.system,
            "thinking": THINKING, "output_config": {"effort": GENERATION_EFFORT},
            "stop_sequences": self.stop_sequences, "messages": [{"role": "user", "content": prompt}]})
        content = text_of(reply)
        return simple_parse(content) or content

    def generate(self, prompt, batch_size=10, temperature=1.0, max_length=512):
        with ThreadPoolExecutor(max_workers=batch_size) as pool:
            return list(pool.map(lambda _: self.complete(prompt), range(batch_size)))


def generation_system(spec, samples):
    parts = [spec]
    for path in sorted(Path(samples).glob("*.sym")):
        parts.append(f"\n\nValid program {path.name}:\n\n{path.read_text()}")
    return "".join(parts)


def make_model_factory(client, system):
    def make_model(eos, model_name, device, max_length):
        return ProxyModel(client, model_name, eos, system, max_length)
    return make_model
