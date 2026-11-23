//! Engine-specific guest-entry lookup and retry identity.
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::FlatUME;
use super::tracer::TraceDecoder;
use super::uop::Uop;
use crate::profile_support::Counters;

#[derive(Default)]
pub(super) struct ExecutionProfile {
    pub(super) counters: Counters,
    pub(super) retry_pending: Option<crate::prelude::Offset32>,
    // Counters outlive the execution cache. Keep one latest decoded image per
    // EIP, rather than emitting duplicate rows or retaining every code version.
    archived: BTreeMap<u32, (String, Vec<Uop>)>,
}

fn rows(tracer: &TraceDecoder) -> impl Iterator<Item = (u32, &str, &[Uop])> {
    let mut starts = tracer.profile_starts.iter().peekable();
    core::iter::from_fn(move || {
        let (&start, (eip, ir)) = starts.next()?;
        let end = starts
            .peek()
            .map_or(tracer.uop_cache().len(), |(next, _)| **next as usize);
        Some((*eip, ir.as_str(), &tracer.uop_cache()[start as usize..end]))
    })
}

impl ExecutionProfile {
    pub(super) fn archive(&mut self, tracer: &TraceDecoder) {
        for (eip, ir, uops) in rows(tracer) {
            self.archived.insert(eip, (String::from(ir), uops.to_vec()));
        }
    }

    pub(super) fn record_internal(&mut self) {
        self.counters.record(None);
    }

    pub(super) fn record(&mut self, tracer: &TraceDecoder) {
        let upc = tracer.current_upc().0;
        let entry = tracer
            .profile_starts
            .range(..=upc)
            .next_back()
            .map(|(&start, (eip, _))| (*eip, start == upc));
        self.counters.record(entry);
    }
}

impl FlatUME {
    /// Entries count attempts, not retired instructions across faults.
    /// Synthetic trace jumps count as dispatches, never as guest instructions.
    /// Counts survive cache invalidation; IR/Uops describe the latest decoded
    /// version at each EIP, including entries no longer in the execution cache.
    pub fn execution_profile_tsv(&self) -> String {
        let mut metadata: BTreeMap<_, _> = self
            .profile
            .archived
            .iter()
            .map(|(&eip, (ir, uops))| (eip, (ir.as_str(), uops.as_slice())))
            .collect();
        for (eip, ir, uops) in rows(&self.tracer) {
            metadata.insert(eip, (ir, uops));
        }
        self.profile.counters.tsv(
            metadata
                .into_iter()
                .map(|(eip, (ir, uops))| (eip, ir, uops)),
        )
    }
}
