//! Validation-only aggregation and formatting, independent of either Uop/tracer.
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use core::fmt::Debug;

#[derive(Default)]
pub(crate) struct Counters {
    counts: BTreeMap<u32, (u64, u64)>,
    pub(crate) faults: u64,
    pub(crate) retries: u64,
    pub(crate) string_elements: u64,
    pub(crate) rep_elements: u64,
    dispatches: u64,
}

impl Counters {
    pub(crate) fn record(&mut self, entry: Option<(u32, bool)>) {
        self.dispatches += 1;
        if let Some((eip, is_start)) = entry {
            let row = self.counts.entry(eip).or_default();
            row.0 += u64::from(is_start);
            row.1 += 1;
        }
    }

    pub(crate) fn tsv<'a, U: Debug + 'a>(
        &self,
        rows: impl Iterator<Item = (u32, &'a str, &'a [U])>,
    ) -> String {
        let mut out = format!(
            "# dispatches={} faults={} retries={} string_elements={} rep_elements={}\neip\tentries\tdispatches\tir\tuops\n",
            self.dispatches, self.faults, self.retries, self.string_elements, self.rep_elements
        );
        for (eip, ir, uops) in rows {
            if let Some((entries, dispatches)) = self.counts.get(&eip) {
                out.push_str(&format!("{eip}\t{entries}\t{dispatches}\t{ir}\t{uops:?}\n"));
            }
        }
        out
    }
}
