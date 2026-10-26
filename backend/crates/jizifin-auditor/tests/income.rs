use chrono::NaiveDate;
use jizifin_auditor::income::{IncomeResolver, SalaryNormalizer};
use jizifin_auditor::Snapshot;
use jizifin_domain::entities::{Entry, EntryKind, Frequency, IncomeOverride, Job, Member};
use jizifin_domain::ids::{EntryId, JobId, MemberId};
use jizifin_domain::{Cents, Ciphertext, Month};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

#[test]
fn normalizer_frequencies_and_rounding() {
    let norm = SalaryNormalizer;
    assert_eq!(norm.monthly(Cents::new(300000), Frequency::Monthly), Cents::new(300000));
    assert_eq!(norm.monthly(Cents::new(120000), Frequency::Annual), Cents::new(10000));
    assert_eq!(norm.monthly(Cents::new(1000), Frequency::Weekly), Cents::new(4333));
    assert_eq!(norm.monthly(Cents::new(2000), Frequency::Biweekly), Cents::new(4333));
}

fn setup_income_snap() -> Snapshot {
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    snap.members.push(Member { id: MemberId(2), name: ct("Bob"), color: "#222".into(), is_active: true });
    snap.jobs.push(Job {
        id: JobId(1), member: MemberId(1), amount: Cents::new(400000), frequency: Frequency::Monthly,
        valid_from: d(2026, 1, 1), valid_to: None, name: ct("Company"), notes: None,
    });
    snap.income_overrides.push(IncomeOverride {
        member: MemberId(1), month: Month::parse("2026-07").unwrap(), amount: Cents::new(500000), note: None,
    });
    snap.entries.push(Entry {
        id: EntryId(1), kind: EntryKind::Income, date: d(2026, 7, 10), amount: Cents::new(50000),
        member: Some(MemberId(1)), category: None, account: None, project: None, tag: None, recurring: None,
        name: Some(ct("Freelance")), note: None,
    });
    snap
}

#[test]
fn income_resolver_contract_override_and_one_off() {
    let snap = setup_income_snap();
    let resolver = IncomeResolver::new(&snap, Month::parse("2026-07").unwrap());
    let res = resolver.resolve();
    let alice = res.get(&MemberId(1)).unwrap();
    assert_eq!(alice.contract, Cents::new(400000));
    assert_eq!(alice.base, Cents::new(500000));
    assert_eq!(alice.one_off, Cents::new(50000));
    assert_eq!(alice.total, Cents::new(550000));
    assert!(alice.has_override);

    let bob = res.get(&MemberId(2)).unwrap();
    assert_eq!(bob.total, Cents::ZERO);
    assert!(!bob.has_override);
}

#[test]
fn weights_and_negative_clamp() {
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    snap.entries.push(Entry {
        id: EntryId(1), kind: EntryKind::Income, date: d(2026, 7, 10), amount: Cents::new(-5000),
        member: Some(MemberId(1)), category: None, account: None, project: None, tag: None, recurring: None,
        name: Some(ct("Clawback")), note: None,
    });
    let resolver = IncomeResolver::new(&snap, Month::parse("2026-07").unwrap());
    let weights = resolver.weights();
    assert_eq!(weights.len(), 1);
    assert_eq!(weights[0].weight, 0.0);
}

#[test]
fn mid_month_switch_prorates_old_job_and_delays_new_salary() {
    // Old job ends July 14: pays 14/31 of 3,000.00. New job starts July 15: nothing in July; the first salary in August also covers the 17 July days (3,500.00 + 1,919.35).
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    snap.jobs.push(Job {
        id: JobId(1), member: MemberId(1), amount: Cents::new(300000), frequency: Frequency::Monthly,
        valid_from: d(2026, 1, 1), valid_to: Some(d(2026, 7, 14)), name: ct("OldJob"), notes: None,
    });
    snap.jobs.push(Job {
        id: JobId(2), member: MemberId(1), amount: Cents::new(350000), frequency: Frequency::Monthly,
        valid_from: d(2026, 7, 15), valid_to: None, name: ct("NewJob"), notes: None,
    });

    let resolver = IncomeResolver::new(&snap, Month::parse("2026-07").unwrap());
    let res = resolver.resolve();
    let alice = res.get(&MemberId(1)).unwrap();
    assert_eq!(alice.contract, Cents::new(135484));

    let august = IncomeResolver::new(&snap, Month::parse("2026-08").unwrap()).resolve();
    assert_eq!(august.get(&MemberId(1)).unwrap().contract, Cents::new(541935));
}

#[test]
fn negative_clawback_exceeding_base_income() {
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    snap.jobs.push(Job {
        id: JobId(1), member: MemberId(1), amount: Cents::new(200000), frequency: Frequency::Monthly,
        valid_from: d(2026, 1, 1), valid_to: None, name: ct("Job"), notes: None,
    });
    // Massive clawback / repayment entry of -3,000.00
    snap.entries.push(Entry {
        id: EntryId(1), kind: EntryKind::Income, date: d(2026, 7, 10), amount: Cents::new(-300000),
        member: Some(MemberId(1)), category: None, account: None, project: None, tag: None, recurring: None,
        name: Some(ct("TaxClawback")), note: None,
    });
    let resolver = IncomeResolver::new(&snap, Month::parse("2026-07").unwrap());
    let res = resolver.resolve();
    let alice = res.get(&MemberId(1)).unwrap();
    assert_eq!(alice.contract, Cents::new(200000));
    assert_eq!(alice.one_off, Cents::new(-300000));
    assert_eq!(alice.total, Cents::new(-100000));
    // Weights clamp to 0.0
    let weights = resolver.weights();
    assert_eq!(weights[0].weight, 0.0);
}

fn job(id: i64, amount: i64, from: NaiveDate, to: Option<NaiveDate>) -> Job {
    Job {
        id: JobId(id), member: MemberId(1), amount: Cents::new(amount), frequency: Frequency::Monthly,
        valid_from: from, valid_to: to, name: ct("Job"), notes: None,
    }
}

fn contract_in(snap: &Snapshot, month: &str) -> Cents {
    let res = IncomeResolver::new(snap, Month::parse(month).unwrap()).resolve();
    res.get(&MemberId(1)).unwrap().contract
}

#[test]
fn first_job_starting_mid_month_is_prorated() {
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    snap.jobs.push(job(1, 300000, d(2026, 6, 15), None));
    // 16 of 30 days.
    assert_eq!(contract_in(&snap, "2026-06"), Cents::new(160000));
    assert_eq!(contract_in(&snap, "2026-07"), Cents::new(300000));
}

#[test]
fn bounded_contract_inside_a_month_counts_only_its_days() {
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    snap.jobs.push(job(1, 300000, d(2026, 6, 10), Some(d(2026, 6, 20))));
    // 11 of 30 days.
    assert_eq!(contract_in(&snap, "2026-06"), Cents::new(110000));
}
