"""
MCP Tools Node: Exposes file and command operations for wrt automation via MCP protocol.
Implements read_file, write_file, run_cmd, and git_diff as MCP tools.
"""
import subprocess, pathlib
from mcp.server import Tool, Server, run_stdio_server
ROOT = pathlib.Path("/workspace").resolve()

def read_file(path: str) -> str: return (ROOT / path).read_text()

def write_file(path: str, content: str) -> str:
    p = ROOT / path; p.parent.mkdir(parents=True, exist_ok=True); p.write_text(content); return "ok"

def run_cmd(cmd: str) -> str: return subprocess.check_output(cmd, shell=True, text=True)

def git_diff() -> str: return subprocess.check_output("git diff -U0", shell=True, text=True)

Server(tools=[Tool.from_function(f) for f in (read_file, write_file, run_cmd, git_diff)])
run_stdio_server() 