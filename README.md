# OpenAI Proxy

A Rust-based API proxy server that transforms between OpenAI Chat Completions API and Responses API formats, with support for Zhipu AI (GLM models).

[English](#english) | [中文文档](#中文文档)

---

<a name="english"></a>
## Features

- **API Format Conversion**: Bidirectional conversion between Chat Completions API and Responses API
- **Multi-Provider Support**: OpenAI and Zhipu AI providers
- **Streaming Support**: Full SSE streaming support for both APIs
- **Model Routing**: Automatic provider selection based on model names
- **CORS Enabled**: Cross-origin requests supported
- **Structured Logging**: JSON-formatted logs with tracing

## Architecture

```
Client Request
     │
     ▼
┌─────────────────────────────┐
│   Axum Web Server          │
│  ┌───────────────────────┐ │
│  │ Transform Layer       │ │
│  │ Chat ↔ Responses      │ │
│  └───────────────────────┘ │
│  ┌───────────────────────┐ │
│  │ Routing Layer         │ │
│  └───────────────────────┘ │
│  ┌───────────────────────┐ │
│  │ LLM Provider Adapter  │ │
│  └───────────────────────┘ │
└─────────────────────────────┘
     │
     ▼
┌───────────┐ ┌───────────┐
│  OpenAI   │ │  Zhipu AI │
│   API     │ │    API    │
└───────────┘ └───────────┘
```

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /` | Health check |
| `GET /health` | Health check |
| `POST /v1/chat/completions` | OpenAI-compatible Chat Completions API |
| `POST /v1/responses` | OpenAI Responses API |
| `POST /v1/providers/zhipu/chat/completions` | Zhipu direct endpoint |

## Installation

### Prerequisites

- Rust 1.70+
- Cargo
- API keys for your providers (OpenAI and/or Zhipu AI)

### Quick Start (快速开始)

Get up and running in 3 steps:

```bash
# 1. Clone and build
git clone <repository-url>
cd rcodex
cargo build --release

# 2. Set your API key (at least one required)
export ZHIPU_API_KEY="your-zhipu-key"
# or
export OPENAI_API_KEY="your-openai-key"

# 3. Run the service
./target/release/openai-proxy
```

Test it works:
```bash
curl http://localhost:8080/health
# Response: {"status":"ok"}
```

### Build

```bash
cargo build --release
```

### Run

```bash
# Set environment variables
export OPENAI_API_KEY="your-openai-key"
export ZHIPU_API_KEY="your-zhipu-key"

# Run with default config (config.yaml)
cargo run --release

# Or specify a custom config
cargo run --release -- --config custom-config.yaml
```

## Configuration

### Configuration File (配置详解)

Create a `config.yaml` file in the project root. The configuration is divided into four main sections:

#### 1. Server Configuration (服务器配置)

```yaml
server:
  host: "0.0.0.0"    # Server bind address (服务器绑定地址)
                      # Use "127.0.0.1" for localhost only
                      # Use "0.0.0.0" to accept connections from all interfaces
  port: 8080          # Server port (服务器端口)
```

#### 2. Provider Configuration (提供商配置)

Each provider (e.g., `openai`, `zhipu`) supports the following options:

```yaml
providers:
  # OpenAI Provider Configuration
  openai:
    api_key: "${OPENAI_API_KEY}"           # API key (支持环境变量 ${VAR} 语法)
                                            # Required for provider to work
    base_url: "https://api.openai.com/v1"  # API endpoint (API 端点地址)
                                            # Can be customized for proxies or alternative endpoints
    default_model: "gpt-4o"                 # Default model when not specified in request (默认模型)
    timeout: 120                            # Request timeout in seconds (请求超时秒数)
                                            # Longer timeouts for complex requests

  # Zhipu AI Provider Configuration (智谱 AI 配置)
  zhipu:
    api_key: "${ZHIPU_API_KEY}"
    base_url: "https://open.bigmodel.cn/api/paas/v4"
    default_model: "glm-4"
    timeout: 60
```

**Provider Options Explained (提供商选项详解):**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `api_key` | String | Yes | API key for authentication. Supports `${ENV_VAR}` syntax for environment variables |
| `base_url` | String | Yes | Base URL for the provider's API endpoint |
| `default_model` | String | Yes | Default model to use when model not specified in request |
| `timeout` | Integer | No | Request timeout in seconds. Default: `60` |

**API Key Formats (API 密钥格式):**
- **OpenAI**: Starts with `sk-` (e.g., `sk-proj-...`)
- **Zhipu AI**: Custom format (e.g., `xxxxxxxx.xxxxxxxxxxxx`)

#### 3. Routing Configuration (路由配置)

The routing section controls how requests are directed to providers:

```yaml
routing:
  default: "openai"      # Default provider when model not in mapping (默认提供商)
  model_mapping:         # Model to provider mapping (模型路由映射)
    # OpenAI models
    gpt-4o: "openai"
    gpt-4: "openai"
    gpt-3.5-turbo: "openai"
    # Zhipu AI models
    glm-4: "zhipu"
    glm-4-flash: "zhipu"
    glm-4-plus: "zhipu"
```

**How Routing Works (路由工作原理):**
1. Request arrives with a `model` parameter
2. System looks up model in `model_mapping`
3. If found, routes to the specified provider
4. If not found, uses the `default` provider
5. Provider's API key and endpoint are used to forward the request

#### 4. Logging Configuration (日志配置)

```yaml
logging:
  level: "info"    # Log level: trace, debug, info, warn, error
                   # 日志级别: trace < debug < info < warn < error
  format: "json"   # Log format: json, pretty
                   # json: Structured JSON logs (适合生产环境)
                   # pretty: Human-readable format (适合开发调试)
```

**Logging Levels Explained (日志级别说明):**

| Level | Use Case (使用场景) |
|-------|---------------------|
| `trace` | Very detailed debugging (详细调试信息) |
| `debug` | Development debugging (开发调试) |
| `info` | Production default (生产环境默认) |
| `warn` | Warnings only (仅警告信息) |
| `error` | Errors only (仅错误信息) |

### Configuration Options Summary (配置选项汇总)

| Section | Field | Type | Description | Default |
|---------|-------|------|-------------|---------|
| `server.host` | String | Server bind address | `"0.0.0.0"` |
| `server.port` | Integer | Server port | `8080` |
| `providers.<name>.api_key` | String | API key (supports `${ENV_VAR}` syntax) | Required |
| `providers.<name>.base_url` | String | Provider API endpoint | Required |
| `providers.<name>.default_model` | String | Default model for this provider | Required |
| `providers.<name>.timeout` | Integer | Request timeout in seconds | `60` |
| `routing.default` | String | Default provider name | Required |
| `routing.model_mapping` | Map | Model pattern to provider mapping | `{}` |
| `logging.level` | String | Log level | `"info"` |
| `logging.format` | String | Output format | `"json"` |

### Environment Variables (环境变量)

| Variable | Description | Required |
|----------|-------------|----------|
| `OPENAI_API_KEY` | OpenAI API key (starts with `sk-`) | If using OpenAI |
| `ZHIPU_API_KEY` | Zhipu AI API key | If using Zhipu |
| `RUST_LOG` | Override log level (e.g., `debug`) | Optional |

### Example Configurations

**Minimal config (Zhipu only):**
```yaml
server:
  port: 8080

providers:
  zhipu:
    api_key: "${ZHIPU_API_KEY}"
    base_url: "https://open.bigmodel.cn/api/paas/v4"
    default_model: "glm-4"
    timeout: 60

routing:
  default: "zhipu"
  model_mapping:
    glm-4: "zhipu"

logging:
  level: "info"
```

**Multi-provider with custom routing:**
```yaml
server:
  host: "0.0.0.0"
  port: 8080

providers:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com/v1"
    default_model: "gpt-4o"
    timeout: 120

  zhipu:
    api_key: "${ZHIPU_API_KEY}"
    base_url: "https://open.bigmodel.cn/api/paas/v4"
    default_model: "glm-4"
    timeout: 60

routing:
  default: "zhipu"
  model_mapping:
    gpt-4o: "openai"
    gpt-4o-mini: "openai"
    glm-4: "zhipu"
    glm-4-flash: "zhipu"

logging:
  level: "info"
  format: "json"
```

### Using Environment Variables in Config

The config file supports environment variable substitution using `${VAR_NAME}` syntax:

```yaml
providers:
  openai:
    api_key: "${OPENAI_API_KEY}"    # Reads from OPENAI_API_KEY env var
    base_url: "${OPENAI_BASE_URL:-https://api.openai.com/v1}"  # With default value
```

**Advanced Environment Variable Syntax (高级环境变量语法):**

| Syntax | Description | Example |
|--------|-------------|---------|
| `${VAR}` | Required variable - fails if not set | `${OPENAI_API_KEY}` |
| `${VAR:-default}` | Use default if VAR not set | `${PORT:-8080}` |
| `${VAR:=default}` | Set VAR to default if not set | `${LOG_LEVEL:=info}` |

### Advanced Configuration Examples (高级配置示例)

**Using with OpenAI Proxy/Custom Endpoint:**
```yaml
providers:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://your-proxy.example.com/v1"  # Your proxy server
    default_model: "gpt-4o"
    timeout: 120
```

**Development vs Production Configs:**

Create `config.dev.yaml` for development:
```yaml
server:
  port: 3000

logging:
  level: "debug"
  format: "pretty"

providers:
  zhipu:
    api_key: "${ZHIPU_API_KEY}"
    base_url: "https://open.bigmodel.cn/api/paas/v4"
    default_model: "glm-4-flash"  # Use cheaper model for dev

routing:
  default: "zhipu"
  model_mapping:
    glm-4-flash: "zhipu"
```

Create `config.prod.yaml` for production:
```yaml
server:
  host: "0.0.0.0"
  port: 8080

logging:
  level: "info"
  format: "json"

providers:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com/v1"
    default_model: "gpt-4o"
    timeout: 120

  zhipu:
    api_key: "${ZHIPU_API_KEY}"
    base_url: "https://open.bigmodel.cn/api/paas/v4"
    default_model: "glm-4"
    timeout: 60

routing:
  default: "openai"
  model_mapping:
    gpt-4o: "openai"
    glm-4: "zhipu"
```

Start with different configs:
```bash
# Development
./target/release/openai-proxy --config config.dev.yaml

# Production
./target/release/openai-proxy --config config.prod.yaml
```

**Advanced Environment Variable Syntax (高级环境变量语法):**

| Syntax | Description | Example |
|--------|-------------|---------|
| `${VAR}` | Required variable - fails if not set | `${OPENAI_API_KEY}` |
| `${VAR:-default}` | Use default if VAR not set | `${PORT:-8080}` |
| `${VAR:=default}` | Set VAR to default if not set | `${LOG_LEVEL:=info}` |

### Advanced Configuration Examples (高级配置示例)

**Using with OpenAI Proxy/Custom Endpoint:**
```yaml
providers:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://your-proxy.example.com/v1"  # Your proxy server
    default_model: "gpt-4o"
    timeout: 120
```

**Development vs Production Configs:**

Create `config.dev.yaml` for development:
```yaml
server:
  port: 3000

logging:
  level: "debug"
  format: "pretty"

providers:
  zhipu:
    api_key: "${ZHIPU_API_KEY}"
    base_url: "https://open.bigmodel.cn/api/paas/v4"
    default_model: "glm-4-flash"  # Use cheaper model for dev

routing:
  default: "zhipu"
  model_mapping:
    glm-4-flash: "zhipu"
```

Create `config.prod.yaml` for production:
```yaml
server:
  host: "0.0.0.0"
  port: 8080

logging:
  level: "info"
  format: "json"

providers:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com/v1"
    default_model: "gpt-4o"
    timeout: 120

  zhipu:
    api_key: "${ZHIPU_API_KEY}"
    base_url: "https://open.bigmodel.cn/api/paas/v4"
    default_model: "glm-4"
    timeout: 60

routing:
  default: "openai"
  model_mapping:
    gpt-4o: "openai"
    glm-4: "zhipu"
```

Start with different configs:
```bash
# Development
./target/release/openai-proxy --config config.dev.yaml

# Production
./target/release/openai-proxy --config config.prod.yaml
```

## Usage Examples

### Chat Completions API

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "Hello!"}
    ],
    "temperature": 0.7
  }'
```

### Responses API

```bash
curl -X POST http://localhost:8080/v1/responses \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4o",
    "input": [
      {"type": "message", "role": "user", "content": "Hello!"}
    ]
  }'
```

### Zhipu AI (Direct Endpoint)

```bash
curl -X POST http://localhost:8080/v1/providers/zhipu/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZHIPU_API_KEY" \
  -d '{
    "model": "glm-4",
    "messages": [
      {"role": "user", "content": "你好，请介绍一下自己"}
    ]
  }'
```

### Streaming Request

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Tell me a story"}],
    "stream": true
  }'
