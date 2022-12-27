use std::{collections::HashMap, fs::File, io::{ErrorKind, self}};
use serde::{Deserialize, Serialize};
use web3::types::Address;
use crate::types::ContractType;

use super::{Storage};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
struct ContractData {
    #[serde(rename = "type")]
    contract_type: ContractType,
}

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    Encoding(serde_json::Error),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Encoding(e)
    }
}

pub struct MemoryStorage<'a> {
    path: &'a str,
    contracts: HashMap<Address, ContractData>
}

impl<'a> MemoryStorage<'a> {
    pub fn new(path: &'a str) -> Result<Self, io::Error> {
        // Read contact types from file or create new hashmap
        let contracts = match File::open(path) {
            Ok(f) => serde_json::from_reader(f)?,
            Err(e) if e.kind() == ErrorKind::NotFound => HashMap::new(),
            Err(e) => return Err(e),
        };

        Ok(Self { path, contracts })
    }

    pub fn persist(&self) -> Result<(), Error> {
        let f = File::create(self.path)?;
        serde_json::to_writer(f, &self.contracts)?;
        Ok(())
    }
}

impl<'a> Storage for MemoryStorage<'a> {
    fn get_contract_type(&self, addr: Address) -> Option<ContractType> {
        Some(self.contracts.get(&addr)?.contract_type)
    }

    fn store_contract_type(&mut self, addr: Address, contract_type: ContractType) -> ContractType {
        let data = ContractData { contract_type };
        self.contracts.entry(addr).or_insert(data).contract_type
    }
}
