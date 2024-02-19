#!/bin/bash

if [[ -z "$1" ]]; then
  echo "Need argument \$1 to be the name to add" >&2
  exit 1
fi

NAME="$(tr '[:upper:]' '[:lower:]' <<< "$1")"
if [[ -e "$NAME" ]]; then
  echo "$NAME exists" >&2
  exit 2
fi

SIMNAME="$(tr '[:lower:]' '[:upper:]' <<<"${NAME:0:1}")${NAME:1}"

cargo new "$NAME"
pushd "$NAME" >/dev/null || exit 3

cat >> Cargo.toml <<<'
aftgraphs = { path="../" }
aftgraphs-macros = { path="../aftgraphs-macros" }
winit = { version = "0.29.3", features=["rwh_05", "wayland", "x11"] }
wgpu = { version="0.18", features=["webgl", "spirv"] }

'"[target.'cfg(target_family = \"wasm\")'.dependencies]"'
js-sys = "0.3"
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [
  "WebGl2RenderingContext",
  "WebGlActiveInfo",
  "WebGlBuffer",
  "WebGlContextAttributes",
  "WebGlContextEvent",
  "WebGlContextEventInit",
  "WebGlFramebuffer",
  "WebGlPowerPreference",
  "WebGlProgram",
  "WebGlQuery",
  "WebGlRenderbuffer",
  "WebGlRenderingContext",
  "WebGlSampler",
  "WebGlShader",
  "WebGlShaderPrecisionFormat",
  "WebGlSync",
  "WebGlTexture",
  "WebGlTransformFeedback",
  "WebGlUniformLocation",
  "WebGlVertexArrayObject",
] }

[lib]  
crate-type = ["cdylib", "rlib"]
'

cat > src/main.rs <<<"
use $NAME::sim_main;

fn main() {
  sim_main();
}
"

cat > src/lib.rs <<<"
use aftgraphs::prelude::*;
use aftgraphs_macros::sim_main;
use std::collections::HashMap;

struct $SIMNAME {
  pipeline: RenderPipeline,
}

impl Simulation for $SIMNAME {
  fn new(renderer: &Renderer) -> Self {
    // CREATE INSTANCE HERE
  }

  async fn on_input(&mut self, _event: InputEvent) {
    // IMPLEMENT KEYBOARD/MOUSE INPUT HERE
  }

  async fn render(
      &mut self,
      renderer: &Renderer,
      render_pass: RenderPass<'_>,
      inputs: &mut HashMap<String, InputValue>,
  ) {
    // RENDER HERE
  }
}

sim_main! { \"/res/$NAME.toml\", $SIMNAME }
"

mkdir -p res/

cat > "res/$NAME.toml" <<<"
[simulation]
name = \"$NAME\"
"

popd >/dev/null || exit 3
