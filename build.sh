#!/bin/bash

# Build and Run script for Family Room (macOS)
echo "Building Family Room (Cozy Glass Edition)..."

# Build the executable
swift build -c release

# Check if build succeeded
if [ $? -eq 0 ]; then
    echo "Build successful! Launching app..."
    ./.build/release/FamilyRoom
else
    echo "Build failed."
    exit 1
fi
