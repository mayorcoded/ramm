extern crate core;

const PRECISION: u32 = 1_000_000;
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Share should be less than totalShare
    InvalidShare,
    /// Insufficient pool balance
    InsufficientLiquidity,
    /// Insufficient amount
    InsufficientAmount,
    /// Equivalent value of tokens not provided
    NonEquivalentValue,
    /// Slippage tolerance exceeded
    SlippageExceeded,
    /// Asset value less than threshold for contribution!
    ThresholdNotReached,
    /// Amount cannot be zero!
    ZeroAmount,
    /// Zero Liquidity
    ZeroLiquidity,
}

mod amm {
    use std::collections::HashMap;
    use crate::{Error, PRECISION};

    //hold the balance of an Account
    type Balances = HashMap<String, u32>;

    #[derive(Default)]
    struct Amm {
        fees: u32,
        total_pool_shares: u32,
        token_a_pool_balance: u32,
        token_b_pool_balance: u32,
        token_a_user_balance: Balances,
        token_b_user_balance: Balances,
        user_pool_shares: Balances,
    }
    impl Amm {
        pub fn new(fees: u32) -> Self {
            Self {
                fees: if fees >= 1000 { 0 } else { fees },
                ..Default::default()
            }
        }

        fn is_valid_amount(&self, account_id: &str, balances: &Balances, amount: u32 ) -> Result<(), Error> {
            let account_balance = *balances.get(account_id).unwrap_or(&0);
            match amount {
                0 => Err(Error::ZeroAmount),
                _ if amount > account_balance => Err(Error::InsufficientAmount),
                _ => Ok(())
            }
        }

        fn is_pool_active(&self) -> Result<(), Error> {
            match self.get_pool_balance() {
                0 => Err(Error::ZeroLiquidity),
                _ => Ok(())
            }
        }

        fn get_pool_balance(&self) -> u32 {
            self.token_a_pool_balance * self.token_b_pool_balance
        }

        pub fn get_free_tokens(&mut self, account_id: String, token_a_amount: u32, token_b_amount: u32) {
            let _account_id = account_id.as_str();
            let token_a_balance = *self.token_a_user_balance.get(_account_id).unwrap_or(&0);
            let token_b_balance = *self.token_b_user_balance.get(_account_id).unwrap_or(&0);
            self.token_a_user_balance.insert(account_id.clone(), token_a_balance + token_a_amount);
            self.token_b_user_balance.insert(account_id, token_b_balance + token_b_amount);
        }

        pub fn get_account_balance(&self, account_id: String,) -> (u32, u32, u32) {
            let token_a_balance = *self.token_a_user_balance
                .get(account_id.as_str()).unwrap_or(&0);
            let token_b_balance = *self.token_b_user_balance.
                get(account_id.as_str()).unwrap_or(&0);

            let pool_shares = *self.user_pool_shares
                .get(account_id.as_str()).unwrap_or(&0);
            (token_a_balance, token_b_balance, pool_shares)
        }

        pub fn get_pool_info(&self) -> (u32, u32, u32, u32) {
            (
                self.token_a_pool_balance,
                self.token_b_pool_balance,
                self.total_pool_shares,
                self.fees
            )

        }

        pub fn deposit(&mut self, account_id: String, token_a_amount: u32, token_b_amount: u32)
            -> Result<u32, Error>
        {
            self.is_valid_amount(
                account_id.as_str(),
                &self.token_a_user_balance,
                token_a_amount
            )?;
            self.is_valid_amount(
                account_id.as_str(),
                &self.token_b_user_balance,
                token_b_amount
            )?;

            let mut shares = 0;
            if self.total_pool_shares == 0 {
                shares = 100 * PRECISION
            } else {
                let token_a_share = self.total_pool_shares * token_a_amount /  self.token_a_pool_balance;
                let token_b_share = self.total_pool_shares * token_b_amount /  self.token_b_pool_balance;

                if token_a_share != token_b_share {
                    return Err(Error::NonEquivalentValue);
                }
                shares = token_a_share;
            }

            if shares == 0 {
                return Err(Error::ThresholdNotReached);
            }

            let token_a_balance = *self.token_a_user_balance.get(account_id.as_str()).unwrap_or(&0);
            let token_b_balance = *self.token_b_user_balance.get(account_id.as_str()).unwrap_or(&0);
            self.token_a_user_balance.insert(
                account_id.clone(),
                token_a_balance - token_a_amount,
            );
            self.token_b_user_balance.insert(
                account_id.clone(),
                token_b_balance - token_b_amount
            );

            self.token_a_pool_balance += token_a_amount;
            self.token_b_pool_balance += token_b_amount;
            self.total_pool_shares += shares;
            self.user_pool_shares
                .entry(account_id)
                .and_modify(|val| { *val += shares })
                .or_insert(shares);

            Ok(shares)
        }

