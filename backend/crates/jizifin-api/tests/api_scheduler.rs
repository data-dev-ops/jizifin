mod common;

use chrono::Datelike;
use common::TestClient;
use jizifin_domain::entities::{CategoryKind, EntryKind, PayMode, Recurrence};
use jizifin_domain::{Cents, Ciphertext};
use jizifin_store::inputs::{CategoryInput, MemberInput, RecurringInput};
use jizifin_store::repo::{Categories, Entries, Members, Recurring};

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

async fn create_due_template(client: &TestClient) {
    let today = chrono::Utc::now().date_naive();
    client.state.write("test", move |tx| {
        let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let c = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Phone"), pay_mode: PayMode::Shared })?;
        Recurring::create(tx, &RecurringInput {
            kind: EntryKind::Expense, amount: Cents::new(4500), member: m, category: c,
            account: None, day_of_month: today.day(), recurrence: Recurrence::Monthly,
            valid_from: today, valid_to: None, name: ct("Phone_Bill"), is_active: true,
        })?;
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn scheduler_startup_recovery_materializes_and_deduplicates() {
    let client = TestClient::authenticated().await;
    create_due_template(&client).await;

    let mut rx = client.state.events.subscribe();
    jizifin_api::scheduler::run_once(&client.state).await;

    let event = rx.recv().await.unwrap();
    assert_eq!(event, "{\"event\":\"entries.created\"}");

    let entries = client.state.read(|conn| Ok(Entries::list(conn, &Default::default())?.entries)).await.unwrap();
    assert_eq!(entries.len(), 1);

    jizifin_api::scheduler::run_once(&client.state).await;
    let entries_after = client.state.read(|conn| Ok(Entries::list(conn, &Default::default())?.entries)).await.unwrap();
    assert_eq!(entries_after.len(), 1);
}
