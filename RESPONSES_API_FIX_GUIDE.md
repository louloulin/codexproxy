# Responses API 请求格式修复指南

## 问题描述

HTTP 422 错误：`input[0]: missing field 'type'`

这个错误表明请求体中的 `input[0]`（第一个输入项）缺少必需的 `type` 字段。

## 根本原因

Responses API 要求严格的 JSON 结构：

### ❌ 错误格式（会导致 422 错误）

```json
{
  "model": "glm-5",
  "input": [
    {
      "role": "user",
      "content": "你好"  // ❌ 错误：content 是字符串，且缺少 type 字段
    }
  ]
}
```

```json
{
  "model": "glm-5",
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": "你好"  // ❌ 错误：content 必须是数组
    }
  ]
}
```

### ✅ 正确格式

```json
{
  "model": "glm-5",
  "input": [
    {
      "type": "message",       // ✅ 必需：指定类型
      "role": "user",
      "content": [             // ✅ 必需：content 是数组
        {
          "type": "input_text", // ✅ 必需：内容块类型
          "text": "你好"
        }
      ]
    }
  ]
}
```

## 完整示例

### 1. 基本请求

```bash
curl -X POST http://localhost:9080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-5",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "你好"
          }
        ]
      }
    ],
    "stream": false
  }'
```

### 2. 带系统指令

```bash
curl -X POST http://localhost:9080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-5",
    "instructions": "你是一个有帮助的助手",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "你好"
          }
        ]
      }
    ]
  }'
```

### 3. 多轮对话

```bash
curl -X POST http://localhost:9080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-5",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "我的名字是小明"
          }
        ]
      },
      {
        "type": "message",
        "role": "assistant",
        "content": [
          {
            "type": "output_text",
            "text": "你好小明！"
          }
        ]
      },
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "我叫什么名字？"
          }
        ]
      }
    ]
  }'
```

## 内容块类型

### 输入类型

- `"type": "input_text"` - 文本输入
- `"type": "input_image"` - 图片输入

### 输出类型

- `"type": "output_text"` - 文本输出

## 调试方法

### 1. 启用详细日志

```bash
RUST_LOG=debug cargo run
```

### 2. 查看日志输出

服务器会记录每个输入项的详细信息：

```
DEBUG responses request body
DEBUG input item index=0 item=Message(MessageItem { ... })
```

### 3. 运行诊断脚本

```bash
./diagnose_responses_api.sh
```

这个脚本会测试各种格式并显示哪些通过/失败。

## 与 Chat Completions API 的区别

### Chat Completions API（宽松格式）

```json
{
  "messages": [
    {"role": "user", "content": "Hello"}  // content 可以是字符串
  ]
}
```

### Responses API（严格格式）

```json
{
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": [  // content 必须是数组
        {"type": "input_text", "text": "Hello"}
      ]
    }
  ]
}
```

## 检查清单

在发送请求前，确认：

- [ ] 每个input项都有 `"type": "message"` 字段
- [ ] `content` 是数组 `[...]` 而不是字符串
- [ ] 数组中的每个内容块都有 `"type": "input_text"` 字段
- [ ] JSON 格式正确（使用 JSON 验证器检查）

## 测试工具

### 在线 JSON 验证器

将你的 JSON 粘贴到 https://jsonlint.com/ 验证格式

### 本地测试

```bash
# 验证 JSON 格式
echo 'YOUR_JSON_HERE' | python3 -m json.tool

# 发送测试请求
curl -X POST http://localhost:9080/v1/responses \
  -H "Content-Type: application/json" \
  -d @- <<'EOF'
{
  "model": "glm-5",
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": [
        {"type": "input_text", "text": "test"}
      ]
    }
  ]
}
EOF
```

## 常见错误代码

| 错误 | 原因 | 解决方法 |
|------|------|---------|
| `input[0]: missing field 'type'` | input项缺少type字段 | 添加 `"type": "message"` |
| `content: invalid type` | content 不是数组 | 改为 `"content": [...]` |
| `content[0]: missing field 'type'` | 内容块缺少type | 添加 `"type": "input_text"` |

## 已修复的文件

以下文件中的示例已修复为正确格式：

- ✅ `tests/integration_tests.rs` - 集成测试
- ✅ `README.md` - 文档示例
- ✅ `codex1.md` - 开发计划
- ✅ `examples/responses_api.sh` - 示例脚本
- ✅ `examples/RESPONSES_API_GUIDE.md` - API 指南

## 获取帮助

如果问题仍然存在：

1. 运行诊断脚本：`./diagnose_responses_api.sh`
2. 查看服务器日志（启用 DEBUG 级别）
3. 检查发送的确切 JSON 格式
4. 使用 `jq` 或 Python 格式化 JSON 以便调试

```bash
# 格式化 JSON 以便调试
cat your_request.json | python3 -m json.tool
```
