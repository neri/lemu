//! Static Linux/i386 ELF runner on FlatUME. See linux32/README.md.
#[path = "support/linux32.rs"]
mod linux32;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::io::{Read, Write};
    let args: Vec<_> = std::env::args().skip(1).collect();
    let Some(path) = args.first() else {
        eprintln!("Usage: linux32 guest.elf [--stats] < input");
        std::process::exit(125);
    };
    let mut input = Vec::new();
    let outcome = std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|elf| {
            std::io::stdin()
                .read_to_end(&mut input)
                .map_err(|e| e.to_string())?;
            let start = std::time::Instant::now();
            let report = linux32::run(&elf, &input)?;
            std::io::stdout()
                .write_all(&report.stdout)
                .map_err(|e| e.to_string())?;
            std::io::stderr()
                .write_all(&report.stderr)
                .map_err(|e| e.to_string())?;
            if args.iter().any(|s| s == "--stats") {
                eprintln!(
                    "guest exit={} syscalls={:?} cached_uops={} elapsed={:?}",
                    report.exit_code,
                    report.syscalls,
                    report.cached_uops,
                    start.elapsed()
                );
            }
            Ok(report.exit_code)
        });
    match outcome {
        Ok(code) => std::process::exit(code as i32),
        Err(error) => {
            eprintln!("linux32: {error}");
            std::process::exit(125);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}

// The JS caller owns the input/output lifetime and calls these sequentially.
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::linux32;
    struct Session {
        bytes: Vec<u8>,
        elf_len: usize,
        report: Option<linux32::Report>,
        error: Vec<u8>,
    }
    static mut SESSION: *mut Session = core::ptr::null_mut();

    fn session() -> &'static mut Session {
        unsafe { SESSION.as_mut().expect("call linux32_prepare first") }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn linux32_prepare(elf_len: u32, input_len: u32) -> u32 {
        assert!(elf_len <= 1024 * 1024 && input_len <= 1024 * 1024);
        let mut next = Box::new(Session {
            bytes: vec![0; elf_len as usize + input_len as usize],
            elf_len: elf_len as usize,
            report: None,
            error: Vec::new(),
        });
        let address = next.bytes.as_mut_ptr() as u32;
        unsafe {
            if !SESSION.is_null() {
                drop(Box::from_raw(SESSION));
            }
            SESSION = Box::into_raw(next);
        }
        address
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn linux32_run() -> i32 {
        let s = session();
        s.report = None;
        s.error.clear();
        match linux32::run(&s.bytes[..s.elf_len], &s.bytes[s.elf_len..]) {
            Ok(report) => {
                let code = report.exit_code;
                s.report = Some(report);
                code as i32
            }
            Err(error) => {
                s.error = error.into_bytes();
                -1
            }
        }
    }

    fn output(stderr: bool) -> &'static [u8] {
        let s = session();
        if let Some(r) = &s.report {
            if stderr { &r.stderr } else { &r.stdout }
        } else if stderr {
            &s.error
        } else {
            &[]
        }
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn linux32_stdout_ptr() -> u32 {
        output(false).as_ptr() as u32
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn linux32_stdout_len() -> u32 {
        output(false).len() as u32
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn linux32_stderr_ptr() -> u32 {
        output(true).as_ptr() as u32
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn linux32_stderr_len() -> u32 {
        output(true).len() as u32
    }
}
