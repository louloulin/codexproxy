#!/bin/bash
# OpenAI Responses API Examples
# This demonstrates calling the /v1/responses endpoint
# Note: The Responses API uses "input" with array content (ContentBlock format)

BASE_URL="http://localhost:8080"

echo "========================================"
echo "Example 1: Non-streaming Responses API"
echo "========================================"

curl -s -X POST "${BASE_URL}/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "Hello! Please respond with a brief greeting."
          }
        ]
      }
    ],
    "stream": false
  }' | jq '.'

echo ""
echo "========================================"
echo "Example 2: Streaming Responses API"
echo "========================================"

curl -s -X POST "${BASE_URL}/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "Count from 1 to 5, one number per line."
          }
        ]
      }
    ],
    "stream": true
  }'

echo ""
echo ""
echo "========================================"
echo "Example 3: Responses API with instructions"
echo "========================================"

curl -s -X POST "${BASE_URL}/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "instructions": "You are a helpful assistant that responds in haiku format.",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "What is programming?"
          }
        ]
      }
    ],
    "stream": false
  }' | jq '.'

echo ""
echo "========================================"
echo "Example 4: Chat Completions API (for comparison)"
echo "========================================"

curl -s -X POST "${BASE_URL}/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {
        "role": "user",
        "content": "Say hello in 3 languages."
      }
    ],
    "stream": false
  }' | jq '.'