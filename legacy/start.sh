#!/bin/bash
# 本地翻译器 - Mac 启动脚本

cd "$(dirname "$0")"

# Check llama-server
if ! command -v llama-server &>/dev/null; then
    echo "[错误] 未找到 llama-server，请先安装: brew install llama.cpp"
    exit 1
fi

# Check model
model_count=$(ls models/*.gguf 2>/dev/null | wc -l)
if [ "$model_count" -eq 0 ]; then
    echo "[错误] models/ 目录中没有 GGUF 模型文件"
    exit 1
fi

# Check jar
if [ ! -f translater.jar ]; then
    echo "[错误] 未找到 translater.jar，请先编译"
    exit 1
fi

# Start
echo "[信息] 启动翻译服务..."
java -jar translater.jar &
SERVER_PID=$!

# Wait then open browser
sleep 3
open http://localhost:7779

echo "[信息] 服务已启动 (PID: $SERVER_PID)"
echo "[信息] 按 Ctrl+C 或运行 ./stop.sh 停止"
wait $SERVER_PID
