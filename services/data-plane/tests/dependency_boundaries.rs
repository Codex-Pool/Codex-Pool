use std::process::Command;

fn cargo_tree_output(args: &[&str]) -> String {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run cargo tree");

    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn default_dependency_tree_does_not_include_redis_backend() {
    let stdout = cargo_tree_output(&[
        "tree",
        "-p",
        "data-plane",
        "--no-default-features",
        "-e",
        "normal",
        "-f",
        "{p}",
    ]);

    assert!(
        !stdout.contains("redis v"),
        "default data-plane dependency tree still includes redis:\n{stdout}"
    );
}

#[test]
fn redis_feature_dependency_tree_includes_redis_backend() {
    let stdout = cargo_tree_output(&[
        "tree",
        "-p",
        "data-plane",
        "--no-default-features",
        "--features",
        "redis-backend",
        "-e",
        "normal",
        "-f",
        "{p}",
    ]);

    assert!(
        stdout.contains("redis v"),
        "redis-backend feature should include redis dependency:\n{stdout}"
    );
}
