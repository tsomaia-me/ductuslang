import json
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from types import SimpleNamespace

from Fuzz4All.util.util import simple_parse

RETRY_SECONDS = 5
RETRIED_STATUSES = {429, 500, 502, 503, 504, 529}
THINKING_OFF = {"claude-sonnet-5-5": {"type": "between_tools"}, "claude-haiku-5-5": {"type": "disabled"}}
COMPLETION_SYSTEM = ("You are a code completion model. Continue the text the user sends as source code: "
                     "reply with SymphonyScript source only.")


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


def with_thinking_off(params):
    if params["model"] in THINKING_OFF:
        return {**params, "thinking": THINKING_OFF[params["model"]]}
    return params


def text_of(message):
    return "".join(block.get("text", "") for block in message["content"] if block["type"] == "text")


def request_engine(client, model, config):
    system = "\n".join(m["content"] for m in config["messages"] if m["role"] == "system")
    messages = [m for m in config["messages"] if m["role"] != "system"]
    params = {"model": model, "max_tokens": config["max_tokens"], "messages": messages}
    if system:
        params["system"] = system
    reply = client.create(with_thinking_off(params))
    return SimpleNamespace(choices=[SimpleNamespace(message=SimpleNamespace(content=text_of(reply)))])


class ProxyModel:
    def __init__(self, client, model, eos):
        self.client = client
        self.model = model
        self.stop_sequences = [stop for stop in dict.fromkeys(eos) if stop and stop.strip()]

    def complete(self, prompt, max_length):
        reply = self.client.create(with_thinking_off({
            "model": self.model, "max_tokens": max_length, "system": COMPLETION_SYSTEM,
            "stop_sequences": self.stop_sequences, "messages": [{"role": "user", "content": prompt}]}))
        content = text_of(reply)
        return simple_parse(content) or content

    def generate(self, prompt, batch_size=10, temperature=1.0, max_length=512):
        with ThreadPoolExecutor(max_workers=batch_size) as pool:
            return list(pool.map(lambda _: self.complete(prompt, max_length), range(batch_size)))


def make_model_factory(client):
    def make_model(eos, model_name, device, max_length):
        return ProxyModel(client, model_name, eos)
    return make_model
