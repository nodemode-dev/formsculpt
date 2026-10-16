import inspect
import math
from dataclasses import dataclass, field
from typing import Any, Callable


class CommandError(Exception):
    def __init__(self, message: str, status: int = 400):
        super().__init__(message)
        self.status = status


@dataclass
class Spec:
    id: str
    label: str
    fn: Callable[..., Any]
    params: dict[str, dict] = field(default_factory=dict)
    group: str = ""
    description: str = ""


REGISTRY: dict[str, Spec] = {}


def command(cid: str, label: str, params: dict[str, dict] | None = None, group: str = "", description: str = "") -> Callable:
    def wrap(fn: Callable) -> Callable:
        if cid in REGISTRY:
            raise RuntimeError(f"command registered twice: {cid}")
        REGISTRY[cid] = Spec(cid, label, fn, params or {}, group or cid.split(".")[0], description or label)
        return fn
    return wrap


def _typed(cid: str, name: str, schema: dict, value: Any) -> Any:
    kind = schema.get("type")
    if value is None:
        return None
    bad = CommandError(f"{cid}: {name} must be {'an' if kind in ('integer', 'object') else 'a'} {kind}")
    if kind == "integer":
        if isinstance(value, bool) or not isinstance(value, (int, float)) or (isinstance(value, float) and not value.is_integer()):
            raise bad
        value = int(value)
    elif kind == "number":
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
            raise bad
    elif kind == "boolean":
        if not isinstance(value, bool):
            raise bad
    elif kind == "string":
        if not isinstance(value, str):
            raise bad
    return value


def dispatch(cid: str, params: dict | None = None) -> Any:
    spec = REGISTRY.get(cid)
    if spec is None:
        raise CommandError(f"Unknown command: {cid}", 404)
    if params is not None and not isinstance(params, dict):
        raise CommandError(f"{cid}: parameters must be an object")
    params = dict(params or {})
    for name, schema in spec.params.items():
        if name in params and isinstance(schema, dict) and schema.get("type"):
            params[name] = _typed(cid, name, schema, params[name])
    sig = inspect.signature(spec.fn)
    unknown = sorted(set(params) - set(sig.parameters))
    if unknown:
        raise CommandError(f"{cid} does not take {', '.join(unknown)}")
    try:
        return spec.fn(**params)
    except CommandError:
        raise
    except (ValueError, KeyError, TypeError, FileNotFoundError) as exc:
        raise CommandError(f"{spec.label}: {exc}") from exc


def catalogue() -> list[dict]:
    return [{"id": s.id, "label": s.label, "group": s.group, "params": s.params, "description": s.description} for s in sorted(REGISTRY.values(), key=lambda s: s.id)]
