#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod token {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct Token {
        balances: Mapping<AccountId, u128>,
        allowances: Mapping<(AccountId, AccountId), u128>,
        blacklist: Mapping<AccountId, bool>,
        owner: AccountId,
        total_supply: u128,
        paused: bool,
    }

    /// Transfer event
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: u128,
    }

    /// Approval event
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: u128,
    }

    /// Pause state changed
    #[ink(event)]
    pub struct Paused {
        #[ink(topic)]
        paused: bool,
    }

    /// Account blacklist status changed
    #[ink(event)]
    pub struct BlacklistUpdated {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        blacklisted: bool,
    }

    /// Ownership transferred
    #[ink(event)]
    pub struct OwnershipTransferred {
        #[ink(topic)]
        previous_owner: AccountId,
        #[ink(topic)]
        new_owner: AccountId,
    }

    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        InsufficientBalance,
        InsufficientAllowance,
        Unauthorized,
        Overflow,
        InvalidAmount,
        ContractPaused,
        AccountBlacklisted,
        SelfApproval,
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
                allowances: Mapping::default(),
                blacklist: Mapping::default(),
                owner: Self::env().caller(),
                total_supply: 0,
                paused: false,
            }
        }

        #[inline]
        fn only_owner(&self) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        #[inline]
        fn when_not_paused(&self) -> Result<()> {
            if self.paused {
                return Err(Error::ContractPaused);
            }
            Ok(())
        }

        #[inline]
        fn not_blacklisted(&self, account: AccountId) -> Result<()> {
            if self.blacklist.get(account).unwrap_or(false) {
                return Err(Error::AccountBlacklisted);
            }
            Ok(())
        }

        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, amount: u128) -> Result<()> {
            self.only_owner()?;
            self.not_blacklisted(to)?;

            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let balance = self.balance_of(to);
            let new_balance = balance.checked_add(amount).ok_or(Error::Overflow)?;
            let new_supply = self
                .total_supply
                .checked_add(amount)
                .ok_or(Error::Overflow)?;

            self.balances.insert(to, &new_balance);
            self.total_supply = new_supply;

            self.env().emit_event(Transfer {
                from: None,
                to: Some(to),
                value: amount,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn burn(&mut self, amount: u128) -> Result<()> {
            self.when_not_paused()?;

            let from = self.env().caller();
            self.not_blacklisted(from)?;

            let balance = self.balance_of(from);

            if balance < amount {
                return Err(Error::InsufficientBalance);
            }

            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let new_balance = balance.checked_sub(amount).ok_or(Error::Overflow)?;
            let new_supply = self
                .total_supply
                .checked_sub(amount)
                .ok_or(Error::Overflow)?;

            self.balances.insert(from, &new_balance);
            self.total_supply = new_supply;

            self.env().emit_event(Transfer {
                from: Some(from),
                to: None,
                value: amount,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn balance_of(&self, account: AccountId) -> u128 {
            self.balances.get(account).unwrap_or(0)
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, amount: u128) -> Result<()> {
            self.when_not_paused()?;

            let from = self.env().caller();
            self.not_blacklisted(from)?;
            self.not_blacklisted(to)?;

            self._transfer(from, to, amount)
        }

        fn _transfer(&mut self, from: AccountId, to: AccountId, amount: u128) -> Result<()> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let from_balance = self.balance_of(from);

            if from_balance < amount {
                return Err(Error::InsufficientBalance);
            }

            let to_balance = self.balance_of(to);
            let new_to_balance = to_balance.checked_add(amount).ok_or(Error::Overflow)?;
            let new_from_balance = from_balance.checked_sub(amount).ok_or(Error::Overflow)?;

            self.balances.insert(from, &new_from_balance);
            self.balances.insert(to, &new_to_balance);

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value: amount,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, amount: u128) -> Result<()> {
            let owner = self.env().caller();

            if owner == spender {
                return Err(Error::SelfApproval);
            }

            self.not_blacklisted(owner)?;
            self.not_blacklisted(spender)?;

            self.allowances.insert((owner, spender), &amount);

            self.env().emit_event(Approval {
                owner,
                spender,
                value: amount,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.allowances.get((owner, spender)).unwrap_or(0)
        }

        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            amount: u128,
        ) -> Result<()> {
            self.when_not_paused()?;

            let caller = self.env().caller();
            self.not_blacklisted(from)?;
            self.not_blacklisted(to)?;
            self.not_blacklisted(caller)?;

            let current_allowance = self.allowance(from, caller);

            if current_allowance < amount {
                return Err(Error::InsufficientAllowance);
            }

            let new_allowance = current_allowance
                .checked_sub(amount)
                .ok_or(Error::Overflow)?;
            self.allowances.insert((from, caller), &new_allowance);

            self._transfer(from, to, amount)?;

            Ok(())
        }

        #[ink(message)]
        pub fn increase_allowance(&mut self, spender: AccountId, added_value: u128) -> Result<()> {
            let owner = self.env().caller();
            let current_allowance = self.allowance(owner, spender);
            let new_allowance = current_allowance
                .checked_add(added_value)
                .ok_or(Error::Overflow)?;

            self.approve(spender, new_allowance)
        }

        #[ink(message)]
        pub fn decrease_allowance(
            &mut self,
            spender: AccountId,
            subtracted_value: u128,
        ) -> Result<()> {
            let owner = self.env().caller();
            let current_allowance = self.allowance(owner, spender);

            if current_allowance < subtracted_value {
                return Err(Error::InsufficientAllowance);
            }

            let new_allowance = current_allowance
                .checked_sub(subtracted_value)
                .ok_or(Error::Overflow)?;
            self.approve(spender, new_allowance)
        }

        #[ink(message)]
        pub fn batch_transfer(&mut self, recipients: Vec<(AccountId, u128)>) -> Result<()> {
            self.when_not_paused()?;

            let from = self.env().caller();
            self.not_blacklisted(from)?;

            let mut total_amount: u128 = 0;
            for (to, amount) in &recipients {
                self.not_blacklisted(*to)?;
                total_amount = total_amount.checked_add(*amount).ok_or(Error::Overflow)?;
            }

            let from_balance = self.balance_of(from);
            if from_balance < total_amount {
                return Err(Error::InsufficientBalance);
            }

            for (to, amount) in recipients {
                if amount > 0 {
                    self._transfer(from, to, amount)?;
                }
            }

            Ok(())
        }

        #[ink(message)]
        pub fn pause(&mut self) -> Result<()> {
            self.only_owner()?;

            if self.paused {
                return Ok(());
            }

            self.paused = true;

            self.env().emit_event(Paused { paused: true });

            Ok(())
        }

        #[ink(message)]
        pub fn unpause(&mut self) -> Result<()> {
            self.only_owner()?;

            if !self.paused {
                return Ok(());
            }

            self.paused = false;

            self.env().emit_event(Paused { paused: false });

            Ok(())
        }

        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.paused
        }

        #[ink(message)]
        pub fn blacklist(&mut self, account: AccountId) -> Result<()> {
            self.only_owner()?;

            if account == self.owner {
                return Err(Error::Unauthorized);
            }

            self.blacklist.insert(account, &true);

            self.env().emit_event(BlacklistUpdated {
                account,
                blacklisted: true,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn unblacklist(&mut self, account: AccountId) -> Result<()> {
            self.only_owner()?;

            self.blacklist.insert(account, &false);

            self.env().emit_event(BlacklistUpdated {
                account,
                blacklisted: false,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn is_blacklisted(&self, account: AccountId) -> bool {
            self.blacklist.get(account).unwrap_or(false)
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
            self.only_owner()?;

            let old_owner = self.owner;
            self.owner = new_owner;

            self.env().emit_event(OwnershipTransferred {
                previous_owner: old_owner,
                new_owner,
            });

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn get_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        #[ink::test]
        fn test_mint() {
            let mut token = Token::new();
            let accounts = get_accounts();

            assert_eq!(token.balance_of(accounts.bob), 0);
            assert_eq!(token.total_supply(), 0);

            // Mint tokens
            token.mint(accounts.bob, 1000).unwrap();

            assert_eq!(token.balance_of(accounts.bob), 1000);
            assert_eq!(token.total_supply(), 1000);

            // Mint more to same account
            token.mint(accounts.bob, 500).unwrap();
            assert_eq!(token.balance_of(accounts.bob), 1500);
            assert_eq!(token.total_supply(), 1500);
        }

        #[ink::test]
        fn test_burn() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Setup: mint some tokens
            token.mint(accounts.alice, 1000).unwrap();

            // Burn tokens
            token.burn(300).unwrap();

            assert_eq!(token.balance_of(accounts.alice), 700);
            assert_eq!(token.total_supply(), 700);

            // Burn more
            token.burn(200).unwrap();
            assert_eq!(token.balance_of(accounts.alice), 500);
            assert_eq!(token.total_supply(), 500);
        }

        #[ink::test]
        fn test_transfer() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Setup: mint tokens to alice
            token.mint(accounts.alice, 1000).unwrap();

            // Transfer to bob
            token.transfer(accounts.bob, 300).unwrap();

            assert_eq!(token.balance_of(accounts.alice), 700);
            assert_eq!(token.balance_of(accounts.bob), 300);
            assert_eq!(token.total_supply(), 1000); // Total unchanged

            // Transfer to charlie
            token.transfer(accounts.charlie, 200).unwrap();

            assert_eq!(token.balance_of(accounts.alice), 500);
            assert_eq!(token.balance_of(accounts.charlie), 200);
        }

        #[ink::test]
        fn test_approve_and_transfer_from() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Setup: mint tokens to alice
            token.mint(accounts.alice, 1000).unwrap();

            // Alice approves bob to spend 300
            token.approve(accounts.bob, 300).unwrap();
            assert_eq!(token.allowance(accounts.alice, accounts.bob), 300);

            // Bob transfers from alice to charlie
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            token
                .transfer_from(accounts.alice, accounts.charlie, 200)
                .unwrap();

            assert_eq!(token.balance_of(accounts.alice), 800);
            assert_eq!(token.balance_of(accounts.charlie), 200);
            assert_eq!(token.allowance(accounts.alice, accounts.bob), 100); // Decreased
        }

        #[ink::test]
        fn test_increase_decrease_allowance() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Initial approval
            token.approve(accounts.bob, 100).unwrap();
            assert_eq!(token.allowance(accounts.alice, accounts.bob), 100);

            // Increase allowance
            token.increase_allowance(accounts.bob, 50).unwrap();
            assert_eq!(token.allowance(accounts.alice, accounts.bob), 150);

            // Decrease allowance
            token.decrease_allowance(accounts.bob, 30).unwrap();
            assert_eq!(token.allowance(accounts.alice, accounts.bob), 120);
        }

        #[ink::test]
        fn test_pause_and_unpause() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Setup: mint tokens
            token.mint(accounts.alice, 1000).unwrap();

            // Initially not paused
            assert!(!token.is_paused());
            token.transfer(accounts.bob, 100).unwrap();

            // Pause contract
            token.pause().unwrap();
            assert!(token.is_paused());

            // Transfers should fail when paused
            assert_eq!(
                token.transfer(accounts.bob, 100),
                Err(Error::ContractPaused)
            );

            // Unpause contract
            token.unpause().unwrap();
            assert!(!token.is_paused());

            // Transfers should work again
            token.transfer(accounts.bob, 100).unwrap();
            assert_eq!(token.balance_of(accounts.bob), 200);
        }

        #[ink::test]
        fn test_blacklist_and_unblacklist() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Setup: mint tokens
            token.mint(accounts.alice, 1000).unwrap();

            // Blacklist bob
            token.blacklist(accounts.bob).unwrap();
            assert!(token.is_blacklisted(accounts.bob));

            // Transfer to blacklisted address should fail
            assert_eq!(
                token.transfer(accounts.bob, 100),
                Err(Error::AccountBlacklisted)
            );

            // Unblacklist bob
            token.unblacklist(accounts.bob).unwrap();
            assert!(!token.is_blacklisted(accounts.bob));

            // Transfer should work now
            token.transfer(accounts.bob, 100).unwrap();
            assert_eq!(token.balance_of(accounts.bob), 100);
        }

        #[ink::test]
        fn test_blacklisted_sender() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Setup: mint to bob
            token.mint(accounts.bob, 500).unwrap();

            // Blacklist bob
            token.blacklist(accounts.bob).unwrap();

            // Bob cannot send tokens when blacklisted
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                token.transfer(accounts.charlie, 100),
                Err(Error::AccountBlacklisted)
            );
        }

        #[ink::test]
        fn test_batch_transfer() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Setup: mint tokens to alice
            token.mint(accounts.alice, 1000).unwrap();

            // Batch transfer to multiple recipients
            let recipients = ink::prelude::vec![
                (accounts.bob, 100),
                (accounts.charlie, 200),
                (accounts.django, 150),
            ];

            token.batch_transfer(recipients).unwrap();

            assert_eq!(token.balance_of(accounts.alice), 550);
            assert_eq!(token.balance_of(accounts.bob), 100);
            assert_eq!(token.balance_of(accounts.charlie), 200);
            assert_eq!(token.balance_of(accounts.django), 150);
        }

        #[ink::test]
        fn test_batch_transfer_insufficient_balance() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Setup: mint only 200 tokens
            token.mint(accounts.alice, 200).unwrap();

            // Try to batch transfer more than balance
            let recipients = ink::prelude::vec![
                (accounts.bob, 100),
                (accounts.charlie, 150), // Total: 250 > 200
            ];

            assert_eq!(
                token.batch_transfer(recipients),
                Err(Error::InsufficientBalance)
            );

            // Balances should remain unchanged (atomic operation)
            assert_eq!(token.balance_of(accounts.alice), 200);
            assert_eq!(token.balance_of(accounts.bob), 0);
            assert_eq!(token.balance_of(accounts.charlie), 0);
        }

        #[ink::test]
        fn test_batch_transfer_with_blacklisted_recipient() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Setup
            token.mint(accounts.alice, 1000).unwrap();
            token.blacklist(accounts.charlie).unwrap();

            // Batch transfer with blacklisted recipient should fail
            let recipients = ink::prelude::vec![
                (accounts.bob, 100),
                (accounts.charlie, 200), // Blacklisted!
            ];

            assert_eq!(
                token.batch_transfer(recipients),
                Err(Error::AccountBlacklisted)
            );
        }

        #[ink::test]
        fn test_ownership_transfer() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Alice is initial owner
            assert_eq!(token.owner(), accounts.alice);

            // Transfer ownership to bob
            token.transfer_ownership(accounts.bob).unwrap();
            assert_eq!(token.owner(), accounts.bob);

            // Old owner cannot mint
            assert_eq!(token.mint(accounts.charlie, 100), Err(Error::Unauthorized));

            // New owner can mint
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            token.mint(accounts.charlie, 100).unwrap();
            assert_eq!(token.balance_of(accounts.charlie), 100);
        }

        #[ink::test]
        fn test_mint_zero_amount_fails() {
            let mut token = Token::new();
            let accounts = get_accounts();

            assert_eq!(token.mint(accounts.bob, 0), Err(Error::InvalidAmount));
        }

        #[ink::test]
        fn test_burn_zero_amount_fails() {
            let mut token = Token::new();
            let accounts = get_accounts();

            token.mint(accounts.alice, 100).unwrap();
            assert_eq!(token.burn(0), Err(Error::InvalidAmount));
        }

        #[ink::test]
        fn test_transfer_zero_amount_fails() {
            let mut token = Token::new();
            let accounts = get_accounts();

            token.mint(accounts.alice, 100).unwrap();
            assert_eq!(token.transfer(accounts.bob, 0), Err(Error::InvalidAmount));
        }

        #[ink::test]
        fn test_burn_insufficient_balance() {
            let mut token = Token::new();
            let accounts = get_accounts();

            token.mint(accounts.alice, 100).unwrap();
            assert_eq!(token.burn(200), Err(Error::InsufficientBalance));
        }

        #[ink::test]
        fn test_transfer_insufficient_balance() {
            let mut token = Token::new();
            let accounts = get_accounts();

            token.mint(accounts.alice, 100).unwrap();
            assert_eq!(
                token.transfer(accounts.bob, 200),
                Err(Error::InsufficientBalance)
            );
        }

        #[ink::test]
        fn test_transfer_from_insufficient_allowance() {
            let mut token = Token::new();
            let accounts = get_accounts();

            token.mint(accounts.alice, 1000).unwrap();
            token.approve(accounts.bob, 100).unwrap();

            // Bob tries to transfer more than allowance
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                token.transfer_from(accounts.alice, accounts.charlie, 200),
                Err(Error::InsufficientAllowance)
            );
        }

        #[ink::test]
        fn test_self_approval_fails() {
            let mut token = Token::new();
            let accounts = get_accounts();

            assert_eq!(token.approve(accounts.alice, 100), Err(Error::SelfApproval));
        }

        #[ink::test]
        fn test_unauthorized_mint() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Bob tries to mint (not owner)
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(token.mint(accounts.charlie, 100), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_unauthorized_pause() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Bob tries to pause (not owner)
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(token.pause(), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_unauthorized_blacklist() {
            let mut token = Token::new();
            let accounts = get_accounts();

            // Bob tries to blacklist (not owner)
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(token.blacklist(accounts.charlie), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_owner_cannot_be_blacklisted() {
            let mut token = Token::new();
            let accounts = get_accounts();

            assert_eq!(token.blacklist(accounts.alice), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_burn_when_paused_fails() {
            let mut token = Token::new();
            let accounts = get_accounts();

            token.mint(accounts.alice, 1000).unwrap();
            token.pause().unwrap();

            assert_eq!(token.burn(100), Err(Error::ContractPaused));
        }

        #[ink::test]
        fn test_batch_transfer_when_paused_fails() {
            let mut token = Token::new();
            let accounts = get_accounts();

            token.mint(accounts.alice, 1000).unwrap();
            token.pause().unwrap();

            let recipients = ink::prelude::vec![(accounts.bob, 100),];

            assert_eq!(token.batch_transfer(recipients), Err(Error::ContractPaused));
        }

        #[ink::test]
        fn test_mint_to_blacklisted_fails() {
            let mut token = Token::new();
            let accounts = get_accounts();

            token.blacklist(accounts.bob).unwrap();
            assert_eq!(
                token.mint(accounts.bob, 100),
                Err(Error::AccountBlacklisted)
            );
        }

        #[ink::test]
        fn test_empty_batch_transfer() {
            let mut token = Token::new();
            let accounts = get_accounts();

            token.mint(accounts.alice, 1000).unwrap();

            let recipients = ink::prelude::vec![];
            token.batch_transfer(recipients).unwrap();

            // Nothing should change
            assert_eq!(token.balance_of(accounts.alice), 1000);
        }
    }
}