```

### SDK Integration Examples (SDK 接入示例)

#### Python (OpenAI SDK)

```python
from openai import OpenAI

# Point to your proxy server
client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="dummy-key"  # Key is validated by proxy config
)

# Chat Completions
response = client.chat.completions.create(
    model="gpt-4o",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello!"}
    ]
)
print(response.choices[0].message.content)

# Streaming
stream = client.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "Count to 5"}],
    stream=True
)
for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

#### Python with Zhipu AI

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="dummy-key"
)

# Automatically routes to Zhipu based on model name
response = client.chat.completions.create(
    model="glm-4",  # Routes to Zhipu
    messages=[
        {"role": "user", "content": "你好，请介绍一下自己"}
    ]
)
print(response.choices[0].message.content)
```

#### JavaScript/TypeScript

```javascript
import OpenAI from 'openai';

const client = new OpenAI({
  baseURL: 'http://localhost:8080/v1',
  apiKey: 'dummy-key'  // Key validated by proxy config
});

async function main() {
  // Chat Completions
  const chat = await client.chat.completions.create({
    model: 'gpt-4o',
    messages: [
      { role: 'system', content: 'You are a helpful assistant.' },
      { role: 'user', content: 'Hello!' }
    ]
  });
  console.log(chat.choices[0].message.content);

  // Streaming
  const stream = await client.chat.completions.create({
    model: 'gpt-4o',
    messages: [{ role: 'user', content: 'Count to 5' }],
    stream: true
  });

  for await (const chunk of stream) {
    process.stdout.write(chunk.choices[0]?.delta?.content || '');
  }
}

