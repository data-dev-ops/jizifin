//! Union-find partition of members who share at least one bucket.

use super::BucketRow;
use jizifin_domain::Cents;
use jizifin_domain::ids::MemberId;
use std::collections::BTreeMap;

/// Splits members with a non-zero net into disjoint interaction groups.
#[derive(Default)]
pub(super) struct ComponentPartitioner {
    parent: BTreeMap<MemberId, MemberId>,
}

impl ComponentPartitioner {
    /// Groups members with non-zero `net` (or tied by an applied offset) that co-occur in any row.
    pub fn partition(
        mut self,
        rows: &[BucketRow],
        net: &BTreeMap<MemberId, Cents>,
        applied: &[(MemberId, MemberId)],
    ) -> Vec<Vec<MemberId>> {
        net.iter().filter(|(_, c)| !c.is_zero()).for_each(|(m, _)| self.insert(*m));
        applied.iter().for_each(|(a, b)| {
            self.insert(*a);
            self.insert(*b);
        });
        rows.iter().for_each(|row| self.join_row(row));
        applied.iter().for_each(|(a, b)| self.union(*a, *b));
        self.groups()
    }

    fn insert(&mut self, member: MemberId) {
        self.parent.entry(member).or_insert(member);
    }

    fn join_row(&mut self, row: &BucketRow) {
        let members: Vec<MemberId> =
            row.paid.keys().chain(row.expected.keys()).copied().filter(|m| self.parent.contains_key(m)).collect();
        members.windows(2).for_each(|pair| self.union(pair[0], pair[1]));
    }

    fn find(&mut self, member: MemberId) -> MemberId {
        let parent = self.parent[&member];
        if parent == member {
            return member;
        }
        let root = self.find(parent);
        self.parent.insert(member, root);
        root
    }

    fn union(&mut self, a: MemberId, b: MemberId) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent.insert(ra.max(rb), ra.min(rb));
        }
    }

    fn groups(mut self) -> Vec<Vec<MemberId>> {
        let members: Vec<MemberId> = self.parent.keys().copied().collect();
        let mut groups: BTreeMap<MemberId, Vec<MemberId>> = BTreeMap::new();
        for member in members {
            let root = self.find(member);
            groups.entry(root).or_default().push(member);
        }
        groups.into_values().collect()
    }
}
