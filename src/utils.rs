use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::time::Instant;
use dashmap::DashMap;
use crate::model::{Account, TxEvent};
use crate::processor;

pub fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path)
    }
}

pub fn process_file(file_path: OsString) -> Result<DashMap<u16, Account>, Box<dyn Error>> {
    let start = Instant::now();
    let file = File::open(file_path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(file);
    let mut processor = processor::Processor::default();
    for record in rdr.records() {
        let ev: TxEvent = record?.deserialize(None)?;
        processor.process(ev)?;
    }
    println!("{:?}", start.elapsed().as_millis());
    Ok(processor.accounts())
}

#[cfg(test)]
mod process_file_tests {
    use std::ffi::OsString;
    use rust_decimal::dec;
    use crate::utils::{process_file};

    #[test]
    fn run_simple_deposit_csv() {
        let result = process_file(OsString::from("src/test_csv_data/test.csv"));
        assert!(!result.is_err());
        let result = result.unwrap();
        assert_eq!(result.get(&1).unwrap().available(), dec!(1.5));
        assert_eq!(result.get(&2).unwrap().available(), dec!(2));
    }

    #[test]
    fn run_test_locked() {
        let result = process_file(OsString::from("src/test_csv_data/test_locked.csv"));
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        let client_one = result.get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(0.5));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(0.5));
        assert_eq!(client_one.locked(), true);
    }


    #[test]
    fn run_test_early_locked() {
        let result = process_file(OsString::from("src/test_csv_data/test_data_early_lock.csv"));
        assert!(!result.is_err());
        let result = result.unwrap();
        let client_one = result.get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(2.0));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(2.0));
        assert_eq!(client_one.locked(), true);
    }

    #[test]
    fn run_test_dispute_resolve() {
        let result = process_file(OsString::from("src/test_csv_data/test_data_dispute_resolve.csv"));
        assert!(!result.is_err());
        let result = result.unwrap();
        let client_one = result.get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(1.5));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(1.5));
        assert_eq!(client_one.locked(), false);
    }

    #[test]
    fn run_test_over_withdrawal() {
        let result = process_file(OsString::from("src/test_csv_data/test_over_withdrawal.csv"));
        assert!(!result.is_err());
        let result = result.unwrap();
        let client_one = result.get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(3.0));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(3.0));
        assert_eq!(client_one.locked(), false);
    }

    #[test]
    fn run_test_dispute_withdrawal() {
        let result = process_file(OsString::from("src/test_csv_data/test_dispute_withdrawal.csv"));
        assert!(!result.is_err());
        let result = result.unwrap();
        let client_one = result.get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(3.0));
        assert_eq!(client_one.held(), dec!(-1.5));
        assert_eq!(client_one.total(), dec!(1.5));
        assert_eq!(client_one.locked(), false);
    }

    #[test]
    fn run_test_dispute_withdrawal_resolve() {
        let result = process_file(OsString::from("src/test_csv_data/test_dispute_withdrawal_resolve.csv"));
        assert!(!result.is_err());
        let result = result.unwrap();
        let client_one = result.get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(1.500));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(1.500));
        assert_eq!(client_one.locked(), false);
    }


    #[test]
    fn run_test_dispute_withdrawal_chargeback() {
        let result = process_file(OsString::from("src/test_csv_data/test_dispute_withdrawal_chargeback.csv"));
        assert!(!result.is_err());
        let result = result.unwrap();
        let client_one = result.get(&1);
        assert!(client_one.is_some());
        let client_one = client_one.unwrap().clone();
        assert_eq!(client_one.available(), dec!(3));
        assert_eq!(client_one.held(), dec!(0));
        assert_eq!(client_one.total(), dec!(3));
        assert_eq!(client_one.locked(), true);
    }
}

