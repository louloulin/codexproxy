#!/usr/bin/env python3
"""
Zhipu AI Real API Test with Transform Layer

This script tests the real Zhipu AI API through our proxy service
to validate the transform functionality.
"""

import json
import requests
import sys

BASE_URL = "http://localhost:8080"


def test_zhipu_chat_completions():
    """Test Zhipu AI Chat Completions API"""
    print("=" * 60)
    print("Testing Zhipu AI Chat Completions API (Real Call)")
    print("=" * 60)

    url = f"{BASE_URL}/v1/chat/completions"
    payload = {
        "model": "glm-4",  # This routes to Zhipu provider
        "messages": [
            {
                "role": "user",
                "content": "你好，请用一句话介绍自己。"
            }
        ],
        "stream": False,
        "temperature": 0.7
    }

    try:
        print(f"\nRequest URL: {url}")
        print(f"Request Payload: {json.dumps(payload, indent=2, ensure_ascii=False)}")
        response = requests.post(url, json=payload, timeout=30)
        print(f"\nStatus Code: {response.status_code}")

        if response.status_code == 200:
            result = response.json()
            print(f"Response: {json.dumps(result, indent=2, ensure_ascii=False)}")

            # Validate response structure
            if "choices" in result and len(result["choices"]) > 0:
                content = result["choices"][0]["message"]["content"]
                print(f"\n✓ AI Response: {content}")
                return True
            else:
                print(f"\n✗ Invalid response structure")
                return False
        else:
            print(f"Error Response: {response.text}")
            return False

    except requests.exceptions.RequestException as e:
        print(f"Request Error: {e}")
        return False
    except Exception as e:
        print(f"Unexpected Error: {e}")
        return False


def test_zhipu_streaming():
    """Test Zhipu AI Streaming Chat Completions"""
    print("\n" + "=" * 60)
    print("Testing Zhipu AI Streaming API (Real Call)")
    print("=" * 60)

    url = f"{BASE_URL}/v1/chat/completions"
    payload = {
        "model": "glm-4",
        "messages": [
            {
                "role": "user",
                "content": "从1数到3"
            }
        ],
        "stream": True
    }

    try:
        print(f"\nRequest URL: {url}")
        print(f"Request Payload: {json.dumps(payload, indent=2, ensure_ascii=False)}")
        response = requests.post(url, json=payload, stream=True, timeout=30)
        print(f"\nStatus Code: {response.status_code}")
        print("Streaming response:")

        full_content = ""
        for line in response.iter_lines():
            if line:
                decoded = line.decode('utf-8')
                if decoded.startswith('data: '):
                    data = decoded[6:]  # Remove 'data: ' prefix
                    if data == '[DONE]':
                        print("\n[DONE]")
                        break
                    else:
                        try:
                            chunk = json.loads(data)
                            if 'choices' in chunk and len(chunk['choices']) > 0:
                                delta = chunk['choices'][0].get('delta', {})
                                content = delta.get('content', '')
                                if content:
                                    print(content, end='', flush=True)
                                    full_content += content
                        except json.JSONDecodeError:
                            pass

        print(f"\n\n✓ Full Response: {full_content}")
        return response.status_code == 200 and len(full_content) > 0

    except requests.exceptions.RequestException as e:
        print(f"Request Error: {e}")
        return False
    except Exception as e:
        print(f"Unexpected Error: {e}")
        return False


def test_transform_roundtrip():
    """Test transform layer through Chat Completions endpoint"""
    print("\n" + "=" * 60)
    print("Testing Transform Layer (Chat → Responses → Chat)")
    print("=" * 60)

    url = f"{BASE_URL}/v1/chat/completions"
    payload = {
        "model": "glm-4",
        "messages": [
            {
                "role": "user",
                "content": "测试转换功能"
            }
        ],
        "stream": False
    }

    try:
        print(f"\nRequest URL: {url}")
        print(f"Request Payload: {json.dumps(payload, indent=2, ensure_ascii=False)}")
        response = requests.post(url, json=payload, timeout=30)
        print(f"\nStatus Code: {response.status_code}")

        if response.status_code == 200:
            result = response.json()
            print(f"Response: {json.dumps(result, indent=2, ensure_ascii=False)}")

            # The response should have standard Chat Completions format
            # even though it went through the transform layer
            if "choices" in result and "usage" in result:
                print("\n✓ Transform layer working correctly")
                print(f"  - Response has standard Chat Completions format")
                print(f"  - Usage: {result['usage']}")
                return True
            else:
                print("\n✗ Response format unexpected")
                return False
        else:
            print(f"Error Response: {response.text}")
            return False

    except requests.exceptions.RequestException as e:
        print(f"Request Error: {e}")
        return False
    except Exception as e:
        print(f"Unexpected Error: {e}")
        return False


def test_health():
    """Test health endpoint"""
    print("\n" + "=" * 60)
    print("Testing Health Check")
    print("=" * 60)

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
    print("=" * 60)
    print("Zhipu AI Real API Test Suite")
    print("=" * 60)
    print(f"Service URL: {BASE_URL}")
    print(f"Provider: Zhipu AI (智谱)")
    print(f"Endpoint: https://open.bigmodel.cn/api/coding/paas/v4")
    print("=" * 60)

    results = {
        "Health Check": test_health(),
        "Chat Completions": test_zhipu_chat_completions(),
        "Streaming": test_zhipu_streaming(),
        "Transform Layer": test_transform_roundtrip(),
    }

    print("\n" + "=" * 60)
    print("Test Results Summary")
    print("=" * 60)
    for test_name, passed in results.items():
        status = "✓ PASSED" if passed else "✗ FAILED"
        print(f"{test_name}: {status}")

    all_passed = all(results.values())
    print("\n" + "=" * 60)
    if all_passed:
        print("✓ All tests passed!")
    else:
        print("✗ Some tests failed")
    print("=" * 60)

    sys.exit(0 if all_passed else 1)
