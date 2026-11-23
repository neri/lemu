//! Engine-specific guest-entry lookup and retry identity.
use alloc::string::String;

use super::HarvardUME;
use super::tracer::TraceDecoder;
use crate::profile_support::Counters;

#[derive(Default)]
pub(super) struct ExecutionProfile {
    pub(super) counters: Counters,
    pub(super) retry_pending: Option<super::uop::AddrIndex>,
}

impl ExecutionProfile {
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

impl HarvardUME {
    /// Entries count attempts, not retired instructions across faults.
    /// Synthetic trace jumps count as dispatches, never as guest instructions.
    pub fn execution_profile_tsv(&self) -> String {
        let starts: alloc::vec::Vec<_> = self.tracer.profile_starts.iter().collect();
        let rows = starts.iter().enumerate().map(|(i, entry)| {
            let (start, (eip, ir)) = *entry;
            let end = starts
                .get(i + 1)
                .map_or(self.tracer.uop_cache().len(), |(s, _)| **s as usize);
            (
                *eip,
                ir.as_str(),
                &self.tracer.uop_cache()[*start as usize..end],
            )
        });
        self.profile.counters.tsv(rows)
    }
}
