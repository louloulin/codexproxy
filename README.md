# OpenAI Proxy

A Rust-based API proxy server that transforms between OpenAI Chat Completions API and Responses API formats, with support for Zhipu AI (GLM models).

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

Create a `config.yaml` file:

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
  default: "openai"
  model_mapping:
    gpt-4*: "openai"
    gpt-3.5*: "openai"
    glm*: "zhipu"

logging:
  level: "info"
  format: "json"
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key |
| `ZHIPU_API_KEY` | Zhipu AI API key |

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

## License

MIT
