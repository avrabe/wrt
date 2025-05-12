# -----------------------------------------------------------------------------
# README.md  (place at repo root)
# -----------------------------------------------------------------------------
**WRT AI Flow** is a self‑contained LangGraph workflow that lets AI coding agents
(Claude, Cursor, etc.) iterate safely on the `wrt` WebAssembly runtime.  It
ensures every automated patch passes the repo's own success matrix:

* `cargo build` with **std** features
* `cargo build` with **no_std + alloc** features
* Full crate‑level `cargo test`
* `cargo clippy` with **-D warnings**

When all four stages are green, the agent opens a draft PR that follows the
Conventional Commit spec.

---
## Directory layout
```
.ai/
 ├── config/ai.toml            # per‑crate build/test commands
 ├── flows/                    # LangGraph DAG drivers
 │   ├── base_dag.py           # plan → code → validate → pr
 │   ├── runtime_flow.py       # runtime crate driver
 │   ├── decoder_flow.py       # decoder crate driver
 │   └── component_flow.py     # component‑model driver
 └── nodes/                    # individual DAG states
     ├── plan_node.py          # ticket → bullet plan (Claude)
     ├── code_node.py          # plan → git diff (Conventional Commit)
     ├── validate_node.py      # build + test + clippy matrix
     └── pr_node.py            # draft PR with status table
Dockerfile.claude / Dockerfile.cursor  # reproducible agent images
```

---
## Prerequisites
* Docker 24+ (with [Buildx](https://docs.docker.com/buildx/working-with-buildx/) enabled)
* Python 3.10+ (inside Docker is fine)
* Rust 1.86 (inside Docker image)
* An API key for **Anthropic Claude** *or* **Cursor CLI**
* (Optional) GitHub token with `repo` scope – enables PR creation

> **Note:** If using Colima or a non-default Docker socket, set:
> ```bash
> export DOCKER_HOST=unix:///Users/r/.colima/default/docker.sock
> ```

---
## One‑time setup
```bash
# clone the repo
$ git clone https://github.com/avrabe/wrt && cd wrt

# build the Claude image (≈2 min on x86‑64)
$ docker buildx build --load -f docker/Dockerfile.claude -t wrt-agent-claude .
# or build the Cursor variant
$ docker buildx build --load -f docker/Dockerfile.cursor -t wrt-agent-cursor .
```

---
## Running a flow locally
Trigger the **runtime** agent for GitHub Issue #42:
```bash
export CLAUDE_KEY="sk-ant-…"        # or CURSOR_API_KEY
export GH_TOKEN="ghp_…"             # optional
export GITHUB_REPOSITORY="avrabe/wrt"
export TICKET=42

# If using Colima, ensure DOCKER_HOST is set:
export DOCKER_HOST=unix:///Users/r/.colima/default/docker.sock

docker run --rm -it \
  -e CLAUDE_KEY -e GH_TOKEN -e GITHUB_REPOSITORY -e TICKET \
  -v "$(pwd)":/workspace \
  wrt-agent-claude \
  python .ai/flows/runtime_flow.py
```
You'll see LangGraph steps stream; logs are saved under `.ai_runs/`.

Switch crates by launching a different flow script, e.g.:
```bash
docker run … python .ai/flows/decoder_flow.py
```

Use Cursor backend by replacing `wrt-agent-claude` with `wrt-agent-cursor` and
exporting `CURSOR_API_KEY` instead of `CLAUDE_KEY`.

---
## Cleaning up
```bash
rm -rf .ai_runs/               # delete logs & checkpoints
docker rmi wrt-agent-claude    # free disk space
```

---
### Troubleshooting & Checks
- **Check Buildx:**
  ```bash
  docker buildx version
  docker buildx ls
  ```
  If not available, see [Docker Buildx docs](https://docs.docker.com/buildx/working-with-buildx/).
- **Check Docker Host:**
  ```bash
  echo $DOCKER_HOST
  docker info
  ```
  Make sure it matches your Colima or Docker Desktop socket.

---
### FAQ
* **Does it push to `main`?** – No, only opens **draft** PRs.
* **Retries?** – The DAG re‑enters the code node up to 3 times if validation
  fails, then stops so you can inspect logs.
* **Manual edits?** – You can jump in, tweak code, and re‑run the same container;
  LangGraph restarts from the last failed node.

Happy hacking – and let the robots handle the boilerplate!