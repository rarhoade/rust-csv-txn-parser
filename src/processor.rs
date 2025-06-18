use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use dashmap::{DashMap, Entry};
use rust_decimal::dec;
use crate::model::{Account, ClientId, RecordKind, TxEvent, TxId, TxKindRaw, TxRecord};
use crate::processor;

pub struct Processor {
    accounts: DashMap<ClientId, Account>,
    tx_history: DashMap<TxId, TxRecord>
}

impl Processor {
    pub fn default() -> Processor {
        Processor {
            accounts: DashMap::new(),
            tx_history: DashMap::new()
        }
    }
    pub fn process_file(file_path: OsString) -> Result<Processor, Box<dyn Error>> {
        let file = File::open(file_path)?;
        let mut rdr = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(file);
        let mut processor = processor::Processor::default();
        for record in rdr.records() {
            let ev: TxEvent = record?.deserialize(None)?;
            processor.process(ev)?;
        }
        Ok(processor)
    }
    pub fn accounts(&self) -> &DashMap<ClientId, Account> { &self.accounts }
    pub fn tx_history(&self) -> &DashMap<TxId, TxRecord> { &self.tx_history }
    pub fn process(&mut self, ev: TxEvent) -> Result<(), Box<dyn Error>>{
        match ev.kind {
            TxKindRaw::Deposit => self.deposit(ev)?,
            TxKindRaw::Withdrawal => self.withdrawal(ev)?,
            TxKindRaw::Dispute => self.dispute(ev)?,
            TxKindRaw::Resolve => self.resolve(ev)?,
            TxKindRaw::Chargeback => self.chargeback(ev)?
        }
        Ok(())
    }

    fn deposit(&mut self, event: TxEvent) -> Result<(), Box<dyn Error>> {
        let amount = match event.amount {
            None => {return Err(From::from(format!("No value amount to deposit for tx {}", event.tx)));}
            Some(a) => a
        };

        self.accounts.entry(event.client)
            .and_modify(|existing| {
                existing.modify_available(&event.amount.unwrap_or(dec!(0)), &RecordKind::Deposit)
            })
            .or_insert({
                let mut acc = Account::new();
                acc.modify_available(&event.amount.unwrap_or(dec!(0)), &RecordKind::Deposit);
                acc
            });
        self.tx_history.insert(event.tx, TxRecord::new(
            event.client,
            amount,
            false,
            RecordKind::Deposit
        ));
        Ok(())
    }
    
    fn withdrawal(&mut self, event: TxEvent) -> Result<(), Box<dyn Error>> {
        let amount = match event.amount {
            None => {return Err(From::from(format!("No value amount to withdraw for tx {}", event.tx)));}
            Some(a) => a
        };
        self.accounts.entry(event.client)
            .and_modify(|existing| {
                if !existing.locked() && existing.available() >= amount {
                    existing.modify_available(&amount, &RecordKind::Withdrawal)
                }
            })
            .or_insert({
                let mut acc = Account::new();
                acc.modify_available(&dec!(0), &RecordKind::Deposit);
                acc
            });
        self.tx_history.insert(event.tx, TxRecord::new(
            event.client,
            event.amount.unwrap(),
            false,
            RecordKind::Withdrawal
        ));
        Ok(())
    }

    fn dispute(&mut self, ev: TxEvent) -> Result<(), Box<dyn Error>> {
        match self.tx_history.entry(ev.tx) {
            Entry::Occupied(mut map_val) => {
                if !map_val.get().disputed() {
                    self.accounts
                        .entry(map_val.get().client())
                        .and_modify(|existing| {
                            if !existing.locked() {
                                existing.dispute_funds(
                                    &map_val.get().amount(),
                                    &map_val.get().kind(),
                                );
                            }
                            map_val.get_mut().modify_disputed(true);
                        });
                }
            }
            Entry::Vacant(_) => {}
        }
        Ok(())
    }
    
