#!/bin/bash

# 流式响应诊断脚本
# 用于自动分析 "stream closed before response.completed" 错误

set -e

LOG_FILE="streaming_debug.log"
TEMP_LOG="temp_debug.log"

echo "🔍 rcodex 流式响应诊断工具"
echo "================================"
echo ""

# 检查是否已编译
if [ ! -f "target/release/rcodex" ]; then
    echo "⚠️  未找到编译后的二进制文件，正在编译..."
    cargo build --release
fi

echo "📋 诊断步骤:"
echo "1. 启动服务 (DEBUG 日志级别)"
echo "2. 等待测试请求"
echo "3. 分析日志输出"
echo ""

# 清理旧日志
> "$LOG_FILE"

# 启动服务并捕获日志
echo "🚀 启动 rcodex 服务..."
echo "   日志文件: $LOG_FILE"
echo ""

# 使用后台进程运行服务
RUST_LOG=rcodex=debug,reqwest=info \
    cargo run --release 2>&1 | tee "$TEMP_LOG" &
SERVER_PID=$!

# 等待服务启动
echo "⏳ 等待服务启动 (5秒)..."
sleep 5

# 检查服务是否运行
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "❌ 服务启动失败！"
    cat "$TEMP_LOG"
    exit 1
fi

echo "✅ 服务已启动 (PID: $SERVER_PID)"
echo ""

# 监控日志
echo "📊 实时监控日志..."
echo "   按 Ctrl+C 停止"
echo ""

# 日志分析函数
analyze_log() {
    local log_file="$1"

    echo ""
    echo "================================"
    echo "📈 日志分析报告"
    echo "================================"
    echo ""

    # 检查 provider 处理
    echo "1️⃣  Provider SSE 处理:"
    grep "processing SSE stream" "$log_file" | tail -5 || echo "   ⚠️  未找到 SSE 处理日志"
    echo ""

    # 检查 chunk 解析
    echo "2️⃣  Chunk 解析统计:"
    grep "SSE stream processing complete" "$log_file" | tail -5 || echo "   ⚠️  未找到解析完成日志"
    echo ""

    # 检查 handler 接收
    echo "3️⃣  Handler 接收 chunks:"
    grep "provider streaming response received" "$log_file" | tail -5 || echo "   ⚠️  未找到 handler 接收日志"
    echo ""

    # 检查 payload 生成
    echo "4️⃣  Payload 生成:"
    grep "generated payloads, adding response.completed" "$log_file" | tail -5 || echo "   ⚠️  未找到 payload 生成日志"
    echo ""

    # 检查转换完成
    echo "5️⃣  转换完成:"
    grep "responses_protocol_payloads_from_chat_chunks complete" "$log_file" | tail -5 || echo "   ⚠️  未找到转换完成日志"
    echo ""

    # 检查错误和警告
    echo "6️⃣  错误和警告:"
    local errors=$(grep -c "ERROR" "$log_file" 2>/dev/null || echo "0")
    local warnings=$(grep -c "WARN" "$log_file" 2>/dev/null || echo "0")
    echo "   错误数: $errors"
    echo "   警告数: $warnings"
    echo ""

    if [ "$errors" -gt 0 ]; then
        echo "   ❌ 发现错误:"
        grep "ERROR" "$log_file" | tail -10
        echo ""
    fi

    if [ "$warnings" -gt 0 ]; then
        echo "   ⚠️  发现警告:"
        grep "WARN" "$log_file" | tail -10
        echo ""
    fi

    # 关键指标分析
    echo "7️⃣  关键指标:"

    # 提取 parsed_chunks 数量
    local parsed=$(grep "parsed_chunks" "$log_file" | tail -1 | grep -oP 'parsed_chunks=\K[0-9]+' || echo "N/A")
    echo "   解析的 chunks: $parsed"

    # 提取 event_count 数量
    local events=$(grep "event_count" "$log_file" | tail -1 | grep -oP 'event_count=\K[0-9]+' || echo "N/A")
    echo "   生成的事件数: $events"

    # 提取 total_payloads 数量
    local payloads=$(grep "total_payloads" "$log_file" | tail -1 | grep -oP 'total_payloads=\K[0-9]+' || echo "N/A")
    echo "   总 payloads: $payloads"

    echo ""

    # 诊断结论
    echo "8️⃣  诊断结论:"

    if [ "$parsed" = "0" ]; then
        echo "   ❌ 问题: 没有解析到任何 chunk"
        echo "   原因可能:"
        echo "     - Provider 返回格式不匹配"
        echo "     - JSON 解析失败"
        echo "     - SSE 流为空"
        echo ""
        echo "   建议:"
        echo "     - 检查 provider 响应格式"
        echo "     - 查看解析错误日志 (WARN failed to parse chunk)"
        echo "     - 添加原始响应日志"
    elif [ "$parsed" != "N/A" ] && [ "$parsed" -gt 0 ]; then
        echo "   ✅ 成功解析 $parsed 个 chunks"

        if [ "$payloads" = "0" ] || [ "$payloads" = "N/A" ]; then
            echo "   ❌ 问题: 没有生成任何 payload"
            echo "   原因可能:"
            echo "     - 转换函数逻辑错误"
            echo "     - 所有 chunks 被过滤掉"
        else
            echo "   ✅ 成功生成 $payloads 个 payloads"

            if grep -q "response.completed" "$log_file"; then
                echo "   ✅ response.completed 事件已生成"
                echo ""
                echo "   🎉 流式响应处理正常！"
            else
                echo "   ❌ 问题: 缺少 response.completed 事件"
                echo "   原因可能:"
                echo "     - chunks 为空触发了快速路径"
                echo "     - 转换函数提前返回"
            fi
        fi
    fi

    echo ""
}

