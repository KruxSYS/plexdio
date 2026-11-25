//! `mockplex [--port N] [--bind ADDR] [--advertise HOST] [--synthetic N] [--no-sonic]
//!           [--manual-approve] [--quiet]`
//!
//! Then start Plexdio with `--plex-tv-url http://HOST:PORT`.
//!
//! `mockplex --releases DIR [--port N] [--bind ADDR]` instead serves a releases folder like
//! plexdio.kruxsys.com does (update manifests and downloads; see `releases.rs`), for
//! `plexdio --update-url http://HOST:PORT`.

use mockplex::{MockConfig, MockServer};

fn main() {
    let mut cfg = MockConfig { port: 32500, verbose: true, ..Default::default() };
    let mut releases: Option<std::path::PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" => cfg.port = args.next().and_then(|v| v.parse().ok()).expect("--port N"),
            "--bind" => cfg.bind = args.next().expect("--bind ADDR"),
            "--advertise" => cfg.advertise_host = args.next().expect("--advertise HOST"),
            "--synthetic" => cfg.synthetic_tracks = args.next().and_then(|v| v.parse().ok()),
            "--no-sonic" => cfg.sonic = false,
            "--manual-approve" => cfg.auto_approve_after = None,
            "--quiet" => cfg.verbose = false,
            "--releases" => releases = Some(args.next().expect("--releases DIR").into()),
            "-h" | "--help" => {
                println!("mockplex [--port N] [--bind ADDR] [--advertise HOST] [--synthetic N] [--no-sonic] [--manual-approve] [--quiet] | --releases DIR [--port N] [--bind ADDR]");
                return;
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }
    if let Some(dir) = releases {
        let bind = if cfg.bind.is_empty() { "127.0.0.1".to_string() } else { cfg.bind.clone() };
        let url = mockplex::releases::serve(dir.clone(), &bind, cfg.port, cfg.verbose).expect("failed to start the release server");
        println!("mockplex release server for {} on {url}", dir.display());
        loop {
            std::thread::park();
        }
    }
    let server = MockServer::start(cfg).expect("failed to start mock server");
    println!("mockplex listening on {}  (user token: {}, server token: {})", server.url, mockplex::USER_TOKEN, mockplex::SERVER_TOKEN);
    loop {
        std::thread::park();
    }
}
