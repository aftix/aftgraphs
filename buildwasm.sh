#!/bin/bash

if [[ "$1" != "debug" ]]; then
  release=1
fi

if [[ $release -eq 1 ]]; then
  cargo build --release --lib --target wasm32-unknown-unknown
else
  cargo build --lib --target wasm32-unknown-unknown
fi

for dir in *; do
  # Don't try to compile files (only dirs)
  [ -d "$dir" ] || continue
  # Don't try to compile the imgui submodules
  [[ "$dir" =~ ^imgui.*$ ]] && continue
  # Only try to compile directories with Cargo.toml
  [[ -f "$dir/Cargo.toml" ]] || continue
  cd "$dir" || exit
  if [[ $release -eq 1 ]]; then
    cargo build --lib --target wasm32-unknown-unknown --profile web-release
  else
    cargo build --lib --target wasm32-unknown-unknown
  fi
  cd .. || exit
done

if [[ $release -eq 1 ]]; then
  targetDir="target/wasm32-unknown-unknown/web-release"
else
  targetDir="target/wasm32-unknown-unknown/debug"
fi

for file in "$targetDir/"*.wasm; do
  file="$(basename "$file")"
  file="${file%.*}"
  if [[ $release -eq 1 ]]; then
    wasm-bindgen --no-typescript --target web --out-dir "./target/web/$file" "./$targetDir/$file.wasm"
  else
    wasm-bindgen --no-typescript --debug --keep-debug --target web --out-dir "./target/web/$file" "./$targetDir/$file.wasm"
  fi
  cp index.html "./target/web/$file/." 
  sed -i "s/{{}}/$file/g" "./target/web/$file/index.html"
done
