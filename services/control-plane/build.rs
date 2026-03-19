use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let frontend_root = manifest_dir.join("../../frontend");
    let dist_dir = frontend_root.join("dist");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir")).join("personal_frontend");

    println!("cargo:rerun-if-changed={}", frontend_root.display());

    if out_dir.exists() {
        fs::remove_dir_all(&out_dir).expect("remove previous embedded frontend directory");
    }
    fs::create_dir_all(&out_dir).expect("create embedded frontend directory");

    if frontend_root.is_dir() && !dist_dir.is_dir() {
        if let Err(error) = run_frontend_build(&frontend_root) {
            println!("cargo:warning=failed to build frontend bundle automatically: {error}");
        }
    }

    if dist_dir.is_dir() {
        copy_dir_all(&dist_dir, &out_dir).expect("copy frontend dist into build output");
    } else {
        write_placeholder_frontend(&out_dir).expect("write placeholder personal frontend");
        println!("cargo:warning=frontend/dist not found; embedding placeholder personal frontend");
    }
}

fn run_frontend_build(frontend_root: &Path) -> io::Result<()> {
    if !frontend_root.join("node_modules").is_dir() {
        run_npm(frontend_root, &["ci", "--legacy-peer-deps"])?;
    }

    run_npm(frontend_root, &["run", "build"])
}

fn run_npm(frontend_root: &Path, args: &[&str]) -> io::Result<()> {
    let status = Command::new("npm")
        .args(args)
        .current_dir(frontend_root)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "npm {} exited with status {status}",
            args.join(" ")
        )))
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn write_placeholder_frontend(out_dir: &Path) -> io::Result<()> {
    fs::write(
        out_dir.join("index.html"),
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Codex Pool Personal</title>
    <style>
      :root {
        color-scheme: light;
        font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        background: #f4efe6;
        color: #182026;
      }
      body {
        margin: 0;
        min-height: 100vh;
        display: grid;
        place-items: center;
        background:
          radial-gradient(circle at top, rgba(255,255,255,0.9), rgba(244,239,230,0.95)),
          linear-gradient(135deg, #f4efe6, #e7dcc8);
      }
      main {
        width: min(640px, calc(100vw - 48px));
        padding: 32px;
        border-radius: 24px;
        background: rgba(255,255,255,0.9);
        box-shadow: 0 24px 80px rgba(24,32,38,0.14);
      }
      h1 {
        margin: 0 0 12px;
        font-size: clamp(32px, 5vw, 44px);
      }
      p {
        margin: 0;
        line-height: 1.6;
      }
      code {
        padding: 0.15em 0.4em;
        border-radius: 999px;
        background: rgba(24,32,38,0.08);
      }
    </style>
  </head>
  <body>
    <main>
      <h1>Codex Pool Personal</h1>
      <p>The frontend bundle has not been built yet. Run <code>cd frontend && npm run build</code>, then rebuild the personal binary to embed the full UI.</p>
    </main>
  </body>
</html>
"#,
    )
}