        pub fn get_token_a_swap_amount_out(&self, token_b_amount: u32) -> Result<u32, Error> {
            self.is_pool_active()?;
            Ok(self.token_a_pool_balance * token_b_amount/self.token_b_pool_balance)
        }

        pub fn get_token_b_swap_amount_out(&self, token_a_amount: u32) -> Result<u32, Error> {
            self.is_pool_active()?;
            Ok(self.token_b_pool_balance * token_a_amount/self.token_a_pool_balance)
        }

        pub fn get_withdraw_amount(&self, share: u32) -> Result<(u32, u32), Error> {
            self.is_pool_active()?;
            if share > self.total_pool_shares {
                return Err(Error::InvalidShare);
            }

            let token_a_amount = self.token_a_pool_balance * share / self.total_pool_shares;
            let token_b_amount = self.token_b_pool_balance * share / self.total_pool_shares;

            Ok((token_a_amount, token_b_amount))
        }

        pub fn withdraw(&mut self, account_id: String, share: u32) -> Result<(u32, u32), Error> {
            self.is_valid_amount(
                account_id.as_str(),
                &self.user_pool_shares,
                share
            )?;
            let (token_a_amount, token_b_amount) = self.get_withdraw_amount(share)?;
            self.user_pool_shares
                .entry(account_id.clone())
                .and_modify(|val| {*val += share});

            self.total_pool_shares -= share;

            self.token_a_pool_balance -= token_a_amount;
            self.token_b_pool_balance -= token_b_amount;

            self.token_a_user_balance
                .entry(account_id.clone())
                .and_modify(|val| { *val += token_a_amount });
            self.token_b_user_balance
                .entry(account_id.clone())
                .and_modify(|val| { *val += token_b_amount });


            Ok((token_a_amount,token_b_amount))
        }

        pub fn get_swap_amount_for_token_b(&self, token_a_amount: u32) -> Result<u32, Error> {
            self.is_pool_active()?;
            let token_a_amount = (1000 - self.fees) * token_a_amount / 1000;

            let total_token_a = self.token_a_pool_balance + token_a_amount;
            let total_token_b = self.get_pool_balance() / total_token_a;
            let mut token_b_amount = self.token_b_pool_balance - total_token_b;

            if total_token_b == self.token_b_pool_balance {
                token_b_amount -= 1;
            }

            Ok(token_b_amount)
        }

        pub fn get_swap_amount_for_token_a(&self, token_b_amount: u32) -> Result<u32, Error> {
            self.is_pool_active()?;
            if token_b_amount > self.token_b_pool_balance {
                return Err(Error::InsufficientLiquidity);
            }

            let total_token_b = self.token_b_pool_balance - token_b_amount;
            let total_token_a = self.get_pool_balance() /total_token_b;
            let token_a_amount = (total_token_a - self.token_a_pool_balance) * 1000 /
                (1000 - self.fees);

            Ok(token_a_amount)
        }

        pub fn swap_token_a_for_token_b(&mut self, account_id: String, token_a_amount: u32, min_token_b: u32)
                                        -> Result<u32, Error> {
            self.is_valid_amount(
                account_id.as_str(),
                &self.token_a_user_balance,
                token_a_amount
            )?;

            let token_b_amount = self.get_swap_amount_for_token_a(token_a_amount)?;
            if token_b_amount < min_token_b {
                return Err(Error::SlippageExceeded);
            }

            self.token_a_user_balance
                .entry(account_id.clone())
                .and_modify(|val| { *val -= token_a_amount });

            self.token_a_pool_balance += token_a_amount;
            self.token_b_pool_balance -= token_b_amount;

            self.token_b_user_balance
                .entry(account_id)
                .and_modify(|val| { *val += token_b_amount });

            Ok(token_b_amount)
        }

