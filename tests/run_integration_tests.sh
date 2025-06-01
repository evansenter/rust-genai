#!/bin/bash

# Script to run integration tests with delays to avoid API rate limits
# Usage: ./run_integration_tests.sh

echo "Running integration tests with delays to avoid rate limits..."
echo "Note: This requires GEMINI_API_KEY to be set"

# Check if GEMINI_API_KEY is set
if [ -z "$GEMINI_API_KEY" ]; then
    echo "Error: GEMINI_API_KEY environment variable is not set"
    exit 1
fi

# Run each test file with delays between them
echo "Running integration_tests.rs..."
cargo test --test integration_tests -- --ignored --test-threads=1
sleep 10

echo "Running function_calling_tests.rs..."
cargo test --test function_calling_tests -- --ignored --test-threads=1
sleep 10

echo "Running debug_mode_tests.rs..."
cargo test --test debug_mode_tests -- --ignored --test-threads=1
sleep 10

echo "Running error_handling_tests.rs..."
cargo test --test error_handling_tests -- --ignored --test-threads=1

echo "All integration tests completed!" 