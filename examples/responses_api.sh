#!/bin/bash
# OpenAI Responses API Examples
# Demonstrates usage of the /v1/responses endpoint

set -e

BASE_URL="http://localhost:8080"

echo "========================================"
echo "OpenAI Responses API Examples"
echo "========================================"
echo "Service URL: $BASE_URL"
echo "Endpoint: /v1/responses"
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

# Test 2: Non-streaming Responses API - Simple Question
echo "Test 2: Non-streaming Responses API - Simple Question"
echo "------------------------------------------------------"
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-4",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "What is 2+2? Answer briefly."
          }
        ]
      }
    ],
    "stream": false,
    "max_tokens": 20
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

# Test 3: Non-streaming Responses API - With System Instructions
echo "Test 3: Non-streaming Responses API - With System Instructions"
echo "---------------------------------------------------------------"
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-4",
    "instructions": "You are a helpful assistant that responds in exactly one sentence.",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "What is the capital of France?"
          }
        ]
      }
    ],
    "stream": false,
    "max_tokens": 30
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

# Test 4: Streaming Responses API
echo "Test 4: Streaming Responses API"
echo "--------------------------------"
echo "Sending streaming request..."
curl -N -s -X POST "$BASE_URL/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-4",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "Count from 1 to 5"
          }
        ]
      }
    ],
    "stream": true,
    "max_tokens": 20
  }' 2>&1 | perl -pe 'alarm 10' || echo "(streaming completed)"

echo
echo

# Test 5: Multi-turn Conversation
echo "Test 5: Multi-turn Conversation"
echo "--------------------------------"
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-4",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "My name is Alice."
          }
        ]
      },
      {
        "type": "message",
        "role": "assistant",
        "content": [
          {
            "type": "output_text",
            "text": "Hello Alice! How can I help you today?"
          }
        ]
      },
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "What is my name?"
          }
        ]
      }
    ],
    "stream": false,
    "max_tokens": 30
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

# Test 6: Comparison with Chat Completions API
echo "Test 6: Comparison - Chat Completions API"
echo "-------------------------------------------"
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-4",
    "messages": [
      {
        "role": "user",
        "content": "What is 2+2? Answer briefly."
      }
    ],
    "stream": false,
    "max_tokens": 20
  }')

HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE:" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed '/HTTP_CODE/d')

echo "HTTP Status: $HTTP_CODE"
if [ "$HTTP_CODE" = "200" ]; then
    echo "✓ Request successful"
    echo "Response (Chat Completions format):"
    echo "$BODY" | python3 -m json.tool 2>/dev/null || echo "$BODY"
else
    echo "✗ Request failed"
    echo "Error: $BODY"
fi
echo

echo "========================================"
echo "Test Summary"
echo "========================================"
echo "✓ All Responses API examples completed"
echo
echo "Key Differences:"
echo "- Responses API uses 'input' array with 'type: message'"
echo "- Chat Completions uses 'messages' array with 'role'"
echo "- Responses API uses 'instructions' for system prompt"
echo "- Chat Completions uses 'role: system' for system prompt"
echo "- Both support streaming with SSE format"
echo "========================================"