main();
```

#### cURL with Responses API

```bash
# Use Responses API format (transformed automatically)
curl -X POST http://localhost:8080/v1/responses \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4o",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [{"type": "input_text", "text": "Hello!"}]
      }
    ]
  }'
```

## Supported Models

### OpenAI
- gpt-4o
- gpt-4o-mini
- gpt-4-turbo
- gpt-3.5-turbo

### Zhipu AI
- glm-4
- glm-4-flash
- glm-4-plus

## Development

### Run Tests

```bash
cargo test
```

### Code Format

```bash
cargo fmt
```

### Lint

```bash
cargo clippy
```

## Troubleshooting (常见问题)

### Quick Diagnostics (快速诊断)

Before diving into specific issues, run these checks:

```bash
# 1. Check if service is running
curl http://localhost:8080/health
# Expected: {"status":"ok"}

# 2. Check logs for errors
# If running with cargo:
RUST_LOG=debug cargo run 2>&1 | grep -i error

# 3. Verify configuration
cat config.yaml | grep -A5 "providers:"

# 4. Test API key validity
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"glm-4","messages":[{"role":"user","content":"test"}]}'
```

### Connection Issues (连接问题)

**Error: `Connection refused`**
- **Cause**: Server not running or wrong port
- **Solutions**:
  ```bash
  # Check if server is running
  curl http://localhost:8080/health

  # Check if port is in use
  lsof -i :8080
  # or
  netstat -an | grep 8080

  # Kill process using the port
  kill -9 $(lsof -t -i:8080)

  # Start server on different port
  # Edit config.yaml: server.port: 8081
  ```

**Error: `Connection timeout`**
- **Cause**: Network issues or server overloaded
- **Solutions**:
  - Increase `timeout` in provider config
  - Check network connectivity
  - Verify firewall settings

### API Errors (API 错误)

**Error: `401 Unauthorized`**
- **Cause**: Invalid or missing API key
- **Solutions**:
  ```bash
  # Check environment variables
  echo $OPENAI_API_KEY
  echo $ZHIPU_API_KEY

  # Verify key format
  # OpenAI: starts with 'sk-'
  # Zhipu: custom format like 'xxxxxxxx.xxxxxxxxxxxx'

  # Set key in environment
  export OPENAI_API_KEY="sk-your-key-here"
  export ZHIPU_API_KEY="your-zhipu-key"
  ```

**Error: `429 Too Many Requests`**
- **Cause**: Rate limit exceeded
- **Solutions**:
  - Wait and retry with exponential backoff
  - Check your API usage limits
  - Consider upgrading your plan
  - Implement request queuing

**Error: `model not found` or `Model X does not exist`**
- **Cause**: Model not in routing configuration
- **Solutions**:
  ```yaml
  # Add model to routing.model_mapping in config.yaml
  routing:
    model_mapping:
      gpt-4o: "openai"      # Add your models here
      glm-4: "zhipu"
  ```

**Error: `500 Internal Server Error`**
- **Cause**: Server-side error, often provider API issues
- **Solutions**:
  - Check server logs: `RUST_LOG=debug cargo run`
  - Verify provider API status
  - Check request format and parameters

### Configuration Errors (配置错误)

**Error: `Config file not found`**
- **Cause**: Missing or misplaced config.yaml
- **Solutions**:
  ```bash
  # Create default config
  cp config.yaml.example config.yaml

  # Or specify config path
  ./target/release/openai-proxy --config /path/to/config.yaml
  ```

**Error: `Environment variable not found`**
- **Cause**: `${VAR}` referenced but not set
- **Solutions**:
  ```bash
  # Set the environment variable
  export OPENAI_API_KEY="your-key"

  # Or use default value in config
  api_key: "${OPENAI_API_KEY:-default-key}"
  ```

### Debugging (调试技巧)

**Enable verbose logging:**
```bash
# Debug level logging
RUST_LOG=debug cargo run

