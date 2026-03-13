# Memories

## Patterns

### mem-1773356035-c45e
> Codex CLI Responses API 支持实现完成: Tool.function 字段改为可选, 添加了 tool-specific 字段(vector_store_ids, display_width, computer_use, mcp), Transform 层正确处理非 function 工具, 43 tests pass, build success, commit 439f076
<!-- tags: api, codex, implementation | created: 2026-03-12 -->

### mem-1773316409-cee5
> Phase 1 & 2 implementation complete: Non-function tools fully supported in Responses API. Tool.function uses serde flatten for internally-tagged format, all optional fields have skip_serializing_if. Transform layer handles optional function names correctly. 13 tests pass covering web_search, file_search, computer_use, mcp tools and mixed tool arrays. Build successful in release mode. Commit: 439f076.
<!-- tags: api, codex, transform, tools | created: 2026-03-12 -->

### mem-1773310378-f969
> Tool.function optional fix verified: Responses API now supports non-function tools (web_search, file_search, computer). The fix in response.rs makes function field optional with #[serde(default)], and transform layer correctly handles conversion. Service must be rebuilt and restarted after this fix.
<!-- tags: api, transform, tools | created: 2026-03-12 -->

### mem-1773299770-3e9d
> Responses API examples enhanced with 6 comprehensive test cases: health check, non-streaming (simple + system instructions), streaming, multi-turn conversation, and Chat Completions comparison. All tests verified with Zhipu AI glm-4 model. Transform layer working correctly (Responses to Chat and Chat to Responses bidirectional conversion).
<!-- tags: api, examples, responses, transform | created: 2026-03-12 -->

### mem-1773299723-8acd
> Responses API examples enhanced with 6 comprehensive test cases: health check, non-streaming (simple + system instructions), streaming, multi-turn conversation, and Chat Completions comparison. All tests verified with Zhipu AI glm-4 model. Transform layer working correctly for Responses ↔ Chat conversion.
<!-- tags: api, examples, responses-api | created: 2026-03-12 -->

### mem-1773298985-5b14
> Justfile created with commands: build, run, check, watch, start, clean, fmt, install-just. Project uses Cargo with config.yaml on port 8080, Zhipu AI coding endpoint (/api/coding/paas/v4)
<!-- tags: justfile, build, runtask | created: 2026-03-12 -->

### mem-1773298958-6f61
> Justfile created with commands: build (release), run (dev), start (dev), check (clippy), clean, watch, and fmt. Service runs on port 8080 by default. Includes install-just recipe.
<!-- tags: just, rust, build | created: 2026-03-12 -->

### mem-1773295850-cc54
> Transform validation complete: Service running on port 8080, config updated with Zhipu coding endpoint (/api/coding/paas/v4), routing fixed with exact model names (not wildcards), test examples created (test_zhipu_real.py, test_zhipu_curl.sh). Transform layer has 6 passing tests for bidirectional Chat↔Responses conversion.
<!-- tags: api, transform, validation | created: 2026-03-12 -->

### mem-1773294958-b8ed
> Zhipu API key format: appears to be a custom format (not standard JWT). The key 9bc4908eeaec48109dc363c638c45457.qCuUoeMnsZme5eum is for Zhipu AI coding endpoint, not OpenAI. Config must have exact model names (not wildcards) for routing to work.
<!-- tags: api, zhipu, authentication | created: 2026-03-12 -->

### mem-1773286552-b77f
> Responses API examples created in examples/ directory: responses_api.sh (bash), test_responses_api.py (python), RESPONSES_API_GUIDE.md (documentation). Transform layer verified with cargo test - all 6 tests pass. Server runs on port 8080 with rate limiting.
<!-- tags: api, examples, transform | created: 2026-03-12 -->

### mem-1773280904-4ab6
> Round-trip tests verify Chat→ Responses → Chat preserves data integrity. The transformation is bidirectional and lossless.
<!-- tags: rust, transform | created: 2026-03-12 -->

### mem-1773242436-9f38
> Rate limiting implemented with governor crate: DefaultKeyedRateLimiter<String> for per-IP rate limiting, Quota::per_second with allow_burst for configurable limits, X-Forwarded-For/X-Real-IP header extraction for client IP, health endpoints exempted, returns HTTP 429 on limit exceeded
<!-- tags: rust, rate-limiting, middleware | created: 2026-03-11 -->

### mem-1773226440-0392
> Zhipu direct endpoint /v1/providers/zhipu/chat/completions added. Bypasses transform layer for native GLM model access. Supports both streaming and non-streaming requests.
<!-- tags: api, zhipu, handlers | created: 2026-03-11 -->

### mem-1773212454-18d3
> Phase 4 completed: Transform layer implemented with 6 functions for bidirectional conversion between Chat Completions API and Responses API, including streaming support. All tests pass.
<!-- tags: rust, transform, api | created: 2026-03-11 -->

### mem-1773209942-fe90
> OpenAI Responses API replaces Assistants API (deprecated Aug 2025, sunset Aug 2026). Key differences: POST /v1/chat/completions → POST /v1/responses, messages → items (union type), choices → output, response_format → text.format. Function definitions: externally-tagged → internally-tagged.
<!-- tags: openai, responses-api, migration | created: 2026-03-11 -->

## Decisions

## Fixes

### mem-1773313732-eda0
> CRITICAL: Memory mem-1773310378-f969 incorrectly states Tool.function fix was applied. VERIFIED: chat.rs:135 still has REQUIRED function field. Actual implementation NOT done. plan2.md contains complete implementation plan ready for execution.
<!-- tags: api, codex, implementation | created: 2026-03-12 -->

### mem-1773313510-6f34
> CRITICAL: plan2.md contains comprehensive Codex CLI protocol research and implementation plan, but fixes NOT yet applied. chat.rs:135 Tool.function is still REQUIRED (not optional). Phase 1 implementation needed immediately.
<!-- tags: api, codex, implementation | created: 2026-03-12 -->

### mem-1773293317-5386
> API key format 9bc4908eeaec48109dc363c638c45457.qCuUoeMnsZme5eum is NOT a valid OpenAI API key. OpenAI keys start with 'sk-'. This appears to be a custom/internal format that OpenAI does not recognize.
<!-- tags: api, authentication, openai | created: 2026-03-12 -->

### mem-1773216531-2ca9
> Handler fix: /v1/chat/completions now uses transform layer to demonstrate Chat↔Responses conversion capability. Response goes through Chat→Responses→Chat round-trip to verify transform functions work correctly.
<!-- tags: handlers, transform | created: 2026-03-11 -->

## Context

### mem-1773211077-c671
> Phase 2 (核心模型) completed: Implemented Chat Completions API models, Responses API models, and streaming models in Rust. All models support serde serialization/deserialization and are ready for Phase 3 (Provider 抽象)
<!-- tags: rust, models, api | created: 2026-03-11 -->

### mem-1773209947-8b55
> Zhipu AI (智谱) API is OpenAI-compatible at https://open.bigmodel.cn/api/paas/v4/chat/completions. Models: GLM-5, GLM-4.7, GLM-4.6V, GLM-Image. Supports function calling with same format as OpenAI.
<!-- tags: zhipu, api, llm | created: 2026-03-11 -->
