#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "Running simple Python file toggle test..."

# Save original file
cp tests/fixtures/simple_python.py tests/fixtures/simple_python.py.orig

# Test 1: Toggle the debug section ON (comment it out)
echo "Test 1: Toggle 'debug' section ON (comment it out)"
cargo run -- --section debug --force on tests/fixtures/simple_python.py

# Check if it was properly commented
if grep -q "#.*print(\"Debug information\")" tests/fixtures/simple_python.py; then
  echo -e "${GREEN}✓ Debug section was properly commented out${NC}"
else
  echo -e "${RED}✗ Failed to comment out debug section${NC}"
  exit 1
fi

# Test 2: Toggle the debug section OFF (uncomment it)
echo "Test 2: Toggle 'debug' section OFF (uncomment it)"
cargo run -- --section debug --force off tests/fixtures/simple_python.py

# Check if it was properly uncommented
if grep -q "print(\"Debug information\")" tests/fixtures/simple_python.py && ! grep -q "#.*print(\"Debug information\")" tests/fixtures/simple_python.py; then
  echo -e "${GREEN}✓ Debug section was properly uncommented${NC}"
else
  echo -e "${RED}✗ Failed to uncomment debug section${NC}"
  exit 1
fi

# Test 3: Toggle the feature section ON (comment it out)
echo "Test 3: Toggle 'feature' section ON (comment it out)"
cargo run -- --section feature --force on tests/fixtures/simple_python.py

# Check if it was properly commented
if grep -q "#.*print(\"This is an experimental feature\")" tests/fixtures/simple_python.py; then
  echo -e "${GREEN}✓ Feature section was properly commented out${NC}"
else
  echo -e "${RED}✗ Failed to comment out feature section${NC}"
  exit 1
fi

# Test 4: Toggle by line range
echo "Test 4: Toggle line range (lines 3-6)"
cargo run -- --line 3:6 tests/fixtures/simple_python.py

# Test 5: Toggle all sections at once
echo "Test 5: Toggle all sections at once"
cargo run -- --section debug --section feature tests/fixtures/simple_python.py

# Restore original file
mv tests/fixtures/simple_python.py.orig tests/fixtures/simple_python.py

echo -e "${GREEN}All tests passed!${NC}" 