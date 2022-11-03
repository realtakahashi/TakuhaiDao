#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

pub use self::dao_contract::{DaoContract, DaoContractRef};

#[openbrush::contract]
pub mod dao_contract {
    use dao_governance_token::dao_governance_token::{ DaoGovernanceTokenRef};
    use dao_psp22::dao_psp22::{ DaoPsp22Ref};
    use dao_psp34::dao_psp34::{ DaoPsp34Ref};
    use ink_prelude::string::{String};
    use ink_prelude::{vec, vec::Vec};
    use ink_storage::traits::{PackedLayout, SpreadLayout};
    use ink_storage::traits::StorageLayout;
    use openbrush::{storage::Mapping};    
    use member_manager::MemberManagerRef;

    #[derive(
        Debug, PartialEq, Eq, scale::Encode, scale::Decode, Clone, SpreadLayout, PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub enum TokenType {
        GovernanceToken,
        Psp22,
        Psp34,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// The Token Does Not Exists.
        TheTokenDoesNotExist,
        /// Invalid Operation.
        InvalidOperation,
        /// Distribution is failure.
        DistributionIsFailure,
        /// Changing Token Status Is Failure.
        ChangingTokenStatusIsFailure,
        /// Withdrawing is Failure.
        WithdrawingIsFailure,
        /// Wrong Csv Data
        WrongCsvData,
        /// Tranfering Contract Balance is Failure
        TransferingContractBalanceIsFailure,
        ThisFunctionCanBeCalledFromDaoManager,
        ThisLuggageDoesNotExists,
        OnlyMemberDoes,
        NotYourLuggage,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(Debug, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout, PartialEq)]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub struct DaoInfo {
        dao_name: String,
        github_url: String,
        description: String,
        is_aproved: bool,
    }