    fn resolve(&mut self, ev: TxEvent) -> Result<(), Box<dyn Error>> {
        match self.tx_history.entry(ev.tx) {
            Entry::Occupied(mut map_val) => {
                if map_val.get().disputed() && !map_val.get().dispute_finished(){
                    self.accounts
                        .entry(map_val.get().client())
                        .and_modify(|existing| {
                            if !existing.locked() {
                                existing.resolve_funds(
                                    &map_val.get().amount(),
                                    &map_val.get().kind(),
                                );
                            }
                            map_val.get_mut().finish_dispute();
                        });
                }
            }
            Entry::Vacant(_) => {}
        }
        Ok(())
    }
    
    fn chargeback(&mut self, ev: TxEvent) -> Result<(), Box<dyn Error>> {
        match self.tx_history.entry(ev.tx) {
            Entry::Occupied(mut map_val) => {
                if map_val.get().disputed() && !map_val.get().dispute_finished() {
                    self.accounts
                        .entry(map_val.get().client())
                        .and_modify(|existing| {
                            if !existing.locked() {
                                existing.chargeback_funds(
                                    &map_val.get().amount(),
                                    &map_val.get().kind(),
                                );
                            }
                            map_val.get_mut().finish_dispute();
                        });
                }
            }
            Entry::Vacant(_) => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod process_file_tests {
    use std::ffi::OsString;
    use rust_decimal::dec;
    use crate::Processor;

    #[test]
    fn run_simple_deposit_csv() {
        let result = Processor::process_file(OsString::from("src/test_csv_data/test.csv"));
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.accounts().get(&1).unwrap().available(), dec!(1.5));
        assert_eq!(result.accounts().get(&2).unwrap().available(), dec!(2));
    }

    #[test]
    fn run_test_locked() {
        let result = Processor::process_file(OsString::from("src/test_csv_data/test_locked.csv"));
        assert!(!result.is_err());
        let result = result.unwrap();
        let client_one = result.accounts().get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(0.5));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(0.5));
        assert!(client_one.locked());
    }


    #[test]
    fn run_test_early_locked() {
        let result = Processor::process_file(OsString::from("src/test_csv_data/test_data_early_lock.csv"));
        assert!(result.is_ok());
        let result = result.unwrap();
        let client_one = result.accounts().get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(2.0));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(2.0));
        assert!(client_one.locked());
    }

    #[test]
    fn run_test_dispute_resolve() {
        let result = Processor::process_file(OsString::from("src/test_csv_data/test_data_dispute_resolve.csv"));
        assert!(result.is_ok());
        let result = result.unwrap();
        let client_one = result.accounts().get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(1.5));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(1.5));
        assert!(!client_one.locked());
    }

    #[test]
    fn run_test_over_withdrawal() {
        let result = Processor::process_file(OsString::from("src/test_csv_data/test_over_withdrawal.csv"));
        assert!(result.is_ok());
        let result = result.unwrap();
        let client_one = result.accounts().get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(3.0));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(3.0));
        assert!(!client_one.locked());
    }

    #[test]
    fn run_test_dispute_withdrawal() {
        let result = Processor::process_file(OsString::from("src/test_csv_data/test_dispute_withdrawal.csv"));
        assert!(result.is_ok());
        let result = result.unwrap();
        let client_one = result.accounts().get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(3.0));
        assert_eq!(client_one.held(), dec!(-1.5));
        assert_eq!(client_one.total(), dec!(1.5));
        assert!(!client_one.locked());
    }

    #[test]
    fn run_test_dispute_withdrawal_resolve() {
        let result = Processor::process_file(OsString::from("src/test_csv_data/test_dispute_withdrawal_resolve.csv"));
        assert!(result.is_ok());
        let result = result.unwrap();
        let client_one = result.accounts().get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(1.500));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(1.500));
        assert!(!client_one.locked());
    }


    #[test]
    fn run_test_dispute_withdrawal_chargeback() {
        let result = Processor::process_file(OsString::from("src/test_csv_data/test_dispute_withdrawal_chargeback.csv"));
        assert!(result.is_ok());
        let result = result.unwrap();
        let client_one = result.accounts().get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(4));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(4));
        assert!(client_one.locked());
    }
}