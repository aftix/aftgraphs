#!/bin/bash

cargo build --release --lib --target wasm32-unknown-unknown

for dir in *; do
  [ -d "$dir" ] || continue
  [ "$dir" = ".git" ] && continue
  [ "$dir" = "target" ] && continue
  cd "$dir" || exit
  cargo build --release --lib --target wasm32-unknown-unknown
  cd .. || exit
done

for file in target/wasm32-unknown-unknown/release/*.wasm; do
  file="$(basename "$file")"
  file="${file%.*}"
  wasm-bindgen --no-typescript --target web --out-dir "./target/web/$file" "./target/wasm32-unknown-unknown/release/$file.wasm"
  cp index.html "./target/web/$file/." 
  sed -i "s/{{}}/$file/g" "./target/web/$file/index.html"
done
