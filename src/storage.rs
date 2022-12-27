use web3::types::Address;

use crate::types::ContractType;

pub mod memory;

pub trait Storage {
    fn get_contract_type(&self, addr: Address) -> Option<ContractType>;
    fn store_contract_type(&mut self, addr: Address, contract_type: ContractType) -> ContractType;
}
