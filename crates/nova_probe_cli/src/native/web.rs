//! The web platform pass: trunk-build the wasm app, serve it, drive
//! headless Chromium at it, and scrape the summary line.

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use crate::run_report::{run_identity, PassRecord};

/// The web/WebGPU capture (ported from perf-web.sh): trunk-build the
/// perf_web wasm app, serve it from an embedded static server, drive a
/// headless-display Chromium at the perf URL with the EXACT flag set
/// the v0.7.0 baseline work calibrated (WebGPU needs the GPU process +
/// Vulkan + a non-blocklisted adapter), scrape the `nova perf:` summary
/// line from the console log, and write it as a frametime.csv row. The
/// positional argument is the SCENARIO id on this platform.
pub(crate) fn web_capture(
    root: &Path,
    out: &Path,
    display: &str,
    scenario: &str,
    timeout: Duration,
) -> Result<PassRecord, String> {
    let quality = std::env::var("QUALITY").unwrap_or_else(|_| "high".into());
    let frames = std::env::var("FRAMES").unwrap_or_else(|_| "600".into());
    let warmup = std::env::var("WARMUP").unwrap_or_else(|_| "180".into());
    let label = format!("{scenario}-{quality}-web");

    // Build the wasm bundle (trunk only supports the release profile).
    let dist = root.join("target/probe-dist");
    eprintln!(
        "probe: trunk build (release) perf.html -> {}",
        dist.display()
    );
    let status = Command::new("trunk")
        .current_dir(root)
        .args(["build", "--release", "-d"])
        .arg(&dist)
        .arg("perf.html")
        .status()
        .map_err(|e| format!("could not run trunk (is it installed?): {e}"))?;
    if !status.success() {
        return Err("trunk build failed".into());
    }

    let port = serve_dir(dist.clone())?;
    let url = format!(
        "http://127.0.0.1:{port}/?perf=1&scenario={scenario}&quality={quality}\
         &frames={frames}&warmup={warmup}&label={label}"
    )
    .replace(' ', "");
    eprintln!("probe: chromium -> {url}");

    // Chromium with the calibrated WebGPU flags (verbatim from
    // perf-web.sh; probed "ADAPTER OK 21 features" on this rig).
    let log_path = out.join("web-run.log");
    let log = std::fs::File::create(&log_path)
        .map_err(|e| format!("could not create {}: {e}", log_path.display()))?;
    let err_log = log.try_clone().map_err(|e| e.to_string())?;
    let mut chromium = Command::new("chromium")
        .args([
            "--no-sandbox",
            "--disable-gpu-sandbox",
            "--ignore-gpu-blocklist",
            "--enable-unsafe-webgpu",
            "--enable-features=Vulkan,WebGPU",
            "--use-angle=vulkan",
            "--enable-logging=stderr",
            "--v=1",
            "--window-size=1280,720",
        ])
        .arg(&url)
        .env("DISPLAY", display)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err_log))
        .spawn()
        .map_err(|e| format!("could not run chromium (is it installed?): {e}"))?;

    // Poll the console log for the summary line; kill by recorded PID
    // either way.
    let needle = format!("nova perf: label={label} frames");
    let start = Instant::now();
    let mut found = false;
    while start.elapsed() < timeout {
        if std::fs::read_to_string(&log_path)
            .map(|log| log.contains(&needle))
            .unwrap_or(false)
        {
            found = true;
            break;
        }
        if let Ok(Some(_)) = chromium.try_wait() {
            break; // chromium exited early; the log has the story
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    let timed_out = !found && start.elapsed() >= timeout;
    let _ = chromium.kill();
    let _ = chromium.wait();

    let mut scraped = false;
    if found {
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        let parsed = log
            .lines()
            .find(|l| l.contains(&needle))
            .and_then(nova_probe::parse_summary_line);
        let Some((parsed_label, stats)) = parsed else {
            // Degrade, never abort: the chromium log holds the line for
            // forensics and the report will show the failed pass.
            eprintln!(
                "probe: summary line found but not parseable (see {}); \
                 the report will show the failed capture",
                log_path.display()
            );
            return Ok(PassRecord {
                name: "web".into(),
                success: false,
                timed_out: false,
            });
        };
        scraped = true;
        let adapter = log
            .lines()
            .find(|l| l.contains("AdapterInfo {"))
            .and_then(|l| l.split("name: \"").nth(1))
            .and_then(|l| l.split('\"').next())
            .unwrap_or("unknown")
            .to_string();
        let (git_sha, host) = run_identity();
        let meta = nova_probe::RunMeta {
            backend: "webgpu".into(),
            adapter,
            resolution: "1280x720".into(),
            quality: quality.clone(),
            git_sha,
            host,
            // The web capture is ALWAYS a trunk --release build (dev
            // wasm is unusably slow and trunk has no custom profiles).
            profile: "release".into(),
        };
        nova_probe::append_frametime_row(&out.join("frametime.csv"), &parsed_label, &stats, &meta)?;
        eprintln!(
            "probe: web capture scraped -> {}",
            out.join("frametime.csv").display()
        );
    } else {
        eprintln!(
            "probe: no summary line captured (see {}); the report will show it",
            log_path.display()
        );
    }
    Ok(PassRecord {
        name: "web".into(),
        success: scraped,
        timed_out,
    })
}

/// Serve `dir` statically on an ephemeral 127.0.0.1 port from a daemon
/// thread (dies with the process). Minimal GET-only server - enough for
/// trunk's dist output; `.wasm` gets its real content type so streaming
/// instantiation works.
fn serve_dir(dir: PathBuf) -> Result<u16, String> {
    use std::io::{BufRead, BufReader, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("could not bind the static server: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let dir = dir.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(match stream.try_clone() {
                    Ok(s) => s,
                    Err(_) => return,
                });
                let mut request_line = String::new();
                if reader.read_line(&mut request_line).is_err() {
                    return;
                }
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok() && line.trim() != "" {
                    line.clear();
                }
                let path = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .split('?')
                    .next()
                    .unwrap_or("/");
                let rel = path.trim_start_matches('/');
                let file = if rel.is_empty() {
                    dir.join("index.html")
                } else {
                    dir.join(rel)
                };
                // No traversal outside the dist dir.
                let safe = file
                    .canonicalize()
                    .ok()
                    .filter(|f| f.starts_with(&dir))
                    .filter(|f| f.is_file());
                match safe.and_then(|f| std::fs::read(&f).ok().map(|b| (f, b))) {
                    Some((f, body)) => {
                        let ctype = match f.extension().and_then(|e| e.to_str()) {
                            Some("html") => "text/html",
                            Some("js") => "application/javascript",
                            Some("wasm") => "application/wasm",
                            Some("css") => "text/css",
                            Some("png") => "image/png",
                            Some("wav") => "audio/wav",
                            Some("ron") | Some("json") => "text/plain",
                            _ => "application/octet-stream",
                        };
                        let _ = write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\n\
                             Content-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(&body);
                    }
                    None => {
                        let _ = write!(
                            stream,
                            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\
                             Connection: close\r\n\r\n"
                        );
                    }
                }
            });
        }
    });
    Ok(port)
}
