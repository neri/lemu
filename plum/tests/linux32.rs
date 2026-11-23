//! Example-host contracts, separate from the four CPU suites. No compiler needed.
#[path = "../examples/support/linux32.rs"]
mod linux32;

fn image() -> Vec<u8> {
    // A single static ELF32 segment at 08048000h with code at +100h.
    let mut bytes = vec![0; 512];
    bytes[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    for (offset, value) in [(16, 2u16), (18, 3), (40, 52), (42, 32), (44, 1)] {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    for (offset, value) in [
        (20, 1u32),
        (24, 0x8048100),
        (28, 52),
        (52, 1),
        (60, 0x8048000),
        (68, 512),
        (72, 1024),
        (76, 7),
        (80, 4096),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    // read(0, 08048180h, 3); write(1, same, bytes_read); unknown syscall;
    // exit(errno). Extra stdin bytes must not be echoed.
    let code = [
        0xb8, 3, 0, 0, 0, 0x31, 0xdb, 0xb9, 0x80, 0x81, 4, 8, 0xba, 3, 0, 0, 0, 0xcd, 0x80, 0x89,
        0xc2, 0xb8, 4, 0, 0, 0, 0xbb, 1, 0, 0, 0, 0xcd, 0x80, 0xb8, 0xff, 0xff, 0, 0, 0xcd, 0x80,
        0x89, 0xc3, 0xb8, 1, 0, 0, 0, 0xcd, 0x80,
    ];
    bytes[256..256 + code.len()].copy_from_slice(&code);
    bytes
}

#[test]
fn l01_loader_rejects_incomplete_header_dynamic_image_and_address_overflow() {
    let valid = image();
    assert!(linux32::run(&valid[..51], b"").is_err());
    let mut dynamic = valid.clone();
    dynamic[44..46].copy_from_slice(&2u16.to_le_bytes());
    dynamic[84..88].copy_from_slice(&3u32.to_le_bytes()); // PT_INTERP after valid PT_LOAD
    assert!(linux32::run(&dynamic, b"").is_err());
    let mut overflow = valid;
    overflow[60..64].copy_from_slice(&0xffffff00u32.to_le_bytes());
    assert_eq!(
        linux32::run(&overflow, b"").err().as_deref(),
        Some("invalid PT_LOAD size or address overflow")
    );
}

#[test]
fn l02_guest_read_write_and_unsupported_syscall_use_linux_host_contract() {
    let report = linux32::run(&image(), b"ABCextra").unwrap();
    assert_eq!(report.stdout, b"ABC");
    assert!(report.stderr.is_empty());
    assert_eq!(report.exit_code, 218); // -ENOSYS (-38), truncated by exit to u8
    assert_eq!(
        report.syscalls.into_iter().collect::<Vec<_>>(),
        [(1, 1), (3, 1), (4, 1), (65535, 1)]
    );
    assert!(report.cached_uops > 0);
}
