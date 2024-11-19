#!/bin/bash

wget https://github.com/WebAssembly/binaryen/releases/download/version_117/binaryen-version_117-x86_64-linux.tar.gz
tar -xf binaryen-version_117-x86_64-linux.tar.gz
mv binaryen-version_117/bin/wasm-opt /usr/local/bin
rm -rf binaryen-version_117*

