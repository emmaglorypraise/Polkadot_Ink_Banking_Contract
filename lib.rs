#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod simple_bank {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct Token {
        balances: Mapping<AccountId, u128>,
        owner: AccountId,
        total_supply: u128,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct Mint {
        #[ink(topic)]
        to: AccountId,
        value: u128,
        new_total_supply: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct Burn {
        #[ink(topic)]
        from: AccountId,
        value: u128,
        new_total_supply: u128,
        timestamp: u64,
    }

    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        InsufficientBalance,
        Unauthorized,
        Overflow,
        InvalidAmount,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl Default for Token {
     fn default() -> Self {
            Self::new()
        }
     }

    impl Token {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                balances: Mapping::default(),
                owner: Self::env().caller(),
                total_supply: 0,
            }
        }

        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, amount: u128) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }

            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let balance = self.balance_of(to);
            let new_balance = balance.checked_add(amount).ok_or(Error::Overflow)?;
            let new_supply = self.total_supply.checked_add(amount).ok_or(Error::Overflow)?;
            
            self.balances.insert(to, &new_balance);
            self.total_supply = new_supply;
            
            self.env().emit_event(Mint { 
                to, 
                value: amount,
                new_total_supply: new_supply,
                timestamp: self.env().block_timestamp(),
            });
            
            Ok(())
        }

        #[ink(message)]
        pub fn burn(&mut self, amount: u128) -> Result<()> {
            let from = self.env().caller();
            let balance = self.balance_of(from);

            if balance < amount {
                return Err(Error::InsufficientBalance);
            }

            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let new_balance = balance.checked_sub(amount).ok_or(Error::Overflow)?;
            let new_supply = self.total_supply.checked_sub(amount).ok_or(Error::Overflow)?;

            self.balances.insert(from, &new_balance);
            self.total_supply = new_supply;

            self.env().emit_event(Burn {
                from,
                value: amount,
                new_total_supply: new_supply,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        #[ink(message)]
        pub fn balance_of(&self, account: AccountId) -> u128 {
            self.balances.get(account).unwrap_or(0)
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, amount: u128) -> Result<()> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let from = self.env().caller();
            let from_balance = self.balance_of(from);

            if from_balance < amount {
                return Err(Error::InsufficientBalance);
            }

            let to_balance = self.balance_of(to);
            let new_to_balance = to_balance.checked_add(amount).ok_or(Error::Overflow)?;

            self.balances.insert(from, &from_balance.checked_sub(amount).ok_or(Error::Overflow)?);
            self.balances.insert(to, &new_to_balance);

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value: amount,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        #[ink(message)]
        pub fn total_supply(&self) -> u128 {
            self.total_supply
        }

        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }
            self.owner = new_owner;
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn full_token_lifecycle() {
            let mut token = Token::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            
            assert!(token.mint(accounts.alice, 1000).is_ok());
            assert_eq!(token.balance_of(accounts.alice), 1000);
            assert_eq!(token.total_supply(), 1000);
            
            assert!(token.transfer(accounts.bob, 300).is_ok());
            assert_eq!(token.balance_of(accounts.alice), 700);
            assert_eq!(token.balance_of(accounts.bob), 300);
            
            assert!(token.burn(200).is_ok());
            assert_eq!(token.balance_of(accounts.alice), 500);
            assert_eq!(token.total_supply(), 800);
        }

        #[ink::test]
        fn zero_amount_fails() {
            let mut token = Token::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            
            assert_eq!(token.mint(accounts.bob, 0), Err(Error::InvalidAmount));
            assert_eq!(token.transfer(accounts.bob, 0), Err(Error::InvalidAmount));
        }

        #[ink::test]
        fn ownership_transfer_works() {
            let mut token = Token::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            
            assert_eq!(token.owner(), accounts.alice);
            
            // Transfer ownership
            assert!(token.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(token.owner(), accounts.bob);
            
            // Old owner can't mint
            assert_eq!(token.mint(accounts.charlie, 100), Err(Error::Unauthorized));
            
            // New owner can mint
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert!(token.mint(accounts.charlie, 100).is_ok());
        }

        #[ink::test]
        fn burn_insufficient_balance_fails() {
            let mut token = Token::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            
            token.mint(accounts.alice, 100).unwrap();
            assert_eq!(token.burn(200), Err(Error::InsufficientBalance));
        }
    }
}