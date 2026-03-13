# Protein-Engine MCP Server

MCP (Model Context Protocol) server that exposes the Protein-Engine platform to Claude Code and other MCP-compatible AI agents.

## Setup

### Option 1: Claude Code configuration

Add to your Claude Code MCP settings (`~/.claude/claude_desktop_config.json` or project `.mcp.json`):

```json
{
  "mcpServers": {
    "protein-engine": {
      "command": "node",
      "args": ["mcp/dist/server.js"],
      "cwd": "/path/to/Protein-Engine"
    }
  }
}
```

### Option 2: With RVF file (standalone)

```json
{
  "mcpServers": {
    "protein-engine": {
      "command": "npx",
      "args": ["@ruvector/rvf-mcp-server", "protein-engine.rvf"]
    }
  }
}
```

### Option 3: HTTP mode (connect to running server)

```json
{
  "mcpServers": {
    "protein-engine": {
      "command": "node",
      "args": ["mcp/dist/server.js"],
      "cwd": "/path/to/Protein-Engine",
      "env": {
        "PE_USE_HTTP": "true",
        "PE_API_URL": "http://localhost:8080"
      }
    }
  }
}
```

## Available Tools

| Tool | Description |
|------|-------------|
| `score_sequence` | Score an amino acid sequence for reprogramming fitness |
| `evolve` | Run directed evolution cycles on a seed sequence |
| `search_similar` | Find similar protein sequences in the vector store |
| `quantum_vqe` | Run a VQE quantum chemistry calculation |
| `ledger_verify` | Verify cryptographic journal chain integrity |
| `rvf_inspect` | Inspect an RVF file's manifest and segments |
| `create_variant` | Create, score, and store a new protein variant |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PE_CLI_PATH` | `protein-engine` | Path to the pe-cli binary |
| `PE_API_URL` | `http://localhost:8080` | HTTP API base URL |
| `PE_USE_HTTP` | `false` | Use HTTP API instead of CLI subprocess |

## Building

```bash
cd mcp
npm install
npm run build
```

## Architecture

The MCP server operates in two modes:

- **CLI mode** (default): Spawns `protein-engine --json <command>` subprocesses. No running server required.
- **HTTP mode**: Forwards requests to a running pe-api server. Set `PE_USE_HTTP=true`.

Tool definitions are loaded from `protein-engine-mcp.json` at startup.
