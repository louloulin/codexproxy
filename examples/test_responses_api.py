#!/usr/bin/env python3
"""
OpenAI Responses API Example

This script demonstrates how to call the /v1/responses endpoint
using Python requests library.
"""

import json
import requests

BASE_URL = "http://localhost:8080"


def test_non_streaming_responses():
    """Test non-streaming Responses API call"""
    print("=" * 50)
    print("Testing Non-Streaming Responses API")
    print("=" * 50)

    url = f"{BASE_URL}/v1/responses"
    payload = {
        "model": "gpt-4o",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "Say 'Hello, World!' in exactly those words."
                    }
                ]
            }
        ],
        "stream": False
    }

    try:
        response = requests.post(url, json=payload, timeout=30)
        print(f"Status Code: {response.status_code}")
        print(f"Response: {json.dumps(response.json(), indent=2)}")
        return response.status_code == 200
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        return False


def test_streaming_responses():
    """Test streaming Responses API call"""
    print("\n" + "=" * 50)
    print("Testing Streaming Responses API")
    print("=" * 50)

    url = f"{BASE_URL}/v1/responses"
    payload = {
        "model": "gpt-4o",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "Count from 1 to 3"
                    }
                ]
            }
        ],
        "stream": True
    }

    try:
        response = requests.post(url, json=payload, stream=True, timeout=30)
        print(f"Status Code: {response.status_code}")
        print("Streaming response:")
        for line in response.iter_lines():
            if line:
                decoded = line.decode('utf-8')
                if decoded.startswith('data: '):
                    data = decoded[6:]  # Remove 'data: ' prefix
                    if data == '[DONE]':
                        print("\n[DONE]")
                    else:
                        chunk = json.loads(data)
                        if 'output' in chunk and len(chunk['output']) > 0:
                            content = chunk['output'][0].get('content', '')
                            print(content, end='', flush=True)
        return response.status_code == 200
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        return False


def test_chat_completions():
    """Test Chat Completions API for comparison"""
    print("\n" + "=" * 50)
    print("Testing Chat Completions API")
    print("=" * 50)

    url = f"{BASE_URL}/v1/chat/completions"
    payload = {
        "model": "gpt-4o",
        "messages": [
            {
                "role": "user",
                "content": "Say 'test' in one word."
            }
        ],
        "stream": False
    }

    try:
        response = requests.post(url, json=payload, timeout=30)
        print(f"Status Code: {response.status_code}")
        print(f"Response: {json.dumps(response.json(), indent=2)}")
        return response.status_code == 200
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        return False


def test_health_check():
    """Test health check endpoint"""
    print("\n" + "=" * 50)
    print("Testing Health Check")
    print("=" * 50)

    url = f"{BASE_URL}/health"
    try:
        response = requests.get(url, timeout=5)
        print(f"Status Code: {response.status_code}")
        print(f"Response: {response.text}")
        return response.status_code == 200
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        return False


if __name__ == "__main__":
    print("OpenAI Responses API Test Suite")
    print("================================\n")

    results = {
        "Health Check": test_health_check(),
        # Note: The following tests require a valid OpenAI API key
        # "Non-Streaming Responses": test_non_streaming_responses(),
        # "Streaming Responses": test_streaming_responses(),
        # "Chat Completions": test_chat_completions(),
    }

    print("\n" + "=" * 50)
    print("Test Results Summary")
    print("=" * 50)
    for test_name, passed in results.items():
        status = "✓ PASSED" if passed else "✗ FAILED"
        print(f"{test_name}: {status}")

    all_passed = all(results.values())
    exit(0 if all_passed else 1)