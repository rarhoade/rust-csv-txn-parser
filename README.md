
# To Run
```shell
$ cargo run -- transactions.csv > output.csv
```

# Input Format
```
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
dispute, 1, 4,
withdrawal, 2, 5, 3.0
chargeback, 1, 4,
```

# Output Format
```
client, available, held, total, locked
1, 1.5, 0.0, 1.5, false
2, 2.0, 0.0, 2.0, false
```

# Extra Implementation
If you would like to see a channel based implementation of this that preserves transaction processing order per account, check out the branch `concurrency-implementation`. I could not get it to beat my benchmarks at 100 clients over 1,000,000 transactions in time but it was fun nontheless.
```bash
git fetch
git checkout concurrency-implementation
```

# Assumptions
- A locked account can receive **deposits** but not process **withdrawals**. Funds being processed and shifted from **disputes**, whether they resolve in **chargebacks** or **resolutions** will also still process on locked accounts.
- A **dispute** can occur for both **deposits** and **withdrawals**.  
  - Deposit
    - Funds will be subtracted from available, and put into held. 
  - Withdrawal
    - Funds will be added to available, subtracted from held. 
  - **Resolution** in both cases with reverse the dispute changes to the account.
  - **Chargeback** in both cases will finalize the dispute and reverse the transaction in both cases.
- A dispute can cause **held** or **available** funds to become negative. This is common for many financial institutions so the same assumption is being made here.
- **Resolutions** and **Chargebacks** on transactions that have already been charged back will be ignored. A transaction that has been disputed and then resolved can be disputed again.