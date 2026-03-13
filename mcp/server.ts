/**
 * Protein-Engine MCP Server
 *
 * Exposes the Protein-Engine platform as MCP tools for Claude Code.
 * Delegates to the pe-cli binary or pe-api HTTP endpoints.
 */

import { execFile } from "node:child_process";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const __dirname = dirname(fileURLToPath(import.meta.url));
const TOOL_DEFS_PATH = join(__dirname, "..", "protein-engine-mcp.json");

// ── Configuration ────────────────────────────────────────────────────

const PE_CLI = process.env.PE_CLI_PATH ?? "protein-engine";
const PE_API_URL = process.env.PE_API_URL ?? "http://localhost:8080";
const USE_HTTP = process.env.PE_USE_HTTP === "true";

// ── CLI execution ────────────────────────────────────────────────────

interface CliResult {
  stdout: string;
  stderr: string;
}

async function runCli(args: string[]): Promise<CliResult> {
  try {
    const { stdout, stderr } = await execFileAsync(PE_CLI, ["--json", ...args], {
      timeout: 120_000,
      maxBuffer: 10 * 1024 * 1024,
    });
    return { stdout, stderr };
  } catch (err: unknown) {
    const e = err as { stdout?: string; stderr?: string; message?: string };
    throw new Error(
      `pe-cli failed: ${e.stderr ?? e.message ?? "unknown error"}`,
    );
  }
}

function parseJsonOutput(stdout: string): unknown {
  const trimmed = stdout.trim();
  if (!trimmed) return {};
  return JSON.parse(trimmed);
}

// ── HTTP execution (alternative to CLI) ──────────────────────────────

async function httpPost(path: string, body: unknown): Promise<unknown> {
  const res = await fetch(`${PE_API_URL}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`HTTP ${res.status}: ${text}`);
  }
  return res.json();
}

async function httpGet(path: string): Promise<unknown> {
  const res = await fetch(`${PE_API_URL}${path}`);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`HTTP ${res.status}: ${text}`);
  }
  return res.json();
}

// ── Tool handlers ────────────────────────────────────────────────────

type ToolInput = Record<string, unknown>;

async function handleScoreSequence(input: ToolInput): Promise<unknown> {
  const sequence = input.sequence as string;
  if (USE_HTTP) {
    return httpPost("/score", { sequence });
  }
  const { stdout } = await runCli(["score", sequence]);
  return parseJsonOutput(stdout);
}

async function handleEvolve(input: ToolInput): Promise<unknown> {
  const args = ["evolve"];
  if (input.generations != null) args.push("--generations", String(input.generations));
  if (input.population_size != null) args.push("--population-size", String(input.population_size));
  if (input.seed_sequence != null) args.push("--seed-sequence", String(input.seed_sequence));
  if (input.mutation_rate != null) args.push("--mutation-rate", String(input.mutation_rate));
  if (input.top_k != null) args.push("--top-k", String(input.top_k));

  const { stdout } = await runCli(args);
  return parseJsonOutput(stdout);
}

async function handleSearchSimilar(input: ToolInput): Promise<unknown> {
  const sequence = input.sequence as string;
  const k = input.k != null ? String(input.k) : "5";
  if (USE_HTTP) {
    return httpPost("/search", { sequence, k: Number(k) });
  }
  const { stdout } = await runCli(["search", sequence, "--k", k]);
  return parseJsonOutput(stdout);
}

async function handleQuantumVqe(input: ToolInput): Promise<unknown> {
  const molecule = input.molecule as string;
  const { stdout } = await runCli(["quantum", "vqe", molecule]);
  return parseJsonOutput(stdout);
}

async function handleLedgerVerify(): Promise<unknown> {
  const { stdout } = await runCli(["ledger", "verify"]);
  return parseJsonOutput(stdout);
}

async function handleRvfInspect(input: ToolInput): Promise<unknown> {
  const path = input.path as string;
  const { stdout } = await runCli(["rvf", "inspect", path]);
  return parseJsonOutput(stdout);
}

async function handleCreateVariant(input: ToolInput): Promise<unknown> {
  // Create variant by scoring and returning combined result
  const sequence = input.sequence as string;
  const name = input.name as string;
  const factor = input.factor as string;

  // Score the sequence first
  const { stdout } = await runCli(["score", sequence]);
  const score = parseJsonOutput(stdout);

  return {
    name,
    sequence,
    factor,
    score,
  };
}

const TOOL_HANDLERS: Record<string, (input: ToolInput) => Promise<unknown>> = {
  score_sequence: handleScoreSequence,
  evolve: handleEvolve,
  search_similar: handleSearchSimilar,
  quantum_vqe: handleQuantumVqe,
  ledger_verify: handleLedgerVerify,
  rvf_inspect: handleRvfInspect,
  create_variant: handleCreateVariant,
};

// ── MCP protocol over stdio ──────────────────────────────────────────

interface McpRequest {
  jsonrpc: "2.0";
  id: number | string;
  method: string;
  params?: Record<string, unknown>;
}

interface McpToolDef {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
}

let toolDefs: McpToolDef[] = [];

async function loadToolDefs(): Promise<void> {
  const raw = await readFile(TOOL_DEFS_PATH, "utf-8");
  const config = JSON.parse(raw) as { tools: McpToolDef[] };
  toolDefs = config.tools;
}

function sendResponse(id: number | string, result: unknown): void {
  const msg = JSON.stringify({ jsonrpc: "2.0", id, result });
  process.stdout.write(msg + "\n");
}

function sendError(id: number | string, code: number, message: string): void {
  const msg = JSON.stringify({ jsonrpc: "2.0", id, error: { code, message } });
  process.stdout.write(msg + "\n");
}

async function handleRequest(req: McpRequest): Promise<void> {
  try {
    switch (req.method) {
      case "initialize":
        sendResponse(req.id, {
          protocolVersion: "2024-11-05",
          capabilities: { tools: {} },
          serverInfo: {
            name: "protein-engine",
            version: "0.1.0",
          },
        });
        break;

      case "tools/list":
        sendResponse(req.id, {
          tools: toolDefs,
        });
        break;

      case "tools/call": {
        const params = req.params as { name: string; arguments?: ToolInput };
        const handler = TOOL_HANDLERS[params.name];
        if (!handler) {
          sendError(req.id, -32601, `Unknown tool: ${params.name}`);
          return;
        }
        const result = await handler(params.arguments ?? {});
        sendResponse(req.id, {
          content: [{ type: "text", text: JSON.stringify(result, null, 2) }],
        });
        break;
      }

      case "notifications/initialized":
        // Acknowledgment, no response needed
        break;

      default:
        sendError(req.id, -32601, `Method not found: ${req.method}`);
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    sendError(req.id, -32603, message);
  }
}

// ── Stdio transport ──────────────────────────────────────────────────

async function main(): Promise<void> {
  await loadToolDefs();

  let buffer = "";

  process.stdin.setEncoding("utf-8");
  process.stdin.on("data", (chunk: string) => {
    buffer += chunk;
    const lines = buffer.split("\n");
    buffer = lines.pop() ?? "";

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      try {
        const req = JSON.parse(trimmed) as McpRequest;
        handleRequest(req);
      } catch {
        process.stderr.write(`Failed to parse: ${trimmed}\n`);
      }
    }
  });

  process.stderr.write("Protein-Engine MCP server ready\n");
}

main();
