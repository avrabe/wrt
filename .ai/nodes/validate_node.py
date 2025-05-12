"""
Validate Node: Runs build, test, and lint commands for wrt automation.
Logs results and returns pass/fail status for each validation step.
"""
import subprocess, shlex, pathlib, os, json
from typing import Dict, List

_LOGROOT = pathlib.Path(".ai_runs")

CMD_KEYS = ["std_build", "nostd_build", "test", "clippy"]


def _run(cmd: str, logfile: pathlib.Path) -> bool:
    logfile.parent.mkdir(parents=True, exist_ok=True)
    with logfile.open("w") as f:
        proc = subprocess.run(shlex.split(cmd), stdout=f, stderr=subprocess.STDOUT)
    return proc.returncode == 0


def validate_node(state: Dict, env) -> Dict:
    cfg = state  # contains cmd strings from ai.toml
    ticket = cfg["ticket"]
    crate  = cfg["crate"]

    passed: List[str] = []
    failed: List[str] = []

    logdir = _LOGROOT / str(ticket) / crate

    for key in CMD_KEYS:
        cmd = cfg.get(key)
        if not cmd:
            continue
        ok = _run(cmd, logdir / f"{key}.log")
        (passed if ok else failed).append(key)

    green = len(failed) == 0
    return {**state, "green": green, "validate_passed": passed, "validate_failed": failed} 