# 后台监控日志
tail -f "$TEMP_LOG" | while IFS= read -r line; do
    echo "$line" >> "$LOG_FILE"

    # 高亮关键日志
    if echo "$line" | grep -q "parsed_chunks"; then
        echo "  👆 CHUNK PARSE INFO" | tee -a "$LOG_FILE"
    fi

    if echo "$line" | grep -q "response.completed"; then
        echo "  ✅ RESPONSE COMPLETED EVENT" | tee -a "$LOG_FILE"
    fi

    if echo "$line" | grep -q "ERROR"; then
        echo "  ❌ ERROR DETECTED" | tee -a "$LOG_FILE"
    fi

    if echo "$line" | grep -q "WARN"; then
        echo "  ⚠️  WARNING DETECTED" | tee -a "$LOG_FILE"
    fi
done &

MONITOR_PID=$!

# 清理函数
cleanup() {
    echo ""
    echo "🛑 停止服务..."
    kill $SERVER_PID 2>/dev/null || true
    kill $MONITOR_PID 2>/dev/null || true

    # 等待进程结束
    wait $SERVER_PID 2>/dev/null || true
    wait $MONITOR_PID 2>/dev/null || true

    echo ""
    echo "📊 分析日志..."
    analyze_log "$LOG_FILE"

    echo ""
    echo "📄 日志已保存到: $LOG_FILE"
    echo "🔍 完整日志: $TEMP_LOG"
    echo ""

    exit 0
}

# 捕获中断信号
trap cleanup INT TERM

# 等待用户
echo "服务正在运行..."
echo ""
echo "📝 测试命令:"
echo "curl -X POST http://localhost:8080/v1/responses \\"
echo "  -H 'Content-Type: application/json' \\"
echo "  -d '{"
echo "    \"model\": \"gpt-4\","
echo "    \"input\": [{\"type\": \"message\", \"role\": \"user\", \"content\": \"Hello\"}],"
echo "    \"stream\": true"
echo "  }'"
echo ""
echo "按 Ctrl+C 停止并分析..."
echo ""

# 等待
wait $SERVER_PID
