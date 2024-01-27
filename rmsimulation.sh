#!/bin/bash

if [[ -z "$1" ]]; then
  echo "Need argument \$1 to be the name to add" >&2
  exit 1
fi

NAME="$(tr '[:upper:]' '[:lower:]' <<< "$1")"
if ! [[ -e "$NAME" ]]; then
  echo "$NAME does not exist" >&2
  exit 2
fi

rm -rf "$NAME"
sed -i -e "s/,[[:space:]]*\"$NAME\"//" -e 's/[[:space:]]*,[[:space:]]*\]/]/' Cargo.toml
