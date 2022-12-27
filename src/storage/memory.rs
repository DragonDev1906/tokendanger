use std::{collections::HashMap, fs::File, io::{ErrorKind, self}};
use serde::{Deserialize, Serialize};
use web3::types::{Address, U256};
use crate::types::ContractType;

use super::{Storage};

const TEMPLATE_MIN_TOKEN_COUNT: usize = 5;

#[derive(Serialize, Deserialize, Debug)]
struct TemplatedTokenUri {
    uri_template: String,
    checked_tokens: Vec<U256>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ContractData {
    /// Type of this contract (the fields below are currently only used for
    /// ERC721 contracts and should probably be part of [ContractType::ERC721])
    #[serde(rename = "type")]
    contract_type: ContractType,
    /// Token URIs that could not be templated. This could be because they're
    /// generated on-the-fly, are random (e.g. because the contract does stupid
    /// stuff) or that just don't have the token ID as part of their url and
    /// thusbe compacted into a template.
    individual_uris: HashMap<U256, String>,
    /// Stores the URI for multiple (usually all) tokens in this contract. This
    /// can only be used if the tokenURI contains the tokenID.
    templated: Vec<TemplatedTokenUri>,
    /// Token IDs that are known to exist (seen a transfer event), but where we
    /// requested the tokenURI (for example because we assumed it to be
    /// equivalent to the templated token uri). Note that inclusion in this list
    /// does not say anything about the real tokenURI, just that we did not
    /// request it and thus may assume a templated tokenURI in templated.
    unchecked_tokens: Vec<U256>,
}

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    Encoding(serde_json::Error),
    UnknownContract,
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
    type Error = Error;

    fn get_contract_type(&self, addr: Address) -> Option<ContractType> {
        Some(self.contracts.get(&addr)?.contract_type)
    }

    fn store_contract_type(&mut self, addr: Address, contract_type: ContractType) -> ContractType {
        let data = ContractData {
            contract_type,
            individual_uris: HashMap::new(),
            templated: Vec::new(),
            unchecked_tokens: Vec::new(),
        };
        self.contracts.entry(addr).or_insert(data).contract_type
    }

    fn add_unchecked_token(&mut self, addr: Address, token: U256) -> Result<(), Error> {
        let data = self.contracts.get_mut(&addr).ok_or(Error::UnknownContract)?;
        data.unchecked_tokens.push(token);
        Ok(())
    }

    fn add_token(&mut self, addr: Address, token: U256, uri: String) -> Result<(), Error> {
        // TODO: For now we always store individual urls, change this to the templated urls where possible.
        let data = self.contracts.get_mut(&addr).ok_or(Error::UnknownContract)?;
        data.individual_uris.insert(token, uri);
        Ok(())
    }

    fn token_uri(&self, addr: Address, token: U256) -> Option<&String> {
        self.contracts[&addr].individual_uris.get(&token)
    }

    fn want_more_uris(&self, addr: Address) -> bool {
        let data = &self.contracts[&addr];
        // In the long run we probably want to get the uri of all tokens.
        // Alternatively it could also be possible to use symbolic execution to
        // know for sure there can't be any other uri (not even in the future).
        // But, especially in the beginning, we don't have to request them all
        // immediately and can (instead store the list of tokens in
        // unchecked_tokens).
        //
        // This may be a bit restrictive. For example if we have two templates
        // we could stop if both templates have enough tokens. However this
        // should probably be a rare case and a single template should catch
        // >99% of the templated cases.
        !(
            data.templated.len() == 1 && 
            data.templated[0].checked_tokens.len() > TEMPLATE_MIN_TOKEN_COUNT
        )
    }
}
