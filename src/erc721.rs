use hex_literal::hex;
use web3::{
    contract::{Contract, Options},
    types::{Address, Bytes, CallRequest, H256, U256},
    Transport, Web3,
};

const FN_TOKEN_URI: [u8; 4] = hex!("c87b56dd");

// pub async fn metadata_token_uri<T: Transport>(
//     w3: &Web3<T>,
//     addr: Address,
//     token_id: H256,
// ) -> Result<String, web3::Error> {
//     let mut data = vec![0u8; 4 + 32];
//     data[0..4].copy_from_slice(&FN_TOKEN_URI);
//     data[4..].copy_from_slice(&token_id.0);

//     let req = CallRequest::builder()
//         .to(addr)
//         .data(data.into())
//         .gas(300_000.into())
//         .build();
//     let Bytes(ret) = w3.eth().call(req, None).await?;
//     println!("Ret: {:?}", ret);
//     Ok("".into())
// }

const erc721metadata_abi: &[u8] = include_bytes!("./erc721_metadata.abi");

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
    let contract = Contract::from_json(w3.eth(), addr, erc721metadata_abi)?;
    let ret: String = contract
        .query("tokenURI", (token_id,), None, Options::default(), None)
        .await?;
    Ok(ret)
}
