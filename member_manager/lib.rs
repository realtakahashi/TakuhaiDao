#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

// use ink_lang as ink;
// #[ink::contract]

pub use self::member_manager::{MemberManager, MemberManagerRef};

#[openbrush::contract]
pub mod member_manager {
    use ink_env::debug_println;
    use ink_prelude::string::{String,ToString};
    use ink_prelude::vec;
    use ink_prelude::vec::Vec;
    use ink_storage::traits::SpreadAllocate;
    use ink_storage::traits::StorageLayout;
    use ink_storage::traits::{PackedLayout, SpreadLayout};
    use openbrush::contracts::ownable::OwnableError;
    use openbrush::{contracts::ownable::*, modifiers, storage::Mapping, traits::Storage};
    use rustc_hex::FromHex;

    #[derive(
        Default, Debug, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout, PartialEq)]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub struct MemberInfo {
        name: String,
        member_address: AccountId,
        member_id: u16,
        token_id: u16,
        is_electoral_commissioner: bool,
    }

    #[ink(storage)]
    // #[derive(SpreadAllocate)]
    #[derive(SpreadAllocate, Storage, Default)]
    pub struct MemberManager {
        #[storage_field]
        ownable: ownable::Data,
        next_no: u16,
        next_member_id: u16,
        proposal_manager_address:AccountId,
        owner: AccountId,
        // ( DAO address , EOA Address ) => MemberInfo
        member_infoes: Mapping<(AccountId, AccountId), MemberInfo>,
        // ( DAO address , member_id ) => MemberInfo
        member_infoes_from_id: Mapping<(AccountId, u16), MemberInfo>,
        // ( DAO address , commissioner_no ) = EOA Address
        electoral_commissioner: Mapping<(AccountId, u16), AccountId>,
    }

    impl Ownable for MemberManager {}

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Not first member
        NotFirstMember,
        /// Target member does not exist.
        MemberDoesNotExist,
        /// Target member already exists.
        MemberAlreadyExists,
        /// Electoral Commissioner Data is mismatched.
        ElectoralCommissionerDataMismatch,
        /// Only Member does.
        OnlyMemberDoes,
        /// Only Electoral Commissioner
        OnlyElectoralCommissioner,
        /// Only Proposal Manager Address call this function.
        OnlyFromProposalManagerAddress,
        /// Csv Convert Failure
        CsvConvertFailure,
    }

    pub type ResultTransaction<T> = core::result::Result<T, Error>;
    pub type ResultOwner<T> = core::result::Result<T, OwnableError>;

    impl MemberManager {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|instance: &mut Self| {
                let caller = instance.env().caller();
                instance._init_with_owner(caller);
            })
        }

        /// set proposal manager address
        #[ink(message)]
        #[modifiers(only_owner)]
        pub fn set_propsal_manager_adress(&mut self, _proposal_manager_address:AccountId) -> ResultOwner<()> {
            self.proposal_manager_address = _proposal_manager_address;
            Ok(())
        }

        /// add first member.
        #[ink(message)]
        pub fn add_first_member(
            &mut self,
            _dao_address: AccountId,
            _member_address: AccountId,
            _name: String,
            _token_id: u16,
        ) -> ResultTransaction<()> {
            if  self.get_member_list(_dao_address).len() != 0 {
                return Err(Error::NotFirstMember);
            }
            self.inline_add_member(_dao_address, _name, _member_address, _token_id, true);
            Ok(())
        }

        /// add a member
        #[ink(message)]
        pub fn add_member(&mut self, _dao_address:AccountId, _csv_data:String) -> ResultTransaction<()> {
            if self.modifier_only_call_from_proposal_manager() == false {
                return Err(Error::OnlyFromProposalManagerAddress);
            }

            // ink_env::debug_println!("########## pass add_member #first position");

            let member_info = match self.inline_convert_csv_2_memberinfo(_csv_data) {
                Some(value) => value,
                None => return Err(Error::CsvConvertFailure),
            };
            if self.member_infoes.get(&(_dao_address, member_info.member_address)) != None {
                return Err(Error::MemberAlreadyExists);
            }
            self.inline_add_member(_dao_address, member_info.name, member_info.member_address, member_info.token_id, false);
            Ok(())
        }

        /// delete the member.
        #[ink(message)]
        pub fn delete_member(
            &mut self,
            _dao_address: AccountId,
            _member_address: AccountId,
        ) -> ResultTransaction<()> {
            if self.modifier_only_call_from_proposal_manager() == false {
                return Err(Error::OnlyFromProposalManagerAddress);
            }
            let member_info = match self.member_infoes.get(&(_dao_address, _member_address)) {
                Some(value) => value,
                None => return Err(Error::MemberDoesNotExist),
            };
            for i in 0..self.next_no {
                let electoral_commissioner_address: AccountId =
                    match self.electoral_commissioner.get(&(_dao_address, i)) {
                        Some(value) => value,
                        None => return Err(Error::ElectoralCommissionerDataMismatch),
                    };
                if electoral_commissioner_address == member_info.member_address {
                    self.electoral_commissioner.remove(&(_dao_address, i));
                }
            }
            self.member_infoes_from_id
                .remove(&(_dao_address, member_info.member_id));
            self.member_infoes.remove(&(_dao_address, _member_address));
            Ok(())
        }

        /// get member list.
        #[ink(message)]
        pub fn get_member_list(&self, _dao_address: AccountId) -> Vec<MemberInfo> {
            let mut member_list: Vec<MemberInfo> = Vec::new();
            for i in 0..self.next_member_id {
                let member_info = match self.member_infoes_from_id.get(&(_dao_address, i)) {
                    Some(value) => value,
                    None => continue,
                };
                member_list.push(member_info.clone());
            }
            member_list
        }

        /// add electoral commissioner.
        #[ink(message)]
        pub fn add_electoral_commissioner(
            &mut self,
            _dao_address: AccountId,
            _member_address: AccountId,
        ) -> ResultTransaction<()> {
            if self.modifier_only_call_from_proposal_manager() == false {
                return Err(Error::OnlyFromProposalManagerAddress);
            }
            let mut member_info: MemberInfo =
                match self.member_infoes.get(&(_dao_address, _member_address)) {
                    Some(value) => value,
                    None => return Err(Error::MemberDoesNotExist),
                };
            self.electoral_commissioner
                .insert(&(_dao_address, self.next_no), &_member_address);
            self.next_no = self.next_no + 1;

            member_info.is_electoral_commissioner = true;
            self.member_infoes
                .insert(&(_dao_address, _member_address), &member_info.clone());
            self.member_infoes_from_id
                .insert(&(_dao_address, member_info.member_id), &member_info.clone());

            Ok(())
        }

        /// dismiss electoral commissioner.
        #[ink(message)]
        pub fn dismiss_electoral_commissioner(
            &mut self,
            _dao_address: AccountId,
        ) -> ResultTransaction<()> {
            if self.modifier_only_call_from_proposal_manager() == false {
                return Err(Error::OnlyFromProposalManagerAddress);
            }
            for i in 0..self.next_no {
                let member_address = match self.electoral_commissioner.get(&(_dao_address, i)) {
                    Some(value) => value,
                    None => return Err(Error::ElectoralCommissionerDataMismatch),
                };
                let mut member_info = match self.member_infoes.get(&(_dao_address, member_address))
                {
                    Some(value) => value,
                    None => return Err(Error::ElectoralCommissionerDataMismatch),
                };
                member_info.is_electoral_commissioner = false;
                self.member_infoes.insert(
                    &(_dao_address, member_info.member_address),
                    &member_info.clone(),
                );
                self.member_infoes_from_id
                    .insert(&(_dao_address, member_info.member_id), &member_info.clone());

                self.electoral_commissioner.remove(&(_dao_address, i));
            }
            Ok(())
        }

        /// modifier of only member 
        #[ink(message)]
        pub fn modifier_only_member(&self, caller: AccountId, _dao_address: AccountId) -> bool {
            match self.member_infoes.get(&(_dao_address, caller)) {
                Some(_value) => true,
                None => false,
            }
        }

        /// modifier of only electoral commissioner
        #[ink(message)]
        pub fn modifier_only_electoral_commissioner(
            &self,
            caller: AccountId,
            _dao_address: AccountId,
        ) -> bool {
            for i in 0..self.next_no {
                match self.electoral_commissioner.get(&(_dao_address, i)) {
                    Some(value) => {
                        if value == caller {
                            return true;
                        }
                    }
                    None => return false,
                };
            }
            false
        }

        #[inline]
        fn convert_string_to_accountid(&self, account_str: &str)-> AccountId{
            let mut output = vec![0xFF; 35];
            bs58::decode(account_str).into(&mut output).unwrap();
            let cut_address_vec:Vec<_> = output.drain(1..33).collect();
            let mut array = [0; 32];
            let bytes = &cut_address_vec[..array.len()]; 
            array.copy_from_slice(bytes);
            let accountId:AccountId = array.into();
            accountId
        }

        #[inline]
        fn inline_convert_csv_2_memberinfo(&self, _csv_data:String)->Option<MemberInfo>{
            let _array:Vec<&str> = _csv_data.split(',').collect();
            if _array.len() != 5 {
                return None;
            };

            Some(MemberInfo{
                name: _array[0].to_string(),
                member_address: self.convert_string_to_accountid(_array[1]),
                member_id: _array[2].parse::<u16>().unwrap(),
                token_id: _array[3].parse::<u16>().unwrap(),
                is_electoral_commissioner:false,
            })
        }

        #[inline]
        fn inline_add_member(
            &mut self,
            _dao_address: AccountId,
            _name: String,
            _member_address: AccountId,
            _token_id: u16,
            _is_electoral_commissioner: bool,
        ) {
            let member_info = MemberInfo {
                name: _name,
                member_address: _member_address,
                member_id: self.next_member_id,
                token_id: _token_id,
                is_electoral_commissioner: _is_electoral_commissioner,
            };

            self.member_infoes
                .insert(&(_dao_address, _member_address), &member_info.clone());
            self.member_infoes_from_id
                .insert(&(_dao_address, self.next_member_id), &member_info.clone());
            self.next_member_id = self.next_member_id + 1;

            if _is_electoral_commissioner {
                self.electoral_commissioner
                    .insert(&(_dao_address, self.next_no), &_member_address);
                self.next_no = self.next_no + 1;
            }
        }


        #[inline]
        fn modifier_only_call_from_proposal_manager(&self) -> bool {
            self.env().caller() == self.proposal_manager_address
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

        #[ink::test]
        fn add_member_works() {
            let mut manager_contract = ManagerContract::new();
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            // normal adding a member
            let _res = manager_contract.add_member(
                accounts.frank,
                0,
                accounts.alice,
                "alice".to_string(),
                0,
            );
            let member_info_list = manager_contract.get_member_list(accounts.frank);
            assert_eq!(member_info_list[0].name, "alice");
            assert_eq!(member_info_list[0].member_address, accounts.alice);
            assert_eq!(member_info_list[0].is_electoral_commissioner, false);

            // normal adding two members
            let _res =
                manager_contract.add_member(accounts.frank, 1, accounts.bob, "bob".to_string(), 1);
            let member_info_list = manager_contract.get_member_list(accounts.frank);
            assert_eq!(member_info_list[1].name, "bob");
            assert_eq!(member_info_list[1].member_address, accounts.bob);
            assert_eq!(member_info_list[1].is_electoral_commissioner, false);

            // duplicated adding
            match manager_contract.add_member(
                accounts.frank,
                0,
                accounts.alice,
                "alice".to_string(),
                0,
            ) {
                Ok(()) => panic!("This is not expected path."),
                Err(error) => assert_eq!(error, Error::MemberAlreadyExists),
            }
        }

        #[ink::test]
        fn delete_member_works() {
            let mut manager_contract = ManagerContract::new();
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // deleting non-existed member
            match manager_contract.delete_member(accounts.frank, 0, accounts.alice) {
                Ok(()) => panic!("This is not expected path."),
                Err(error) => assert_eq!(error, Error::MemberDoesNotExist),
            };
            // deleting existed member
            let _res = manager_contract.add_member(
                accounts.frank,
                0,
                accounts.alice,
                "alice".to_string(),
                0,
            );
            let _res =
                manager_contract.add_member(accounts.frank, 1, accounts.bob, "bob".to_string(), 1);
            match manager_contract.delete_member(accounts.frank, 1, accounts.bob) {
                Ok(()) => {
                    let member_list = manager_contract.get_member_list(accounts.frank);
                    assert_eq!(1, member_list.len());
                    assert_eq!(accounts.alice, member_list[0].member_address);
                }
                Err(_error) => panic!("This is not expected path."),
            }
        }
    }
}