# Trace level (very verbose)
RUST_LOG=trace cargo run

# Module-specific logging
RUST_LOG=openai_proxy=debug,tower_http=debug cargo run
```

**Check request/response:**
```bash
# Use curl with verbose output
curl -v -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"glm-4","messages":[{"role":"user","content":"test"}]}'
```

**Monitor logs in real-time:**
```bash
# JSON format (easier to parse)
logging:
  level: "debug"
  format: "json"

# Then run and pipe to jq for pretty output
cargo run 2>&1 | jq .
```

### Common Issues Table (常见问题速查表)

| Issue | Cause | Solution |
|-------|-------|----------|
| Server won't start | Port in use | Change port or kill process |
| API requests timeout | Slow provider response | Increase `timeout` in config |
| Wrong provider selected | Model not mapped | Add to `routing.model_mapping` |
| Streaming not working | Missing headers | Add `Accept: text/event-stream` |
| Invalid API key | Wrong format or expired | Verify key with provider |
| Transform errors | Malformed request | Check request body format |
| CORS errors | Browser blocking | CORS is enabled by default |

### Getting Help (获取帮助)

If you're still having issues:

1. **Check existing examples**: See `examples/` directory for working code
2. **Run tests**: `cargo test` to verify setup
3. **Enable debug logging**: `RUST_LOG=debug cargo run`
4. **Check provider status**:
   - OpenAI: https://status.openai.com
   - Zhipu AI: https://open.bigmodel.cn
5. **Search issues**: Check project issue tracker

## Project Structure

```
src/
├── main.rs                 # Entry point
├── lib.rs                  # Library root
├── config/                 # Configuration management
│   └── app_config.rs
├── error.rs               # Error types
├── error_response.rs      # Error responses
├── handlers/              # HTTP handlers
│   └── mod.rs
├── logging.rs            # Logging setup
├── models/               # Data models
│   ├── chat.rs           # Chat Completions models
│   ├── response.rs       # Responses API models
│   └── streaming.rs      # Streaming models
├── providers/            # LLM providers
│   ├── mod.rs
│   ├── openai.rs
│   ├── trait_.rs
│   └── zhipu.rs
├── server/               # Server setup
│   ├── mod.rs
│   └── router.rs
└── transform/            # API transformation
    └── mod.rs