        pub fn swap_token_b_for_token_a(&mut self, account_id: String, token_b_amount: u32, min_token_a: u32)
                                        -> Result<u32, Error> {
            self.is_valid_amount(
                account_id.as_str(),
                &self.token_b_user_balance,
                token_b_amount
            )?;

            let token_a_amount = self.get_swap_amount_for_token_a(token_b_amount)?;
            if token_a_amount < min_token_a {
                return Err(Error::SlippageExceeded);
            }

            self.token_b_user_balance
                .entry(account_id.clone())
                .and_modify(|val| { *val -= token_b_amount });

            self.token_a_pool_balance -= token_a_amount;
            self.token_b_pool_balance += token_b_amount;

            self.token_a_user_balance
                .entry(account_id)
                .and_modify(|val| { *val += token_a_amount });

            Ok(token_a_amount)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn get_account_id() -> String {
            String::from("account-1")
        }

        #[test]
        fn test_constructor() {
            let amm = Amm::new(0);
            assert_eq!(amm.get_account_balance(get_account_id()), (0, 0, 0));
            assert_eq!(amm.get_pool_info(), (0, 0, 0, 0));
        }

        #[test]
        fn test_get_free_tokens() {
            let mut amm = Amm::new(100);
            amm.get_free_tokens(get_account_id(), 100, 200);
            assert_eq!(amm.get_account_balance(get_account_id()), (100, 200, 0));
        }

        #[test]
        fn test_zero_liquidity() {
            let mut amm = Amm::new(100);
            let res = amm.get_token_a_swap_amount_out(4);
            assert_eq!(res, Err(Error::ZeroLiquidity));
        }

        #[test]
        fn test_deposit() {
            let mut amm = Amm::new(100);
            amm.get_free_tokens(get_account_id(), 100, 200);
            let share = amm.deposit(
                get_account_id(),
                10,
                20
            ).unwrap();
            assert_eq!(share, 100_000_000);
            assert_eq!(amm.get_pool_info(), (10, 20, share, 100));
            assert_eq!(amm.get_account_balance(get_account_id()), (90, 180, share));
        }

        #[test]
        fn test_withdraw() {
            let mut amm = Amm::new(0);
            amm.get_free_tokens(get_account_id(), 100, 200);
            let share = amm.deposit(
                get_account_id(),
                10,
                20
            ).unwrap();
            assert_eq!(amm.withdraw(get_account_id(),share / 5).unwrap(), (2, 4));
            //assert_eq!(amm.get_account_balance(get_account_id()), (92, 184, 4 * share / 5));
            assert_eq!(amm.get_pool_info(), (8, 16, 4 * share / 5, 0));
        }

        #[test]
        fn test_swap() {
            let mut amm = Amm::new(0);
            amm.get_free_tokens(get_account_id(), 100, 200);
            let share = amm.deposit(
                get_account_id(),
                50,
                100
            ).unwrap();
            let token_b_amount = amm.swap_token_a_for_token_b(
                get_account_id(),
                50,
                50
            ).unwrap();
            assert_eq!(token_b_amount, 50);
            assert_eq!(amm.get_pool_info(), (100, 50, share, 0));
            assert_eq!(amm.get_account_balance(get_account_id()), (0, 150, share));
        }

        #[test]
        fn test_slippage() {
            let mut amm = Amm::new(0);
            amm.get_free_tokens(get_account_id(), 100, 200);
            let share = amm.deposit(
                get_account_id(),
                50,
                100
            ).unwrap();
            let token_b_amount = amm.swap_token_a_for_token_b(
                get_account_id(),
                50,
                51
            );
            assert_eq!(token_b_amount, Err(Error::SlippageExceeded));
            assert_eq!(amm.get_pool_info(), (50, 100, share, 0));
            assert_eq!(amm.get_account_balance(get_account_id()), (50, 100, share));
        }

        #[test]
        fn test_fees() {
            let mut amm = Amm::new(100);
            amm.get_free_tokens(get_account_id(), 100, 200);
            let share = amm.deposit(
                get_account_id(),
                50,
                100
            ).unwrap();
            let token_b_amount = amm.get_swap_amount_for_token_b(50).unwrap();
            assert_eq!(token_b_amount, 48);
        }
    }
}

