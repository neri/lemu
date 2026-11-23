//! Read the public TSV output; expectations remain in the engine tests.
use std::collections::BTreeMap;

pub struct Profile {
    counters: BTreeMap<String, u64>,
    pub rows: BTreeMap<u32, (u64, u64)>,
}

impl Profile {
    pub fn counter(&self, name: &str) -> u64 {
        *self.counters.get(name).expect("missing profile counter")
    }

    pub fn entries(&self) -> BTreeMap<u32, u64> {
        self.rows
            .iter()
            .map(|(&eip, &(entries, _))| (eip, entries))
            .collect()
    }
}

pub fn read(tsv: &str) -> Profile {
    let mut lines = tsv.lines();
    let header = lines.next().unwrap().strip_prefix("# ").unwrap();
    let mut counters = BTreeMap::new();
    for field in header.split_whitespace() {
        let (name, value) = field.split_once('=').unwrap();
        assert!(
            counters
                .insert(name.to_owned(), value.parse().unwrap())
                .is_none()
        );
    }
    assert_eq!(lines.next(), Some("eip\tentries\tdispatches\tir\tuops"));
    let mut rows = BTreeMap::new();
    for line in lines {
        let fields: Vec<_> = line.splitn(5, '\t').collect();
        assert_eq!(fields.len(), 5, "{line}");
        let row = (fields[1].parse().unwrap(), fields[2].parse().unwrap());
        assert!(
            rows.insert(fields[0].parse().unwrap(), row).is_none(),
            "duplicate EIP: {line}"
        );
    }
    Profile { counters, rows }
}