```

---

<a name="中文文档"></a>
# 中文文档

## 概述

OpenAI Proxy 是一个基于 Rust 的 API 代理服务器，支持在 OpenAI Chat Completions API 和 Responses API 之间进行格式转换，同时支持 Zhipu AI (智谱 AI) 的 GLM 模型。

## 主要特性

- **API 格式转换**: 支持 Chat Completions API 与 Responses API 之间的双向转换
- **多提供商支持**: 支持 OpenAI 和 Zhipu AI
- **流式响应**: 完整的 SSE 流式响应支持
- **自动路由**: 根据模型名称自动选择提供商
- **CORS 支持**: 跨域请求支持

## 快速开始

```bash
# 1. 克隆并构建
git clone <repository-url>
cd rcodex
cargo build --release

# 2. 设置 API 密钥 (至少需要一个)
export ZHIPU_API_KEY="your-zhipu-key"
# 或
export OPENAI_API_KEY="your-openai-key"

# 3. 启动服务
./target/release/openai-proxy
```

测试服务是否正常运行:
```bash
curl http://localhost:8080/health
# 返回: {"status":"ok"}
```

## 配置说明

### 配置文件

在项目根目录创建 `config.yaml`:

```yaml
server:
  host: "0.0.0.0"
  port: 8080

