use rust_decimal::{dec, Decimal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TxEvent {
    #[serde(rename = "type")]
    pub kind: TxKindRaw,
    pub client: ClientId,
    pub tx: TxId,
    pub amount: Option<Decimal>
}

#[derive(Debug, Default, Clone)]
pub struct Account {
    available: Decimal,
    held:      Decimal,
    locked:    bool,
}

impl Account {
    pub fn new() -> Account {
        Account {
            available: dec!(0),
            held: dec!(0),
            locked: false
        }
    }
    pub fn total(&self) -> Decimal { self.available + self.held }
    pub fn modify_available(&mut self, val: &Decimal, record_kind: &RecordKind) {
        match record_kind {
            RecordKind::Deposit => { self.available += val}
            RecordKind::Withdrawal => { self.available -= val}
        }
    }
    pub fn modify_held(&mut self, val: &Decimal, record_kind: &RecordKind) {
        match record_kind {
            RecordKind::Deposit => { self.held += val}
            RecordKind::Withdrawal => { self.held -= val}
        }
    }
    pub fn dispute_funds(&mut self, val: &Decimal, record_kind: &RecordKind) {
        match record_kind {
            RecordKind::Deposit => {
                self.available -= val;
                self.held += val
            },
            RecordKind::Withdrawal => {
                self.available += val;
                self.held -= val

            }
        }
    }
    pub fn resolve_funds(&mut self, val: &Decimal, record_kind: &RecordKind) {
        match record_kind {
            RecordKind::Deposit => {
                self.available += val;
                self.held -= val
            },
            RecordKind::Withdrawal => {
                self.available -= val;
                self.held += val

            }
        }
    }
    pub fn chargeback_funds(&mut self, val: &Decimal, record_kind: &RecordKind) {
        match record_kind {
            RecordKind::Deposit => {
                self.held -= val
            },
            RecordKind::Withdrawal => {
                self.held += val

            }
        }
        self.lock();
    }
    pub fn available(&self) -> Decimal { self.available }
    pub fn held(&self) -> Decimal { self.held }
    pub fn locked(&self) -> bool { self.locked }
    pub fn lock(&mut self) { self.locked = true}
    pub fn unlock(&mut self) { self.locked = false}
}

#[derive(Debug, Clone)]
pub struct TxRecord {
    client:  ClientId,
    amount:  Decimal,
    disputed: bool,
    dispute_finished: bool,
    kind:    RecordKind, // Deposit | Withdrawal
}

impl TxRecord {
    pub fn new(client: ClientId, amount: Decimal, disputed: bool, kind: RecordKind) -> TxRecord {
        TxRecord {
            client,
            amount,
            disputed,
            dispute_finished: false,
            kind,
        }
    }
    pub fn client(&self) -> ClientId { self.client }
    pub fn amount(&self) -> Decimal { self.amount }
    pub fn disputed(&self) -> bool { self.disputed }
    pub fn dispute_finished(&self) -> bool { self.dispute_finished }
    pub fn kind(&self) -> RecordKind { self.kind.clone() }
    pub fn modify_disputed(&mut self, val: bool) { self.disputed = val }
    pub fn finish_dispute(&mut self) { self.dispute_finished = true  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(deserialize="lowercase"))]
pub enum TxKindRaw {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum RecordKind {
    Deposit,
    Withdrawal
}

pub type ClientId = u16;
pub type TxId = u32;

#[cfg(test)]
mod test_account {
    use rust_decimal::dec;
    use crate::model::{Account, RecordKind};

    #[test]
    fn make_account_with_getters() {
        let account = Account::new();
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert!(!account.locked);
    }

    #[test]
    fn test_account_total() {
        let mut account = Account::new();
        account.modify_available(&dec!(10), &RecordKind::Deposit);
        account.modify_held(&dec!(3), &RecordKind::Deposit);
        assert_eq!(account.available, dec!(10));
        assert_eq!(account.held, dec!(3));
        assert_eq!(account.total(), account.available + account.held);
    }


    #[test]
    fn test_modify_held_and_available() {
        let mut account = Account::new();
        account.modify_available(&dec!(10), &RecordKind::Deposit);
        account.modify_held(&dec!(20), &RecordKind::Deposit);
        assert_eq!(account.available, dec!(10));
        assert_eq!(account.held, dec!(20));
        assert_eq!(account.total(),dec!(30));

    }

    #[test]
    fn test_lock_unlock() {
        let mut account = Account::new();
        account.lock();
        assert!(account.locked);
        account.unlock();
        assert!(!account.locked);
    }

    #[test]
    fn test_dispute_funds_and_resolve_funds() {
        let mut account = Account::new();
        account.modify_available(&dec!(10), &RecordKind::Deposit);
        account.dispute_funds(&dec!(7), &RecordKind::Deposit);
        assert_eq!(account.available, dec!(3));
        assert_eq!(account.held, dec!(7));
        account.resolve_funds(&dec!(7), &RecordKind::Deposit);
        assert_eq!(account.available, dec!(10));
        assert_eq!(account.held, dec!(0));
    }
}

#[cfg(test)]
mod test_tx_record {
    use rust_decimal::dec;
    use crate::model::{RecordKind, TxRecord};

    #[test]
    fn test_new_with_getters() {
        let record = TxRecord::new(
            1,
            dec!(1),
            false,
            RecordKind::Withdrawal
        );

        assert_eq!(record.amount(), dec!(1));
        assert_eq!(record.client(), 1);
        assert_eq!(record.kind(), RecordKind::Withdrawal);
        assert!(!record.dispute_finished());
        assert!(!record.disputed());
    }

    #[test]
    fn test_modify_disputed() {
        let mut record = TxRecord::new(
            1,
            dec!(1),
            false,
            RecordKind::Withdrawal
        );
        record.modify_disputed(true);
        assert!(record.disputed());
        record.finish_dispute();
        assert!(record.dispute_finished());
    }
}