/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(unused_imports)]

use std::env;
use std::path::PathBuf;
use std::process::Command;

use bindgen;

fn main() {
    compile_metal();
}

#[cfg(not(feature = "osx-metal"))]
fn compile_metal() {}

///
/// Compiles a shader in the Metal shader language
///
#[cfg(feature = "osx-metal")]
fn compile_metal_shader(input_path: &str, output_path: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap().into_string().unwrap();

    println!("cargo:rerun-if-changed={}", input_path);

    let shader_compile_output = Command::new("xcrun")
        .args(&["-sdk", "macosx"])
        .arg("metal")
        .args(&["-I", "."])
        .args(&["-c", input_path])
        .args(&["-o", &format!("{}/{}", out_dir, output_path)])
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();

    if !shader_compile_output.status.success() {
        panic!(
            "{}\n\n{}",
            String::from_utf8_lossy(&shader_compile_output.stdout),
            String::from_utf8_lossy(&shader_compile_output.stderr)
        );
    }
}

///
/// Links some shaders compiled by compile_metal_shader
///
#[cfg(feature = "osx-metal")]
fn link_metal_shaders(input_paths: Vec<&str>, output_path: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap().into_string().unwrap();

    let shader_link_output = Command::new("xcrun")
        .args(&["-sdk", "macosx"])
        .arg("metallib")
        .args(
            input_paths
                .into_iter()
                .map(|path| format!("{}/{}", out_dir, path)),
        )
        .args(&["-o", &format!("{}/{}", out_dir, output_path)])
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();

    if !shader_link_output.status.success() {
        panic!(
            "{}\n\n{}",
            String::from_utf8_lossy(&shader_link_output.stdout),
            String::from_utf8_lossy(&shader_link_output.stderr)
        );
    }
}

#[cfg(feature = "osx-metal")]
fn compile_metal() {
    // Compile the shaders
    println!("cargo:rerun-if-changed=shaders");
    compile_metal_shader("shaders/simple/simple.metal", "simple.air");
    compile_metal_shader("shaders/simple/clip_mask.metal", "clip_mask.air");
    compile_metal_shader("shaders/simple/postprocessing.metal", "postprocessing.air");
    compile_metal_shader(
        "shaders/texture/gradient_fragment.metal",
        "gradient_fragment.air",
    );
    compile_metal_shader(
        "shaders/texture/texture_fragment.metal",
        "texture_fragment.air",
    );
    link_metal_shaders(
        vec![
            "simple.air",
            "texture_fragment.air",
            "gradient_fragment.air",
            "clip_mask.air",
            "postprocessing.air",
        ],
        "flo.metallib",
    );

    // Generate .rs files from the binding headers
    println!("cargo:rerun-if-changed=bindings");

    let bindings = match &env::var("CARGO_CFG_TARGET_ARCH").unwrap()[..] {
        "aarch64" | "arm64" => bindgen::Builder::default()
            .header("bindings/metal_vertex2d.h")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .clang_args(vec!["-arch", "arm64"])
            .generate(),

        _ => bindgen::Builder::default()
            .header("bindings/metal_vertex2d.h")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .generate(),
    }
    .expect("Unable to generate bindings");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out = out.join("metal_vertex2d.rs");

    bindings.write_to_file(out).unwrap();
}
