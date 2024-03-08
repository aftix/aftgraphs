#!/bin/bash

if [[ "$#" != "1" ]]; then
  echo "$0 requires one argument" >&2
  exit 1
fi

./buildwasm.sh "$SERVE_DEBUG"

if ! [[ -d "target/web/$1" ]]; then
  echo "target/web/$1 not a directory" >&2
  exit 1
fi

cp serve.json "target/web/$1"
yarn exec serve "target/web/$1"
