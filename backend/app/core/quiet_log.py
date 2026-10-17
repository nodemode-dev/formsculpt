"""Keeps the terminal readable: the window asks the service for news about once a second (agent activity, the
version, the scene after each change), and uvicorn's access log printed a line for every one of those requests.
Successful requests are no longer printed; a request that fails (status 400 and up) still is, as are errors and
startup lines. FSC_ACCESS_LOG=1 prints every request again, for debugging."""

import logging
import os


class _FailuresOnly(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        # uvicorn's access records carry (client, method, path, http version, status) as their args.
        args = record.args
        if isinstance(args, tuple) and len(args) >= 5 and isinstance(args[4], int):
            return args[4] >= 400
        return True


def install() -> None:
    if os.environ.get("FSC_ACCESS_LOG") == "1":
        return
    access = logging.getLogger("uvicorn.access")
    if not any(isinstance(f, _FailuresOnly) for f in access.filters):
        access.addFilter(_FailuresOnly())
