import sys
from types import ModuleType, SimpleNamespace


def make_model(**kwargs):
    raise RuntimeError("Fuzz4All's local model is not installed; model calls go through the proxy")


model = ModuleType("Fuzz4All.model")
model.make_model = make_model
torch = ModuleType("torch")
torch.cuda = SimpleNamespace(empty_cache=lambda: None)
sys.modules["torch"] = torch
sys.modules["Fuzz4All.model"] = model
