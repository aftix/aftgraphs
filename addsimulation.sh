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
wasm-bindgen = "0.2"
js-sys = "0.3"
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

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
struct $SIMNAME;

impl Simulation for $SIMNAME {
  fn new(renderer: &Renderer) -> Self {
    // CREATE INSTANCE HERE
  }

  async fn render(&mut self, renderer: Arc<Mutex<Renderer>>, out_img: Arc<Mutex<Vec<u8>>>) {
    let renderer = renderer.lock().await;
    // RENDER HERE
  }
}

sim_main! { \"/res/$NAME.toml\", SIMNAME }
"

popd >/dev/null || exit 3
