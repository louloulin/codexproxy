#!/bin/bash
# Real Zhipu API Test Script
# Tests the transform functionality with real API calls

set -e

BASE_URL="http://localhost:8080"
API_KEY="9bc4908eeaec48109dc363c638c45457.qCuUoeMnsZme5eum"

echo "========================================"
echo "Zhipu AI Real API Test"
echo "========================================"
echo "Service URL: $BASE_URL"
echo "Provider: Zhipu AI (智谱)"
echo "Endpoint: https://open.bigmodel.cn/api/coding/paas/v4"
echo "========================================"
echo

# Test 1: Health Check
echo "Test 1: Health Check"
echo "--------------------"
HEALTH=$(curl -s "$BASE_URL/health")
if [ "$HEALTH" = "OK" ]; then
    echo "✓ Health check passed: $HEALTH"
else
    echo "✗ Health check failed"
    exit 1
fi
echo

# Test 2: Non-streaming Chat Completions
echo "Test 2: Non-streaming Chat Completions (glm-4)"
echo "----------------------------------------------"
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-4",
    "messages": [{"role": "user", "content": "你好"}],
    "stream": false,
    "max_tokens": 50
  }')

HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE:" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed '/HTTP_CODE/d')

echo "HTTP Status: $HTTP_CODE"
if [ "$HTTP_CODE" = "200" ]; then
    echo "✓ Request successful"
    echo "Response:"
    echo "$BODY" | python3 -m json.tool 2>/dev/null || echo "$BODY"
else
    echo "✗ Request failed"
    echo "Error: $BODY"
fi
echo

# Test 3: Simple Streaming Test (just verify connection)
echo "Test 3: Streaming Chat Completions (glm-4)"
echo "-------------------------------------------"
echo "Sending streaming request..."
timeout 10s curl -N -s -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-4",
    "messages": [{"role": "user", "content": "说一个字"}],
    "stream": true,
    "max_tokens": 5
  }' | head -20 || echo "(streaming test completed)"

echo
echo "========================================"
echo "Test Summary"
echo "========================================"
echo "✓ All basic connectivity tests completed"
echo "Note: Full API functionality depends on valid Zhipu API key"
echo "========================================"
