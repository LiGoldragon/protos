//! Size probes: reading, building, printing, cloning and dropping wide and
//! deep structures in bounded, linear memory.
//!
//! Each probe runs in its own process under an address-space cap and a
//! timeout, and the parent reads the child's peak resident size. Growth is
//! called linear when the peak at the largest size stays within the slope the
//! two smaller sizes set. The reader bounds its own descent, so every deep
//! probe here builds its structure rather than reading one.

use std::process::Command;

use protos::{
    BoundedProtosizable, Canonicalizable, Enclosure, Extent, Protos, ReaderBudget, Separator,
    Symbol, Textualizable,
};

const SIZES: [usize; 3] = [1_000, 10_000, 100_000];
const MODES: [&str; 6] = [
    "read-vector",
    "read-wide-nest",
    "build-nested",
    "build-vector",
    "clone-deep",
    "canonicalize-deep",
];
const NOWHERE: Extent = Extent { start: 0, end: 0 };

fn peak_kb() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("this process's status");
    let line = status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .expect("a high-water mark");
    line.split_whitespace()
        .nth(1)
        .expect("the mark itself")
        .parse()
        .expect("a number of kilobytes")
}

fn generous(n: usize) -> ReaderBudget {
    ReaderBudget {
        remaining: 4 * n + 16,
    }
}

fn nested(n: usize) -> Protos {
    let mut form = Protos::Enclosed {
        extent: NOWHERE,
        enclosure: Enclosure::Bracketed,
        children: vec![],
    };
    for _ in 0..n {
        form = Protos::Enclosed {
            extent: NOWHERE,
            enclosure: Enclosure::Bracketed,
            children: vec![form],
        };
    }
    form
}

fn deep_chain(n: usize) -> Protos {
    let mut form = Protos::Bare {
        extent: NOWHERE,
        text: String::from("x"),
    };
    for _ in 0..n {
        form = Protos::Headed {
            extent: NOWHERE,
            head: Symbol(String::from("V")),
            constraints: None,
            separator: Separator::Period,
            body: Box::new(form),
        };
    }
    form
}

fn probe(mode: &str, n: usize) {
    match mode {
        "read-vector" => {
            let text = format!("[ {}]", "1 ".repeat(n));
            let form = text
                .protosize_with(&mut generous(n))
                .expect("a wide vector");
            assert!(matches!(&form, Protos::Enclosed { children, .. } if children.len() == n));
        }
        "read-wide-nest" => {
            // Wide at two levels: the reader's depth bound forbids a deep one.
            let text = format!("[ {}]", "{ 1 2 } ".repeat(n));
            let form = text.protosize_with(&mut generous(n)).expect("a wide nest");
            assert!(matches!(&form, Protos::Enclosed { children, .. } if children.len() == n));
        }
        "build-nested" => {
            let mut form = nested(n);
            form.canonicalize();
            let text = form.textualize();
            assert_eq!(text.len(), 4 * n + 2);
        }
        "build-vector" => {
            let children = (0..n)
                .map(|_| Protos::Bare {
                    extent: NOWHERE,
                    text: String::from("1"),
                })
                .collect();
            let mut form = Protos::Enclosed {
                extent: NOWHERE,
                enclosure: Enclosure::Bracketed,
                children,
            };
            form.canonicalize();
            assert_eq!(form.textualize().len(), 2 * n + 3);
        }
        "clone-deep" => {
            let form = deep_chain(n);
            let copied = form.clone();
            assert_eq!(form, copied);
        }
        "canonicalize-deep" => {
            let mut form = deep_chain(n);
            form.canonicalize();
            assert_eq!(form.textualize().len(), 2 * n + 1);
        }
        other => panic!("unknown probe {other}"),
    }
    println!("peak-kb {}", peak_kb());
}

fn run_child(mode: &str, n: usize) -> u64 {
    let executable = std::env::current_exe().expect("this test executable");
    let output = Command::new("sh")
        .arg("-c")
        .arg("ulimit -v 2000000; exec timeout 120 \"$0\" \"$@\"")
        .arg(executable)
        .arg(mode)
        .arg(n.to_string())
        .output()
        .expect("a bounded child process");
    assert!(
        output.status.success(),
        "{mode} {n} failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("the child's report");
    let line = stdout
        .lines()
        .find(|line| line.starts_with("peak-kb "))
        .unwrap_or_else(|| panic!("{mode} {n} reported no peak:\n{stdout}"));
    line["peak-kb ".len()..]
        .trim()
        .parse()
        .expect("a number of kilobytes")
}

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() == 3 {
        probe(&arguments[1], arguments[2].parse().expect("a size"));
        return;
    }
    let mut exceeded = 0;
    for mode in MODES {
        let peaks: Vec<u64> = SIZES.iter().map(|&n| run_child(mode, n)).collect();
        let (small, medium, large) = (peaks[0], peaks[1], peaks[2]);
        let step = medium.saturating_sub(small);
        let bound = small + 15 * step + 16 * 1024;
        let within = large <= bound && large < 1024 * 1024;
        println!(
            "{} {mode}: 1000 -> {small} kB, 10000 -> {medium} kB, 100000 -> {large} kB (bound {bound} kB)",
            if within { "ok  " } else { "FAIL" }
        );
        if !within {
            exceeded += 1;
        }
    }
    assert_eq!(exceeded, 0, "{exceeded} probes exceeded linear memory");
}
