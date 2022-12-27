use web3::{
    contract::{Contract, Options},
    types::{Address, U256},
    Transport, Web3,
};

const ERC721_METADATA_ABI: &[u8] = include_bytes!("./erc721_metadata.abi");

#[derive(Debug)]
pub enum Error {
    Ethabi(web3::ethabi::Error),
    Contract(web3::contract::Error),
}

impl From<web3::ethabi::Error> for Error {
    fn from(e: web3::ethabi::Error) -> Self {
        Error::Ethabi(e)
    }
}
impl From<web3::contract::Error> for Error {
    fn from(e: web3::contract::Error) -> Self {
        Error::Contract(e)
    }
}

pub async fn metadata_token_uri<T: Transport>(
    w3: &Web3<T>,
    addr: Address,
    token_id: U256,
) -> Result<String, Error> {
    let contract = Contract::from_json(w3.eth(), addr, ERC721_METADATA_ABI)?;
    let ret: String = contract
        .query("tokenURI", (token_id,), None, Options::default(), None)
        .await?;
    Ok(ret)
}
