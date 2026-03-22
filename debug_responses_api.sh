#!/bin/bash
# Debug script for Responses API
# This script tests different JSON formats to help identify issues

BASE_URL="http://localhost:9080"

echo "=========================================="
echo "Responses API Debug Script"
echo "=========================================="
echo

# Test 1: CORRECT format with array of content blocks
echo "Test 1: Correct format (array of content blocks)"
echo "--------------------------------------------------"
curl -X POST "$BASE_URL/v1/responses" \
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
  }' 2>&1

echo
echo

# Test 2: WRONG format (plain string content - will fail)
echo "Test 2: WRONG format (plain string - this WILL fail)"
echo "-------------------------------------------------------"
curl -X POST "$BASE_URL/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-5",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": "你好"
      }
    ],
    "stream": false
  }' 2>&1

echo
echo

# Test 3: WRONG format (missing type field - will fail)
echo "Test 3: WRONG format (missing type - this WILL fail)"
echo "-------------------------------------------------------"
curl -X POST "$BASE_URL/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-5",
    "input": [
      {
        "role": "user",
        "content": "你好"
      }
    ],
    "stream": false
  }' 2>&1

echo
echo

echo "=========================================="
echo "Summary"
echo "=========================================="
echo "✓ Test 1 should succeed (correct format)"
echo "✗ Test 2 should fail (content must be array)"
echo "✗ Test 3 should fail (missing type field)"
echo
echo "Correct format:"
echo "  content: [{\"type\": \"input_text\", \"text\": \"...\"}]"
echo
echo "Wrong formats:"
echo "  content: \"plain string\""
echo "  missing 'type' field in input items"
echo "=========================================="
