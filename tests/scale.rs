//! Size probes: reading, writing and dropping deep and wide structures in bounded,
//! linear memory. Each probe runs in its own process under an address-space cap
//! and a timeout; the parent reads the child's peak resident size.

use std::process::Command;

use protos::{Bare, Enclosure, Head, Protoform, Protosizable, Situating, Textualizable};

const SIZES: [usize; 3] = [1_000, 10_000, 100_000];
const MODES: [&str; 6] = [
    "read-brackets",
    "read-chain",
    "read-chain-enclosed",
    "read-vector",
    "write-brackets",
    "write-vector",
];

fn peak_kb() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let line = status.lines().find(|l| l.starts_with("VmHWM:")).unwrap();
    line.split_whitespace().nth(1).unwrap().parse().unwrap()
}

fn nested(n: usize) -> Protoform {
    let mut form = Protoform::Enclosed(Enclosure::Bracketed, vec![]);
    for _ in 0..n {
        form = Protoform::Enclosed(Enclosure::Bracketed, vec![form]);
    }
    form
}

fn probe(mode: &str, n: usize) {
    match mode {
        "read-brackets" => {
            let text = format!("{}{}", "[".repeat(n), "]".repeat(n));
            let d = text.protosize().unwrap();
            assert_eq!(d.0.len(), 1);
            drop(d);
        }
        "read-chain" => {
            let text = format!("{}A", "A.".repeat(n));
            let d = text.protosize().unwrap();
            assert_eq!(d.0.len(), 1);
            drop(d);
        }
        "read-chain-enclosed" => {
            let text = format!("{}{{}}", "A.".repeat(n));
            let d = text.protosize().unwrap();
            assert_eq!(d.0.len(), 1);
            drop(d);
        }
        "read-vector" => {
            let text = format!("[ {}]", "1 ".repeat(n));
            let d = text.protosize().unwrap();
            assert!(matches!(&d.0[0].1, Protoform::Enclosed(_, c) if c.len() == n));
            drop(d);
        }
        "write-brackets" => {
            let form = nested(n);
            let text = form.textualize();
            assert_eq!(text.len(), 4 * n + 2);
            let situated = form.situate();
            assert_eq!(situated.1, text);
            drop(situated);
            drop(form);
        }
        "write-vector" => {
            let mut children = Vec::with_capacity(n);
            for _ in 0..n {
                children.push(Protoform::Bare(Head::Bare(Bare::try_from("1").unwrap())));
            }
            let form = Protoform::Enclosed(Enclosure::Bracketed, children);
            let text = form.textualize();
            assert_eq!(text.len(), 2 * n + 3);
            drop(form.situate());
            drop(form);
        }
        other => panic!("unknown probe {other}"),
    }
    println!("peak-kb {}", peak_kb());
}

fn run_child(mode: &str, n: usize) -> u64 {
    let exe = std::env::current_exe().unwrap();
    let output = Command::new("sh")
        .arg("-c")
        .arg("ulimit -v 2000000; exec timeout 120 \"$0\" \"$@\"")
        .arg(exe)
        .arg(mode)
        .arg(n.to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{mode} {n} failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let line = stdout.lines().find(|l| l.starts_with("peak-kb ")).unwrap();
    line[8..].trim().parse().unwrap()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 {
        probe(&args[1], args[2].parse().unwrap());
        return;
    }
    let mut failures = 0;
    for mode in MODES {
        let peaks: Vec<u64> = SIZES.iter().map(|&n| run_child(mode, n)).collect();
        let (small, medium, large) = (peaks[0], peaks[1], peaks[2]);
        let step = medium.saturating_sub(small);
        let linear_bound = small + 15 * step + 16 * 1024;
        let ok = large <= linear_bound && large < 1024 * 1024;
        println!(
            "{} {mode}: 1000 -> {small} kB, 10000 -> {medium} kB, 100000 -> {large} kB (bound {linear_bound} kB)",
            if ok { "ok  " } else { "FAIL" }
        );
        if !ok {
            failures += 1;
        }
    }
    assert_eq!(failures, 0, "{failures} probes exceeded linear memory");
}
