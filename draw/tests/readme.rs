/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::str;

///
/// Reads the README file for the crate
///
fn readme() -> &'static str {
    let readme_bytes = include_bytes!("../README.md");
    let readme_str = str::from_utf8(readme_bytes);

    readme_str.expect("Could not decode README.md")
}

#[test]
fn starts_with_version_number_toml() {
    let major_version = env!("CARGO_PKG_VERSION_MAJOR");
    let minor_version = env!("CARGO_PKG_VERSION_MINOR");

    let expected = format!(
        "```toml
flo_draw = \"{}.{}\"
```",
        major_version, minor_version
    );

    println!("{}", expected);
    assert!(readme().starts_with(&expected));
}
