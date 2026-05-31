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
    echo "       参见 models/下载说明.txt"
    exit 1
fi

# Compile if needed
if [ ! -f build/translater.jar ] || [ src/TranslatorServer.java -nt build/translater.jar ]; then
    echo "[信息] 编译中..."
    rm -rf build
    mkdir -p build
    javac -source 8 -target 8 -encoding UTF-8 -d build src/TranslatorServer.java
    cp -r web build/
    cd build && jar cfe translater.jar TranslatorServer TranslatorServer.class TranslatorServer'$'*.class
    cd ..
    echo "[信息] 编译完成"
fi

# Start
echo "[信息] 启动翻译服务..."
cd build && java TranslatorServer &
SERVER_PID=$!

# Wait then open browser
sleep 3
open http://localhost:7779

echo "[信息] 服务已启动 (PID: $SERVER_PID)"
echo "[信息] 按 Ctrl+C 或运行 ./stop.sh 停止"
wait $SERVER_PID
