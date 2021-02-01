use std::process::Command;

fn main() {
    let output = String::from_utf8(
        Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .expect("Failed to execute git rev-parse HEAD")
            .stdout,
    )
    .unwrap();
    let hash = output.trim();

    let short_output = String::from_utf8(
        Command::new("git")
            .args(&["rev-parse", "--short", "HEAD"])
            .output()
            .expect("Failed to execute git rev-parse HEAD")
            .stdout,
    )
    .unwrap();

    let short_hash = short_output.trim();

    let timestamp_output = String::from_utf8(
        Command::new("git")
            .args(&["show", "-s", "--format=%ct", "HEAD"])
            .output()
            .expect("Failed to execute git show -s --format=%ct HEAD")
            .stdout,
    )
    .unwrap();

    let timestamp = timestamp_output.trim();

    println!("cargo:rustc-env=GIT_REF={}", hash);
    println!("cargo:rustc-env=GIT_SHORT_REF={}", short_hash);
    println!("cargo:rustc-env=GIT_TIMESTAMP={}", timestamp);
}