providers:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com/v1"
    default_model: "gpt-4o"
    timeout: 120

  zhipu:
    api_key: "${ZHIPU_API_KEY}"
    base_url: "https://open.bigmodel.cn/api/paas/v4"
    default_model: "glm-4"
    timeout: 60

routing:
  default: "zhipu"
  model_mapping:
    gpt-4o: "openai"
    glm-4: "zhipu"
    glm-4-flash: "zhipu"

logging:
  level: "info"
  format: "json"
```

### 配置选项详解

| 配置项 | 说明 | 示例值 |
|--------|------|--------|
| `server.host` | 服务器绑定地址 | `0.0.0.0` |
| `server.port` | 服务器端口 | `8080` |
| `providers.*.api_key` | API 密钥 (支持环境变量) | `${OPENAI_API_KEY}` |
| `providers.*.base_url` | API 端点 | `https://api.openai.com/v1` |
| `providers.*.default_model` | 默认模型 | `gpt-4o` |
| `providers.*.timeout` | 请求超时(秒) | `60` |
| `routing.default` | 默认提供商 | `openai` |
| `routing.model_mapping` | 模型到提供商的映射 | 见上文 |

### 环境变量

| 变量名 | 说明 | 必填 |
|--------|------|------|
| `OPENAI_API_KEY` | OpenAI API 密钥 | 使用 OpenAI 时必填 |
| `ZHIPU_API_KEY` | Zhipu AI API 密钥 | 使用智谱时必填 |
| `RUST_LOG` | 日志级别 (debug, info, warn, error) | 可选 |

## API 端点

| 端点 | 说明 |
|------|------|
| `GET /` | 健康检查 |
| `GET /health` | 健康检查 |
| `POST /v1/chat/completions` | Chat Completions API |
| `POST /v1/responses` | Responses API |
| `POST /v1/providers/zhipu/chat/completions` | Zhipu 直连端点 |

## 使用示例

### cURL 调用

```bash
# Chat Completions API
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "你好!"}]
  }'

# Zhipu AI
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZHIPU_API_KEY" \
  -d '{
    "model": "glm-4",
    "messages": [{"role": "user", "content": "你好，请介绍一下自己"}]
  }'
```

### Python SDK

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="dummy-key"
)

response = client.chat.completions.create(
    model="glm-4",
    messages=[{"role": "user", "content": "你好!"}]
)
print(response.choices[0].message.content)
```

## 支持的模型

### OpenAI
- gpt-4o
- gpt-4o-mini
- gpt-4-turbo
- gpt-3.5-turbo

### Zhipu AI (智谱 AI)
- glm-4
- glm-4-flash
- glm-4-plus

## 常见问题

### 服务启动失败
- 检查端口 8080 是否被占用: `lsof -i :8080`
- 查看日志了解详细错误信息

### API 请求失败
- 确认 API 密钥配置正确
- 检查网络连接
- 确认模型名称是否正确

### 请求超时
- 增加 provider 配置中的 `timeout` 值
- 检查网络延迟

## License

MIT
