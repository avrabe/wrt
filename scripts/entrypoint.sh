#!/usr/bin/env bash
set -euo pipefail
if [[ "${MCP_AUTOSTART:-1}" == "1" ]]; then
  npx -y @modelcontextprotocol/server-filesystem --root /workspace --port 7725 &
  export MCP_SERVER="tcp://127.0.0.1:7725"
fi
python /.ai/nodes/mcp_tools.py &
exec "$@" 