/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(not(target_os = "linux"))]
fn main() {
    // No build steps to take for non-linux OSes
}

#[cfg(target_os = "linux")]
fn main() {
    use std::env;
    use std::path::PathBuf;

    // Linux build: generate bindings for gbm
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out = out.join("gbm.rs");

    bindgen::Builder::default()
        .header("tiny_gbm.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings for gbm")
        .write_to_file(out)
        .expect("Could not write gbm.rs");
}
