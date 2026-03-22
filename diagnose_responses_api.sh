#!/bin/bash
# 完整诊断脚本 - 测试 Responses API 的所有格式

BASE_URL="http://localhost:9080"
ERROR_COUNT=0

echo "=========================================="
echo "Responses API 完整诊断脚本"
echo "=========================================="
echo "目标服务器: $BASE_URL"
echo "时间: $(date)"
echo "=========================================="
echo

# 函数：测试请求
test_request() {
    local test_name="$1"
    local expected_status="$2"
    local json_data="$3"

    echo "测试: $test_name"
    echo "预期状态码: $expected_status"
    echo "请求数据:"
    echo "$json_data" | python3 -m json.tool 2>/dev/null || echo "$json_data"
    echo

    HTTP_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/v1/responses" \
        -H "Content-Type: application/json" \
        -d "$json_data" 2>&1)

    HTTP_CODE=$(echo "$HTTP_RESPONSE" | grep "HTTP_CODE:" | cut -d: -f2)
    BODY=$(echo "$HTTP_RESPONSE" | sed '/HTTP_CODE/d')

    echo "实际状态码: $HTTP_CODE"
    if [ "$HTTP_CODE" = "$expected_status" ]; then
        echo "✅ 测试通过"
    else
        echo "❌ 测试失败"
        ERROR_COUNT=$((ERROR_COUNT + 1))
    fi

    echo "响应内容:"
    echo "$BODY" | python3 -m json.tool 2>/dev/null || echo "$BODY"
    echo
    echo "----------------------------------------"
    echo
}

# 1. 测试正确的格式（应该成功）
test_request \
    "正确格式 - content 为数组" \
    "200" \
    '{
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

# 2. 测试错误的格式 - content 为字符串（应该失败）
test_request \
    "错误格式 - content 为字符串（应失败）" \
    "422" \
    '{
        "model": "glm-5",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": "你好"
            }
        ],
        "stream": false
    }'

# 3. 测试错误的格式 - 缺少 type 字段（应该失败）
test_request \
    "错误格式 - 缺少 type 字段（应失败）" \
    "422" \
    '{
        "model": "glm-5",
        "input": [
            {
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

# 4. 测试错误的格式 - content 数组中的元素缺少 type（应该失败）
test_request \
    "错误格式 - content 数组缺少 type（应失败）" \
    "422" \
    '{
        "model": "glm-5",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "text": "你好"
                    }
                ]
            }
        ],
        "stream": false
    }'

# 5. 测试系统消息
test_request \
    "正确格式 - 包含系统指令" \
    "200" \
    '{
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
        ],
        "stream": false
    }'

# 6. 测试多轮对话
test_request \
    "正确格式 - 多轮对话" \
    "200" \
    '{
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
        ],
        "stream": false
    }'

# 总结
echo "=========================================="
echo "测试总结"
echo "=========================================="
if [ $ERROR_COUNT -eq 0 ]; then
    echo "✅ 所有测试通过！"
else
    echo "❌ 发现 $ERROR_COUNT 个错误"
fi
echo
echo "关键要点："
echo "1. 每个input项必须有 'type' 字段"
echo "2. content 必须是数组: [{\"type\": \"input_text\", \"text\": \"...\"}]"
echo "3. 不能是字符串: \"content\": \"...\""
echo "=========================================="
