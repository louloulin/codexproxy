# Research & Analysis: Codex CLI + Responses API Tools

## Date: 2026-03-12

## Current Issue Analysis

### Error from Codex CLI
```
codex_core::codex: Turn error: unexpected status 422 Unprocessable Entity:
Failed to deserialize the JSON body into the target type:
tools[0]: missing field `function` at line 1 column 32330
url: http://localhost:8080/v1/responses
```

### Root Cause
The current implementation requires `Tool.function` to be present, but OpenAI's Responses API supports non-function tools:
- `web_search` - No function definition needed
- `file_search` - No function definition needed
- `computer` / `computer_use` - No function definition needed
- `mcp` - Remote MCP servers, no function definition needed

### Code Location of Issue

**src/models/chat.rs:**
```rust
pub struct Tool {
    pub tool_type: String,
    pub function: FunctionDefinition,  // REQUIRED - This is the problem
}
```

**src/models/response.rs:**
```rust
pub struct Tool {
    pub tool_type: String,
    pub function: Option<FunctionDefinition>,  // Already optional - good!
}
```

**src/transform/mod.rs:**
The transform layer assumes all tools have function definitions:
```rust
let tools = chat_req.tools.as_ref().map(|tools| {
    tools.iter().map(|t| crate::models::response::Tool {
        tool_type: t.tool_type.clone(),
        function: Some(FunctionDefinition { ... }),  // Always wrapping in Some()
    }).collect()
}).unwrap_or_default();
```

## Official OpenAI Documentation Analysis

### Tool Types in Responses API

Based on official documentation:

1. **web_search**
```json
{ "type": "web_search" }
```
No additional fields required.

2. **file_search**
```json
{
  "type": "file_search",
  "vector_store_ids": ["<vector_store_id>"]
}
```
Requires `vector_store_ids` array.

3. **computer_use**
```json
{
  "type": "computer_use",
  "display_width": 1024,
  "display_height": 768,
  "environment": "mac" // or "windows", "linux", "ubuntu"
}
```

4. **mcp** (Remote MCP servers)
```json
{
  "type": "mcp",
  "server_label": "dmcp",
  "server_description": "Description",
  "server_url": "https://example.com/sse",
  "require_approval": "never"
}
```

5. **function** (Traditional function calling)
```json
{
  "type": "function",
  "name": "get_weather",
  "description": "Get weather",
  "parameters": { ... },
  "strict": true
}
```
Note: In Responses API, function is internally-tagged (flat structure), not wrapped in `function` object.

## Key Differences: Chat Completions vs Responses API

| Aspect | Chat Completions | Responses API |
|--------|------------------|---------------|
| Tool types | Only `function` supported | `function`, `web_search`, `file_search`, `computer_use`, `mcp`, etc. |
| Function format | Externally-tagged | Internally-tagged |
| Non-function tools | Not supported | Fully supported |
| Strict mode | Default: false | Default: true |

## Codex CLI Request Protocol

Codex CLI sends requests to `/v1/responses` endpoint with:
1. Non-function tools (`web_search`, `file_search`, etc.)
2. Function tools in the new internally-tagged format
3. Multi-turn conversations using `previous_response_id`
4. Stateful context with `store: true`

## Impact Assessment

### Files Requiring Modification

1. **src/models/chat.rs** - Make `Tool.function` optional
2. **src/models/response.rs** - Add support for tool-specific fields (already partially done)
3. **src/transform/mod.rs** - Update conversion logic for non-function tools
4. **src/handlers/mod.rs** - Ensure handlers pass through non-function tools correctly

### Backward Compatibility Concerns

- Must maintain backward compatibility with existing Chat Completions API clients
- Chat Completions only supports `function` type tools
- When converting Responses → Chat, non-function tools should be filtered or handled specially

## Next Steps

1. Fix Tool.function to be optional in chat.rs
2. Add tool-specific fields to response.rs Tool struct
3. Update transform layer to handle non-function tools
4. Add tests for each tool type
5. Document the changes and migration path

## 2026-03-12 Research: Codex CLI Protocol Analysis

### Codex CLI Architecture (from OpenAI Blog)