    #[derive(Debug, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout, PartialEq)]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub struct TokenInfo {
        token_type: TokenType,
        token_address: AccountId,
    }

    #[derive(Debug, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout, PartialEq)]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub struct LuggageInfo {
        to_dao_name: String,
        id: u128,
        contents: String,
        to_person_name: String,
        registar_timestamp: u64,
    }

    #[ink(storage)]
    pub struct DaoContract {
        /// token list id => token info
        token_list_for_id: Mapping<u128, TokenInfo>,
        /// token address => token info
        token_list_for_address: Mapping<AccountId, TokenInfo>,
        /// next token list id
        next_token_id: u128,
        /// dao manager account id
        dao_manager_account_id: AccountId,
        /// dao info
        dao_info: DaoInfo,
        /// luggage info list (is not picked up)
        luggage_list_alive: Mapping<u128, LuggageInfo>,
        /// luggage info list (is already picked up)
        luggage_list_dead: Mapping<u128, LuggageInfo>,
        /// luggage_id
        next_luggage_id: u128,
        /// member_manager reference
        member_manager: MemberManagerRef,
    }

    impl DaoContract {
        #[ink(constructor)]
        pub fn new(
            dao_manager_account_id: AccountId,
            dao_name: String,
            github_url: String,
            description: String,
            member_manager: MemberManagerRef,
        ) -> Self {
            Self {
                token_list_for_id: Mapping::default(),
                token_list_for_address: Mapping::default(),
                next_token_id: 0,
                dao_manager_account_id: dao_manager_account_id,
                dao_info: DaoInfo {
                    dao_name: dao_name,
                    github_url: github_url,
                    description: description,
                    is_aproved: false,
                },
                member_manager: member_manager,
                luggage_list_alive: Mapping::default(),
                luggage_list_dead: Mapping::default(),
                next_luggage_id: 0,
            }
        }

        #[ink(message)]
        pub fn change_aproval(&mut self, is_aproved: bool) -> Result<()> {
            if !self._is_calling_from_dao_manager() {
                return Err(Error::ThisFunctionCanBeCalledFromDaoManager);
            };
            self.dao_info.is_aproved = is_aproved;
            Ok(())
        }

        #[ink(message)]
        pub fn get_dao_info(&self) -> DaoInfo {
            self.dao_info.clone()
        }

        #[ink(message)]
        pub fn add_dao_token(
            &mut self,
            token_type: TokenType,
            token_address: AccountId,
        ) -> Result<()> {
            if !self._is_calling_from_dao_manager() {
                return Err(Error::ThisFunctionCanBeCalledFromDaoManager);
            }

            let token_info = TokenInfo {
                token_type: token_type,
                token_address: token_address,
            };
            self.token_list_for_id
                .insert(&self.next_token_id, &token_info.clone());
            self.token_list_for_address
                .insert(&token_address, &token_info.clone());
            self.next_token_id = self.next_token_id + 1;
            Ok(())
        }

        #[ink(message)]
        pub fn get_token_list(&self) -> Vec<TokenInfo> {
            let mut result: Vec<TokenInfo> = Vec::new();
            for i in 0..self.next_token_id {
                match self.token_list_for_id.get(&i) {
                    Some(value) => result.push(value),
                    None => (),
                }
            }
            result
        }

        #[ink(message)]
        pub fn change_token_sales_status(
            &mut self,
            token_address: AccountId,
            is_start: bool,
        ) -> Result<()> {
            if !self._is_calling_from_dao_manager() {
                return Err(Error::ThisFunctionCanBeCalledFromDaoManager);
            };

            let token_info: TokenInfo = match self.token_list_for_address.get(&token_address) {
                Some(value) => value,
                None => return Err(Error::TheTokenDoesNotExist),
            };

            match token_info.token_type {
                TokenType::Psp22 => {
                    let mut instance: DaoPsp22Ref =
                        ink_env::call::FromAccountId::from_account_id(token_address);
                    match instance.change_token_sale_status(is_start) {
                        Ok(()) => (),
                        Err(_e) => return Err(Error::ChangingTokenStatusIsFailure),
                    };
                }
                TokenType::Psp34 => {
                    let mut instance: DaoPsp34Ref =
                        ink_env::call::FromAccountId::from_account_id(token_address);
                    match instance.change_token_sale_status(is_start) {
                        Ok(()) => (),
                        Err(_e) => return Err(Error::ChangingTokenStatusIsFailure),
                    };
                }
                _ => {
                    return Err(Error::InvalidOperation);
                }
            }
            Ok(())
        }

        #[ink(message)]
        pub fn withdraw_token_proceeds(&mut self, token_address: AccountId) -> Result<()> {
            if !self._is_calling_from_dao_manager() {
                return Err(Error::ThisFunctionCanBeCalledFromDaoManager);
            }

            let token_info: TokenInfo = match self.token_list_for_address.get(&token_address) {
                Some(value) => value,
                None => return Err(Error::TheTokenDoesNotExist),
            };

            match token_info.token_type {
                TokenType::Psp22 => {
                    let mut instance: DaoPsp22Ref =
                        ink_env::call::FromAccountId::from_account_id(token_address);
                    match instance.withdraw() {
                        Ok(()) => (),
                        Err(_e) => return Err(Error::WithdrawingIsFailure),
                    };
                }
                TokenType::Psp34 => {
                    let mut instance: DaoPsp34Ref =
                        ink_env::call::FromAccountId::from_account_id(token_address);
                    match instance.withdraw() {
                        Ok(()) => (),
                        Err(_e) => return Err(Error::WithdrawingIsFailure),
                    };
                }
                _ => {
                    return Err(Error::InvalidOperation);
                }
            }
            Ok(())
        }

        #[ink(message)]
        pub fn distribute_governance_token(
            &mut self,
            token_address: AccountId,
            csv_data: String,
        ) -> Result<()> {
            if !self._is_calling_from_dao_manager() {
                return Err(Error::ThisFunctionCanBeCalledFromDaoManager);
            }

            let token_info: TokenInfo = match self.token_list_for_address.get(&token_address) {
                Some(value) => value,
                None => return Err(Error::TheTokenDoesNotExist),
            };

            match token_info.token_type {
                TokenType::GovernanceToken => {
                    let mut instance: DaoGovernanceTokenRef =
                        ink_env::call::FromAccountId::from_account_id(token_address);
                    let lines: Vec<&str> = csv_data.split('?').collect();
                    for line in lines {
                        let part_data: Vec<&str> = line.split('#').collect();
                        if part_data.len() != 2 {
                            return Err(Error::WrongCsvData);
                        }
                        match instance.distribute_token(
                            self._convert_string_to_accountid(part_data[0]),
                            part_data[1].parse::<u128>().unwrap(),
                        ) {
                            Ok(()) => continue,
                            Err(_e) => return Err(Error::DistributionIsFailure),
                        }
                    }
                }
                _ => return Err(Error::InvalidOperation),
            };
            Ok(())
        }

        #[ink(message)]
        #[ink(payable)]
        pub fn donate_to_the_dao(&mut self) -> Result<()> {
            Ok(())
        }

        #[ink(message)]
        pub fn distribute_dao_treasury(&mut self, to: AccountId, amount: Balance) -> Result<()> {
            if !self._is_calling_from_dao_manager() {
                return Err(Error::ThisFunctionCanBeCalledFromDaoManager);
            }

            match self.env().transfer(to, amount) {
                Ok(()) => Ok(()),
                Err(_e) => Err(Error::TransferingContractBalanceIsFailure),
            }
        }

        #[ink(message)]
        pub fn get_contract_balance(&self) -> Balance {
            self.env().balance()
        }

        #[ink(message)]
        pub fn register_luggage_info_of_recipt(&mut self, dao_address:AccountId, luggage_id:u128) -> Result<()> {
            let caller = self.env().caller();
            let member_info = match self.member_manager.get_member_info(dao_address, caller) {
                Some(value) => value,
                None => {
                    ink_env::debug_println!("################### not member error.");
                    return Err(Error::OnlyMemberDoes);
                },
            };
            let luggage_info = match self.luggage_list_alive.get(&luggage_id) {
                Some(value) => value,
                None => return Err(Error::ThisLuggageDoesNotExists),
            };
            if member_info.name != luggage_info.to_person_name {
                return Err(Error::NotYourLuggage);
            }

            self.luggage_list_dead.insert(&luggage_info.id, &luggage_info.clone());
            self.luggage_list_alive.remove(&luggage_info.id);
            Ok(())
        }

        #[ink(message)]
        pub fn register_luggage_info_of_arraival(
            &mut self,
            to_dao_name: String,
            contents: String,
            to_person_name: String,
        ) -> Result<()> {
            if !self._is_calling_from_dao_manager() {
                return Err(Error::ThisFunctionCanBeCalledFromDaoManager);
            }
            let luggage_info = LuggageInfo {
                to_dao_name: to_dao_name,
                id: self.next_luggage_id,
                contents: contents,
                to_person_name: to_person_name,
                registar_timestamp: self.env().block_timestamp(),
            };
            self.luggage_list_alive.insert(&self.next_luggage_id, &luggage_info);
            self.next_luggage_id = self.next_luggage_id + 1;
            Ok(())
        }

        #[inline]
        fn _is_calling_from_dao_manager(&self) -> bool {
            self.env().caller() == self.dao_manager_account_id
        }

        #[inline]
        fn _convert_string_to_accountid(&self, account_str: &str) -> AccountId {
            let mut output = vec![0xFF; 35];
            bs58::decode(account_str).into(&mut output).unwrap();
            let cut_address_vec: Vec<_> = output.drain(1..33).collect();
            let mut array = [0; 32];
            let bytes = &cut_address_vec[..array.len()];
            array.copy_from_slice(bytes);
            let account_id: AccountId = array.into();
            account_id
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// Imports `ink_lang` so we can use `#[ink::test]`.
        use ink_lang as ink;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            let dao_contract = DaoContract::default();
            assert_eq!(dao_contract.get(), false);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn it_works() {
            let mut dao_contract = DaoContract::new(false);
            assert_eq!(dao_contract.get(), false);
            dao_contract.flip();
            assert_eq!(dao_contract.get(), true);
        }
    }
}
