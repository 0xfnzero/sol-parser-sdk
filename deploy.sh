#!/bin/bash
set -euo pipefail

# Skip sourcing .zshrc to avoid potential issues

rm -rf deploy_package
rm -rf deploy_package.tar.gz

echo "🚀 Building wick-catching-bot for Linux target..."
# Use clean build directory and compile the main binary
CARGO_TARGET_DIR=target cargo build --target x86_64-unknown-linux-gnu --release --bin wick-bot

echo "📦 Creating deployment package..."
mkdir -p deploy_package
rm -rf deploy_package/*

# Copy the bot binary file
RELEASE_DIR="target/x86_64-unknown-linux-gnu/release"
if [[ -f "$RELEASE_DIR/wick-bot" ]]; then
    cp "$RELEASE_DIR/wick-bot" deploy_package/wick-bot
    chmod +x deploy_package/wick-bot
    echo "✅ Copied wick-bot binary"
else
    echo "❌ Error: wick-bot binary not found in $RELEASE_DIR"
    echo "Available files:"
    ls -la "$RELEASE_DIR/" | head -10
    exit 1
fi

# Copy config directory if it exists
if [[ -d "config/" ]]; then
    cp -rf config/ deploy_package/config/
    echo "✅ Copied config directory"
else
    echo "⚠️ Warning: config directory not found, skipping..."
fi

echo "📦 Creating tarball..."
tar -czvf deploy_package.tar.gz -C deploy_package .

echo "🚀 Deploying to server..."
echo "Creating target directory and uploading..."

# First, create the directory and upload the file
scp deploy_package.tar.gz ubuntu@64.130.37.195:~/

if [ $? -eq 0 ]; then
    echo "✅ Upload successful!"
        
    echo "✅ Deployment complete!"
    echo ""
    echo "🚀 To start the bot:"
    echo "ssh ubuntu@64.130.37.195"
    echo "cd ~/wick-catching-bot"
    echo "./wick-bot"
    echo ""
    echo "📝 Remember to:"
    echo "1. Update config/app.yaml with your private key"
    echo "2. Make sure Redis server is running"
    echo "3. Configure any environment variables as needed"
else
    echo "❌ Upload failed!"
    echo ""
    echo "📋 Manual deployment steps:"
    echo "1. Copy deploy_package.tar.gz to your server manually"
    echo "2. On the server, run:"
    echo "   mkdir -p ~/wick-catching-bot"
    echo "   cd ~/wick-catching-bot"
    echo "   tar -xzf ~/deploy_package.tar.gz"
    echo "   chmod +x wick-bot"
    echo "   ./wick-bot"
fi

# Clean up deployment files
echo "🧹 Cleaning up..."
rm -rf deploy_package
rm -rf deploy_package.tar.gz