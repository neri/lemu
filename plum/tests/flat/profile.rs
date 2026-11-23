//! Engine-specific profile contracts; shared TSV parsing has no expectations.
use super::*;

#[test]
fn f07_profile_keeps_cumulative_rows_and_latest_ir_after_repeated_code_updates() {
    let mut cpu = machine(&[0xb8, 1, 0, 0, 0, 0xcd, 0x40, 0xeb, 0xf7]);
    cpu.write_bytes(BASE + 64, &[0xcd, 0x40]).unwrap();
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 1);
    // Replace MOV EAX,1 by MOV ECX,2 at the same guest address.
    cpu.write_bytes(BASE, &[0xb9, 2, 0, 0, 0]).unwrap();
    cpu.resume_next();
    done(&mut cpu);
    assert_eq!(cpu.state().ecx().read(), 2);
    let profile = profile_output::read(&cpu.execution_profile_tsv());
    assert_eq!(
        profile.entries(),
        [(BASE, 2), (BASE + 5, 2), (BASE + 7, 1)]
            .into_iter()
            .collect()
    );

    // Invalidate again, then stop elsewhere so BASE is absent from the new cache.
    cpu.write_bytes(BASE + 8, &[55]).unwrap(); // JMP from BASE+7 to BASE+64
    cpu.resume_next();
    done(&mut cpu);
    let tsv = cpu.execution_profile_tsv();
    let profile = profile_output::read(&tsv);
    assert_eq!(
        profile.entries(),
        [(BASE, 2), (BASE + 5, 2), (BASE + 7, 2), (BASE + 64, 1)]
            .into_iter()
            .collect()
    );
    assert_eq!(
        (profile.counter("faults"), profile.counter("retries")),
        (0, 0)
    );
    let row = tsv
        .lines()
        .skip(2)
        .find(|line| line.starts_with(&format!("{BASE}\t")))
        .unwrap();
    assert_eq!(row.split('\t').nth(3), Some("MOV_Rd_Id(ECX, 2)"));
}

#[test]
fn f07_profile_attributes_cached_loop_attempts_to_guest_eips() {
    // MOV ECX,3; INC EBX; DEC ECX; JNZ back to INC; INT 40h.
    let mut cpu = machine(&[0xb9, 3, 0, 0, 0, 0x43, 0x49, 0x75, 0xfc, 0xcd, 0x40]);
    done(&mut cpu);
    assert_eq!((cpu.state().ebx().read(), cpu.state().ecx().read()), (3, 0));
    let profile = profile_output::read(&cpu.execution_profile_tsv());
    assert_eq!(
        profile.entries(),
        [
            (BASE, 1),
            (BASE + 5, 3),
            (BASE + 6, 3),
            (BASE + 7, 3),
            (BASE + 9, 1)
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        (profile.counter("faults"), profile.counter("retries")),
        (0, 0)
    );
    for (&eip, &(entries, dispatches)) in &profile.rows {
        assert!(dispatches >= entries, "eip={eip:x}");
    }
    let attributed: u64 = profile.rows.values().map(|row| row.1).sum();
    assert!(profile.counter("dispatches") >= attributed);
}

#[test]
fn f07_profile_counts_fault_attempts_retries_and_skipped_faults_separately() {
    // DIV ECX; INT 40h; INC EBX; INT 40h.
    let mut cpu = machine(&[0xf7, 0xf1, 0xcd, 0x40, 0x43, 0xcd, 0x40]);
    cpu.state().eax().write(10);
    for (faults, retries) in [(1, 0), (2, 1)] {
        assert!(matches!(cpu.execute(), Err(Exception::DivisionError)));
        let profile = profile_output::read(&cpu.execution_profile_tsv());
        assert_eq!(profile.entries(), [(BASE, faults)].into_iter().collect());
        assert_eq!(
            (profile.counter("faults"), profile.counter("retries")),
            (faults, retries)
        );
    }
    cpu.state().ecx().write(2);
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 5);
    let profile = profile_output::read(&cpu.execution_profile_tsv());
    assert_eq!(
        profile.entries(),
        [(BASE, 3), (BASE + 2, 1)].into_iter().collect()
    );
    assert_eq!(
        (profile.counter("faults"), profile.counter("retries")),
        (2, 2)
    );
    cpu.resume_next();
    done(&mut cpu);
    let profile = profile_output::read(&cpu.execution_profile_tsv());
    assert_eq!(
        profile.entries(),
        [(BASE, 3), (BASE + 2, 1), (BASE + 4, 1), (BASE + 5, 1)]
            .into_iter()
            .collect()
    );
    assert_eq!(
        (profile.counter("faults"), profile.counter("retries")),
        (2, 2)
    );
    assert_eq!(cpu.state().ebx().read(), 1);

    let mut cpu = machine(&[0xf7, 0xf1]);
    assert!(matches!(cpu.execute(), Err(Exception::DivisionError)));
    cpu.resume_next(); // host explicitly skips the faulting DIV
    done(&mut cpu);
    let profile = profile_output::read(&cpu.execution_profile_tsv());
    assert_eq!(
        profile.entries(),
        [(BASE, 1), (BASE + 2, 1)].into_iter().collect()
    );
    assert_eq!(
        (profile.counter("faults"), profile.counter("retries")),
        (1, 0)
    );
}

#[test]
fn f07_profile_rep_counts_only_completed_elements_across_retry() {
    // REP MOVSB; MOVSB; REP MOVSB (ECX=0); INT 40h.
    let mut cpu = machine(&[0xf3, 0xa4, 0xa4, 0xf3, 0xa4, 0xcd, 0x40]);
    cpu.write_bytes(BASE + 510, &[1, 2]).unwrap();
    cpu.state().esi().write(BASE + 512 - 2);
    cpu.state().edi().write(BASE + 64);
    cpu.state().ecx().write(4);
    assert!(matches!(
        cpu.execute(),
        Err(Exception::SegmentationViolation(_))
    ));
    let profile = profile_output::read(&cpu.execution_profile_tsv());
    assert_eq!(profile.entries(), [(BASE, 1)].into_iter().collect());
    assert_eq!(
        (
            profile.counter("string_elements"),
            profile.counter("rep_elements")
        ),
        (2, 2)
    );
    assert_eq!(
        (profile.counter("faults"), profile.counter("retries")),
        (1, 0)
    );
    cpu.write_bytes(BASE + 128, &[9, 10, 11]).unwrap();
    cpu.state().esi().write(BASE + 128);
    done(&mut cpu);
    assert_eq!(&cpu.memory()[64..69], &[1, 2, 9, 10, 11]);
    let profile = profile_output::read(&cpu.execution_profile_tsv());
    assert_eq!(
        profile.entries(),
        [(BASE, 2), (BASE + 2, 1), (BASE + 3, 1), (BASE + 5, 1)]
            .into_iter()
            .collect()
    );
    assert_eq!(
        (
            profile.counter("string_elements"),
            profile.counter("rep_elements")
        ),
        (5, 4)
    );
    assert_eq!(
        (profile.counter("faults"), profile.counter("retries")),
        (1, 1)
    );
}
