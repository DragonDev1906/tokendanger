use web3::types::{Address, U256};

use crate::types::ContractType;

pub mod memory;

pub trait Storage {
    type Error;

    fn get_contract_type(&self, addr: Address) -> Option<ContractType>;
    fn store_contract_type(&mut self, addr: Address, contract_type: ContractType) -> ContractType;
    fn add_unchecked_token(&mut self, addr: Address, token: U256) -> Result<(), Self::Error>;
    fn add_token(&mut self, addr: Address, token: U256, uri: String) -> Result<(), Self::Error>;
    fn token_uri(&self, addr: Address, token: U256) -> Option<&String>;
    fn want_more_uris(&self, addr: Address) -> bool;
}
