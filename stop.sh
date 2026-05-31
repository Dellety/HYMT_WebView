#!/bin/bash
# 本地翻译器 - Mac 停止脚本

echo "[信息] 正在停止服务..."

# Kill llama-server
pkill -f "llama-server.*7780" 2>/dev/null && echo "[信息] llama-server 已停止"

# Kill TranslatorServer
pkill -f "java TranslatorServer" 2>/dev/null && echo "[信息] Java 服务已停止"

echo "[信息] 完成"
