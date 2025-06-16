use std::error::Error;
use dashmap::{DashMap, Entry};
use rust_decimal::dec;
use crate::model::{Account, RecordKind, TxEvent, TxKindRaw, TxRecord};

pub struct Processor {
    accounts: DashMap<u16, Account>,
    tx_history: DashMap<u32, TxRecord>
}

impl Processor {
    pub fn default() -> Processor {
        Processor {
            accounts: DashMap::new(),
            tx_history: DashMap::new()
        }
    }
    pub fn accounts(&self) -> DashMap<u16, Account> { self.accounts.clone() }
    pub fn tx_history(&self) -> DashMap<u32, TxRecord> { self.tx_history.clone() }
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