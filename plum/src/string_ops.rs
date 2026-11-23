//! Kernels over validated allocation offsets; no guest state or engine types.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompareRepeat {
    Once,
    WhileZero,
    WhileNotZero,
}

/// Count complete elements before a limit or index wrap. Division is a shift.
pub(crate) fn span(len: usize, addr: u32, shift: u32, reverse: bool, requested: u32) -> u32 {
    let start = addr as usize;
    let width = 1usize << shift;
    if !start.checked_add(width).is_some_and(|end| end <= len) {
        return 0;
    }
    let available = if reverse {
        (addr >> shift).saturating_add(1)
    } else {
        let memory = ((len - start) >> shift).min(u32::MAX as usize) as u32;
        let wrap = ((u32::MAX - addr) >> shift).saturating_add(1);
        memory.min(wrap)
    };
    available.min(requested)
}

/// `count` is positive and the complete span has already been validated.
pub(crate) fn range<const N: usize, const REVERSE: bool>(
    addr: u32,
    count: u32,
) -> core::ops::Range<usize> {
    let bytes = (count as usize)
        .checked_mul(N)
        .expect("validated span size");
    let start = if REVERSE {
        addr as usize - (bytes - N)
    } else {
        addr as usize
    };
    start..start.checked_add(bytes).expect("validated span end")
}

pub(crate) fn load<const N: usize>(bytes: &[u8]) -> u32 {
    match N {
        1 => bytes[0] as u32,
        2 => u16::from_le_bytes(bytes.try_into().unwrap()) as u32,
        4 => u32::from_le_bytes(bytes.try_into().unwrap()),
        _ => unreachable!(),
    }
}

/// Return the number of bytes visited before the first match in guest order.
fn find_byte<const REVERSE: bool>(bytes: &[u8], needle: u8) -> Option<usize> {
    let pattern = u64::from_ne_bytes([needle; 8]);
    let no_match = |chunk: &&[u8]| {
        let x = u64::from_ne_bytes((*chunk).try_into().unwrap()) ^ pattern;
        // A zero byte in x is a match. Subtracting one sets a high bit in
        // at least one zero lane; mask off lanes whose high bit was set.
        x.wrapping_sub(0x0101_0101_0101_0101) & !x & 0x8080_8080_8080_8080 == 0
    };
    if REVERSE {
        let skipped = bytes.rchunks_exact(8).take_while(no_match).count() * 8;
        bytes[..bytes.len() - skipped]
            .iter()
            .rev()
            .position(|&b| b == needle)
            .map(|i| skipped + i)
    } else {
        let skipped = bytes.chunks_exact(8).take_while(no_match).count() * 8;
        bytes[skipped..]
            .iter()
            .position(|&b| b == needle)
            .map(|i| skipped + i)
    }
}

/// Both slices contain the same positive number of complete elements.
/// Returns consumed count and the last subtraction operands.
pub(crate) fn compare<const N: usize, const REVERSE: bool>(
    source: Option<&[u8]>,
    dest: &[u8],
    accumulator: u32,
    repeat: CompareRepeat,
) -> (u32, u32, u32) {
    let count = dest.len() / N;
    let equal_stops = repeat == CompareRepeat::WhileNotZero;
    let mut skipped = 0;
    // Skip equal blocks in guest traversal order. Resolve the first mismatch
    // element by element so that the final subtraction uses the full operand.
    if repeat == CompareRepeat::WhileZero {
        if let Some(src) = source {
            let blocks = if REVERSE {
                src.rchunks_exact(64)
                    .zip(dest.rchunks_exact(64))
                    .take_while(|(a, b)| a == b)
                    .count()
            } else {
                src.chunks_exact(64)
                    .zip(dest.chunks_exact(64))
                    .take_while(|(a, b)| a == b)
                    .count()
            };
            skipped = blocks * (64 / N);
        }
    }
    // Select CMPS versus SCAS outside the element iterator.
    let found = match (source, REVERSE) {
        (None, _) if N == 1 && equal_stops => find_byte::<REVERSE>(dest, accumulator as u8),
        (Some(src), true) => src[..(count - skipped) * N]
            .chunks_exact(N)
            .rev()
            .zip(dest[..(count - skipped) * N].chunks_exact(N).rev())
            .position(|(a, b)| (load::<N>(a) == load::<N>(b)) == equal_stops)
            .map(|i| skipped + i),
        (None, true) => dest
            .chunks_exact(N)
            .rev()
            .position(|b| (accumulator == load::<N>(b)) == equal_stops),
        (Some(src), false) => src[skipped * N..]
            .chunks_exact(N)
            .zip(dest[skipped * N..].chunks_exact(N))
            .position(|(a, b)| (load::<N>(a) == load::<N>(b)) == equal_stops)
            .map(|i| skipped + i),
        (None, false) => dest
            .chunks_exact(N)
            .position(|b| (accumulator == load::<N>(b)) == equal_stops),
    };
    let stop = found.unwrap_or(count - 1);
    let index = if REVERSE { count - 1 - stop } else { stop };
    let a = source.map_or(accumulator, |s| load::<N>(&s[index * N..(index + 1) * N]));
    (
        stop as u32 + 1,
        a,
        load::<N>(&dest[index * N..(index + 1) * N]),
    )
}

/// Copy a positive validated span, preserving x86 element order on overlaps.
pub(crate) fn copy<const N: usize, const REVERSE: bool>(
    data: &mut [u8],
    src_range: core::ops::Range<usize>,
    dest_range: core::ops::Range<usize>,
    count: u32,
) {
    // A bulk move is equivalent when no store overwrites an unread element.
    let bulk = if REVERSE {
        dest_range.start >= src_range.start || dest_range.end <= src_range.start
    } else {
        dest_range.start <= src_range.start || dest_range.start >= src_range.end
    };
    if bulk {
        data.copy_within(src_range, dest_range.start);
    } else {
        // A whole element must be read before storing, including overlaps
        // with a displacement smaller than N. No per-element guest checks.
        for i in 0..count as usize {
            let offset = if REVERSE {
                (count as usize - 1 - i) * N
            } else {
                i * N
            };
            let value = load::<N>(&data[src_range.start + offset..src_range.start + offset + N]);
            data[dest_range.start + offset..dest_range.start + offset + N]
                .copy_from_slice(&value.to_le_bytes()[..N]);
        }
    }
}

/// Fill a positive validated span with the low N bytes of the accumulator.
pub(crate) fn fill<const N: usize>(dest: &mut [u8], accumulator: u32) {
    let pattern = accumulator.to_le_bytes();
    if N == 1 {
        dest.fill(pattern[0]);
    } else {
        dest[..N].copy_from_slice(&pattern[..N]);
        let mut filled = N;
        while filled < dest.len() {
            let next = filled.min(dest.len() - filled);
            dest.copy_within(..next, filled);
            filled += next;
        }
    }
}
