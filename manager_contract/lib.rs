#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

// use ink_lang as ink;
// #[ink::contract]

pub use self::manager_contract::{ManagerContract, ManagerContractRef};

#[openbrush::contract]
pub mod manager_contract {
    use ink_prelude::string::{String, ToString};
    use ink_prelude::vec::Vec;
    use ink_storage::traits::SpreadAllocate;
    use ink_storage::traits::StorageLayout;
    use ink_storage::traits::{PackedLayout, SpreadLayout};
    use openbrush::{contracts::ownable::*, modifiers, storage::Mapping, traits::Storage};

    #[derive(
        Default, Debug, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout, PartialEq,
    )]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub struct MemberInfo {
        name: String,
        member_address: AccountId,
        member_id: u16,
        token_id: u16,
        is_electoral_commissioner: bool,
    }

    #[derive(
        Debug, PartialEq, Eq, scale::Encode, scale::Decode, Clone, SpreadLayout, PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub enum ProposalStatus {
        /// initial value
        None,
        /// proposed
        Proposed,
        /// voting
        Voting,
        /// Finished voting
        FinishedVoting,
        /// running
        Running,
        /// denied
        Denied,
        /// finished
        Finished,
    }

    #[derive(Debug, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout, PartialEq)]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub struct ProposalInfo {
        proposal_id: u128,
        proposer: AccountId,
        title: String,
        outline: String,
        detail: String,
        status: ProposalStatus,
    }

    #[ink(storage)]
    // #[derive(SpreadAllocate)]
    #[derive(SpreadAllocate, Storage, Default)]
    pub struct ManagerContract {
        #[storage_field]
        ownable: ownable::Data,

        /// member function values.
        next_member_id: u16,
        next_no: u16,
        owner: AccountId,
        // ( DAO address , EOA Address ) => MemberInfo
        member_infoes: Mapping<(AccountId, AccountId), MemberInfo>,
        // ( DAO address , member_id ) => MemberInfo
        member_infoes_from_id: Mapping<(AccountId, u16), MemberInfo>,
        // ( DAO address , commissioner_no ) = EOA Address
        electoral_commissioner: Mapping<(AccountId, u16), AccountId>,

        /// proposal function values.

        /// proposal_id
        next_proposal_id: u128,
        /// ( dao address, proposal_id) => proposal info
        proposal_infoes: Mapping<(AccountId, u128), ProposalInfo>,
    }

    impl Ownable for ManagerContract {}

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
        /// The proposal does not exist.
        ProposalDoesNotExist,
        /// The status you are trying to change is invalid.
        InvalidChanging,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl ManagerContract {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|instance: &mut Self| {
                let caller = instance.env().caller();
                instance._init_with_owner(caller);
            })
        }

        /// Functions of Member.

        /// add first member.
        #[ink(message)]
        pub fn add_first_member(
            &mut self,
            _dao_address: AccountId,
            _member_address: AccountId,
            _name: String,
            _token_id: u16,
        ) -> Result<()> {
            let member_list = self.get_member_list(_dao_address);
            if member_list.len() != 0 {
                return Err(Error::NotFirstMember);
            }
            self.inline_add_member(_dao_address, _name, _member_address, _token_id, true);
            Ok(())
        }

        /// add a member.
        #[ink(message)]
        pub fn add_member(
            &mut self,
            _dao_address: AccountId,
            _proposal_id: u128,
            _member_address: AccountId,
            _name: String,
            _token_id: u16,
        ) -> Result<()> {
            //todo:check proposal is valid
            if self.member_infoes.get(&(_dao_address, _member_address)) != None {
                return Err(Error::MemberAlreadyExists);
            }
            self.inline_add_member(_dao_address, _name, _member_address, _token_id, false);
            Ok(())
        }

        /// delete the member.
        #[ink(message)]
        pub fn delete_member(
            &mut self,
            _dao_address: AccountId,
            _proposal_id: u128,
            _member_address: AccountId,
        ) -> Result<()> {
            // todo:check invalid proposal
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
            _proposal_id: u128,
        ) -> Result<()> {
            // todo: check only member
            // todo: check invalid proposal
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
            _proposal_id: u128,
        ) -> Result<()> {
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

        #[inline]
        fn inline_add_member(
            &mut self,
            _dao_address: AccountId,
            _name: String,
            _member_address: AccountId,
            _token_id: u16,
            is_electoral_commissioner: bool,
        ) {
            let member_info = MemberInfo {
                name: _name,
                member_address: _member_address,
                member_id: self.next_member_id,
                token_id: _token_id,
                is_electoral_commissioner: is_electoral_commissioner,
            };

            self.member_infoes
                .insert(&(_dao_address, _member_address), &member_info.clone());
            self.member_infoes_from_id
                .insert(&(_dao_address, self.next_member_id), &member_info.clone());
            self.next_member_id = self.next_member_id + 1;
        }

        #[inline]
        fn modifier_only_member(&self, _dao_address: AccountId) -> bool {
            let caller = self.env().caller();
            match self.member_infoes.get(&(_dao_address, caller)) {
                Some(_value) => true,
                None => false,
            }
        }

        #[inline]
        fn modifier_only_electoral_commissioner(&self, _dao_address: AccountId) -> bool {
            let caller = self.env().caller();
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

        /// We test a simple use case of our contract.
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
