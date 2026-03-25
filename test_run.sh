#!/bin/bash
# 一键测试脚本 - 使用 Ollama 对亡弟归来项目运行 auto-drama
set -e

PROJECT_DIR="/Users/apple/Documents/漫剧剧本/亡弟归来"
AUTO_DRAMA_DIR="/Users/apple/Documents/漫剧剧本/auto-drama"

echo "========================================="
echo "  Auto-Drama 循环测试 - 亡弟归来"
echo "========================================="

# 1. 检查 Ollama 云端服务
echo ""
echo "[1/4] 检查 Ollama 服务..."
echo "✅ 使用 Ollama 云端服务 (https://api.ollama.com)"

# 2. 复制 config.toml 到亡弟归来目录
echo ""
echo "[2/4] 准备项目配置..."
if [ ! -f "$PROJECT_DIR/config.toml" ]; then
    cp "$AUTO_DRAMA_DIR/config.toml" "$PROJECT_DIR/config.toml"
    echo "✅ 已复制 config.toml 到 $PROJECT_DIR"
else
    echo "✅ config.toml 已存在"
fi

# 3. 创建必要的目录
mkdir -p "$PROJECT_DIR/output" "$PROJECT_DIR/skills" "$PROJECT_DIR/templates" "$PROJECT_DIR/.auto-drama"
echo "✅ 目录结构就绪"

# 4. 编译并运行健康检查
echo ""
echo "[3/4] 编译 auto-drama..."
cd "$AUTO_DRAMA_DIR"
cargo build 2>&1 | tail -5
echo "✅ 编译完成"

# 5. 运行完整流程
echo ""
echo "[4/4] 启动创作循环..."
echo "========================================="
cd "$AUTO_DRAMA_DIR"
RUST_LOG=info cargo run -- \
    -p "$PROJECT_DIR" \
    create \
    --title "亡弟归来" \
    --genre "古装权谋" \
    --theme "兄弟情义与家国天下" \
    --episodes 80 \
    --era "隋末唐初" \
    --style "快节奏、多反转" \
    --audience "18-35岁年轻观众" \
    --tone "情感丰富、有笑有泪" \
    --reference "李世民、李智云历史背景"

echo ""
echo "========================================="
echo "  测试完成"
echo "========================================="