**Core Mechanism:**
- Codex CLI sends HTTP requests to **Responses API** (`/v1/responses`)
- Uses Server-Sent Events (SSE) for streaming
- Agent loop: Prompt → Inference → Tool Calls → Response → Repeat

**Endpoints:**
- ChatGPT login: `https://chatgpt.com/backend-api/codex/responses`
- API key auth: `https://api.openai.com/v1/responses`
- OSS mode (ollama/LM Studio): `http://localhost:11434/v1/responses`
- Custom providers: Any Responses API compatible endpoint

**Request Protocol Details:**

1. **Input Structure:**
   - `instructions`: System/developer messages
   - `tools`: Array of tool definitions
   - `input`: List of items (messages, reasoning, function calls, outputs)

2. **Tool Types Supported:**
   - `function`: Traditional function calling (internally-tagged format)
   - `web_search`: Built-in web search tool
   - `file_search`: Vector store search
   - `computer` / `computer_use`: UI automation
   - `mcp`: Remote MCP servers

3. **Response Format:**
   - SSE stream with events like:
     - `response.output_text.delta`: Streaming text
     - `response.output_item.added`: New output item
     - `response.output_item.done`: Complete output item
   - Output types: `message`, `reasoning`, `function_call`, `function_call_output`

4. **Context Management:**
   - `previous_response_id`: Optional statefulness (Codex doesn't use for ZDR support)
   - Prompt caching: Exact prefix matching required
   - Compaction: `/responses/compact` endpoint for context window management

### Key Protocol Differences: Chat Completions vs Responses API

| Aspect | Chat Completions | Responses API |
|--------|------------------|---------------|
| **Endpoint** | `/v1/chat/completions` | `/v1/responses` |
| **Input** | `messages[]` array | `input[]` items (union type) |
| **Output** | `choices[]` with `message` | `output[]` items (union type) |
| **Tool Format** | Externally-tagged | Internally-tagged |
| **Function Strict** | Default: false | Default: true |
| **Non-function Tools** | Not supported | Fully supported |
| **Reasoning** | Not available | `reasoning` items with encrypted content |
| **Statefulness** | Manual context management | Optional `previous_response_id` + `store` |

### Function Definition Format Differences

**Chat Completions (Externally-tagged):**
```json
{
  "type": "function",
  "function": {
    "name": "get_weather",
    "description": "Get weather",
    "parameters": {...}
  }
}
```

**Responses API (Internally-tagged):**
```json
{
  "type": "function",
  "name": "get_weather",
  "description": "Get weather",
  "parameters": {...}
}
```

### Current Implementation Issues

**Issue 1: chat.rs Tool.function is REQUIRED**
- Location: `src/models/chat.rs:135`
- Problem: `pub function: FunctionDefinition` (not optional)
- Impact: Cannot deserialize Responses API requests with non-function tools
- Fix: Make it `Option<FunctionDefinition>`

**Issue 2: Transform layer assumes all tools have functions**
- Location: `src/transform/mod.rs:52-59`
- Problem: Always wraps in `Some(FunctionDefinition {...})`
- Impact: Incorrectly creates function definitions for non-function tools
- Fix: Check tool type before creating function definition

**Issue 3: Missing tool-specific fields in response.rs**
- Current: Only has `tool_type` and `function`
- Missing: `vector_store_ids` (file_search), `display_width/height/environment` (computer_use), `server_label/server_url/require_approval` (mcp)

### Codex CLI Specific Behaviors

1. **Zero Data Retention (ZDR) Support:**
   - Does NOT use `previous_response_id`
   - Keeps requests fully stateless
   - Uses `encrypted_content` for reasoning items

2. **Prompt Caching Strategy:**
   - Static content at beginning (instructions, tools)
   - Variable content at end (user messages)
   - Cache misses from: tool changes, model changes, config changes

3. **Tool Enumeration:**
   - Codex-provided tools (e.g., `shell`)
   - Responses API built-in tools (e.g., `web_search`)
   - User tools via MCP servers

### Transformation Requirements

**Responses → Chat Conversion:**
1. Filter out non-function tools (Chat only supports `function`)
2. Convert internally-tagged to externally-tagged format
3. Map `input[]` items to `messages[]` array
4. Map `output[]` items to `choices[]` array
5. Handle reasoning items (preserve as metadata or discard)

**Chat → Responses Conversion:**
1. Convert externally-tagged to internally-tagged format
2. Map `messages[]` to `input[]` items
3. Map `choices[]` to `output[]` items
4. Add support for non-function tools if needed

## 2026-03-12 Additional Research: Official Documentation Analysis

### Compaction API (Context Window Management)

**Endpoint:** `POST /v1/responses/compact`

**Purpose:** Reduce context size while preserving state for long-running conversations.

**Modes:**
1. **Server-side compaction** - Set `context_management` with `compact_threshold` in Responses create request
2. **Standalone compaction** - Explicit call to `/responses/compact` endpoint

**ZDR Compatibility:**
- Server-side compaction is ZDR-friendly when `store=false`
- Compaction item is encrypted and carries context forward

### Web Search Tool Details

**Actions:**
- `search` - Standard web search (includes queries)
- `open_page` - Open a page (reasoning models only)
- `find_in_page` - Search within a page (reasoning models only)

**Features:**
- Domain filtering with `filters.allowed_domains` (up to 100 URLs)
- User location with `user_location` (country, city, region, timezone)
- External web access control with `external_web_access` (default: true)
- Sources retrieval with `include: ["web_search_call.action.sources"]`

### File Search Tool Details

**Features:**
- `vector_store_ids` - Required array of vector store IDs
- `max_num_results` - Limit number of results (default: unlimited)
- `filters` - Metadata filtering
- Include search results with `include: ["file_search_call.results"]`

### Computer Use Tool Details

**Parameters:**
- `display_width` - Display width in pixels
- `display_height` - Display height in pixels
- `environment` - One of: "mac", "windows", "linux", "ubuntu"

### MCP Tool Details (Remote MCP Servers)

**Parameters:**
- `server_label` - Label for the MCP server
- `server_description` - Optional description
- `server_url` - SSE endpoint URL
- `require_approval` - "never" or "always"

### Statefulness & Context Management

**Options:**
1. **Stateless mode** (ZDR-compatible):
   - Set `store: false`
   - Use `include: ["reasoning.encrypted_content"]`
   - Pass encrypted reasoning items in future requests

2. **Stateful mode:**
   - Set `store: true` (default for Responses API)
   - Use `previous_response_id` to chain responses
   - Server maintains context automatically

### Prompt Caching

**Eligibility:**
- Standard prompt caching IS ZDR-compliant (in-memory only)
- Extended prompt caching is NOT ZDR-eligible (storage requirements)

**Benefits:**
- Up to 80% latency reduction
- Up to 90% cost reduction
- Works automatically on all API requests

### Current Implementation Gap Analysis

**Missing Tool Fields in response.rs:**

Currently only `tool_type` and `function` are defined. Missing:

1. **File search fields:**
   - `vector_store_ids: Option<Vec<String>>`
   - `max_num_results: Option<u32>` (optional)
   - `filters: Option<serde_json::Value>` (optional)

2. **Computer use fields:**
   - `display_width: Option<u32>`
   - `display_height: Option<u32>`
   - `environment: Option<String>`

3. **Mcp fields:**
   - `server_label: Option<String>`
   - `server_description: Option<String>`
   - `server_url: Option<String>`
   - `require_approval: Option<String>`

4. **Web search fields:**
   - `filters: Option<WebSearchFilters>` (for domain filtering)
   - `user_location: Option<UserLocation>` (for geo-targeting)
   - `external_web_access: Option<bool>` (for cache-only mode)

### Additional Model Structures Needed

**WebSearchFilters:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchFilters {
    pub allowed_domains: Option<Vec<String>>,
}
```

**UserLocation:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLocation {
    #[serde(rename = "type")]
    pub location_type: String,  // "approximate"
    pub country: Option<String>,  // ISO country code
    pub city: Option<String>,
    pub region: Option<String>,
    pub timezone: Option<String>,  // IANA timezone
}
```

### Transform Layer Issues

**Current Code (transform/mod.rs:46-63):**
- Always creates `Some(FunctionDefinition {...})` even for non-function tools
- Will panic if `t.function` is None
- Does not preserve tool-specific fields (vector_store_ids, etc.)

**Fix Required:**
```rust
let tools = chat_req
    .tools
    .as_ref()
    .map(|tools| {
        tools
            .iter()
            .filter_map(|t| {
                // Only convert tools that have a function definition
                t.function.as_ref().map(|f| {
                    crate::models::response::Tool {
                        tool_type: t.tool_type.clone(),
                        function: Some(crate::models::response::FunctionDefinition {
                            name: f.name.clone(),
                            description: f.description.clone(),
                            parameters: f.parameters.clone(),
                            strict: Some(true),  // Responses API default
                        }),
                        // New fields default to None
                        vector_store_ids: None,
                        display_width: None,
                        display_height: None,
                        environment: None,
                        server_label: None,
                        server_description: None,
                        server_url: None,
                        require_approval: None,
                    }
                })
            })
            .collect()
    })
    .unwrap_or_default();
```

### Streaming Event Types (Responses API)

**Events:**
- `response.output_text.delta` - Streaming text
- `response.output_item.added` - New output item added
- `response.output_item.done` - Output item completed
- `response.reasoning.delta` - Reasoning content
- `response.function_call.delta` - Function call arguments
- `done` - Stream complete

### Output Item Types

**Current:** `message`, `reasoning`, `function_call`

**Missing:**
- `web_search_call` - Web search tool call
- `file_search_call` - File search tool call
- `computer_call` - Computer use tool call
- `image_generation_call` - Image generation call
- `code_interpreter_call` - Code execution call
- `mcp_tool_call` - MCP server tool call


## 2026-03-12 Research Complete - Ready for Implementation

### Research Phase Summary

**Comprehensive Codex CLI protocol analysis complete:**

1. **Codex CLI Architecture Understood:**
   - Uses Responses API exclusively (/v1/responses)
   - Supports multiple auth modes (ChatGPT login, API key, OSS mode, custom providers)
   - Agent loop: Prompt → Inference → Tool Calls → Response
   - ZDR-compatible: No previous_response_id, uses encrypted reasoning

2. **Protocol Differences Documented:**
   - Chat Completions vs Responses API comparison table
   - Tool format: Externally-tagged vs Internally-tagged
   - Output types: choices[] vs output[] (union types)
   - Statefulness: Manual vs previous_response_id

3. **All Tool Types Analyzed:**
   - function: Internally-tagged format
   - web_search: No additional fields
   - file_search: vector_store_ids required
   - computer_use: display dimensions + environment
   - mcp: Remote MCP servers with server_url

4. **Current Issues Identified:**
   - **CRITICAL BLOCKER**: chat.rs:135 Tool.function is REQUIRED (not optional)
   - Transform layer assumes all tools have functions
   - Missing tool-specific fields in response.rs
   - No output item types for tool calls

5. **Implementation Plan Created:**
   - Phase 1 (P0): Fix core models - Make Tool.function optional
   - Phase 2 (P0): Update transform layer for non-function tools
   - Phase 3 (P1): Handler updates with logging
   - Phase 4 (P1): Comprehensive testing
   - Phase 5 (P2): Documentation and examples

6. **Additional Research Completed:**
   - Compaction API for context management
   - Web search tool specifications (actions, filters, citations)
   - File search tool details (formats, setup, output)
   - MCP tool specifications
   - Prompt caching & ZDR compatibility
   - Streaming events and output item types

### Key Deliverables in plan2.md

- Executive summary with problem/solution approach
- Complete tool type reference with JSON examples
- Current implementation analysis with code locations
- Detailed implementation plan with code changes
- Migration path with timeline estimates
- Risk assessment and mitigation strategies
- Success criteria and metrics
- References to official documentation

### Status

✅ Research: COMPLETE
✅ Plan: COMPLETE (plan2.md)
❌ Implementation: NOT STARTED

### Next Step

**Ready for Phase 1 implementation:**
1. Fix chat.rs Tool.function to Option<FunctionDefinition>
2. Add tool-specific fields to response.rs
3. Update transform layer
4. Add tests for non-function tools

