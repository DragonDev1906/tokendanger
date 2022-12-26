use std::{
    collections::{HashMap, HashSet},
    env,
    fs::File,
    io,
    ops::Range,
};

use hex_literal::hex;
use serde::{Deserialize, Serialize, Serializer};
use web3::{
    types::{Address, BlockNumber, Filter, FilterBuilder, Log, H256, U256, U64},
    Transport, Web3,
};

mod erc165;
mod erc721;

const ERC721_TRANSFER_EVENT: H256 = H256(hex!(
    "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
));

#[tokio::main]
async fn main() {
    let infura_key = env::var("INFURA_KEY").expect("environment variable INFURA_KEY not found.");
    let url = format!("https://mainnet.infura.io/v3/{}", infura_key);

    let transport = web3::transports::Http::new(&url).expect("could not connec to infura");
    let w3 = Web3::new(transport);

    // Get latest block height
    // let block_num = w3
    //     .eth()
    //     .block_number()
    //     .await
    //     .expect("Could not receive block height");
    // println!("Block Number: {}", block_num);

    // ERC 165
    // 57f1887a8bf19b14fc0df6fd9b2acc9af147ea85 // ERC-721 ?
    // 0000000000a39bb272e79075ade125fd351887ac // ERC-20 without ERC-165
    // let addr = hex!("0000000000a39bb272e79075ade125fd351887ac").into();
    // let is721 = erc165::is_erc721(&w3, addr).await.unwrap();
    // println!("Is EIP-721: {}", is721);

    // Topic filter
    let count = 2;
    let end = 16249135;
    let start = end - count;
    // let filter = FilterBuilder::default()
    //     .from_block(start.into())
    //     .to_block(end.into())
    //     .topics(Some(vec![ERC721_TRANSFER_EVENT]), None, None, None)
    //     .build();
    // let logs = w3.eth().logs(filter).await.unwrap();
    // // for l in &logs[..100] {
    // //     println!("Log: {:#?}", l);
    // // }

    // let addresses: HashSet<Address> = logs.iter().map(|l| l.address).collect();

    // let mut tokens: Vec<(Address, TransferEvent)> = logs
    //     .iter()
    //     .filter_map(|l| match l.topics.len() {
    //         4 => Some((l.address, TransferEvent::MaybeERC721(l.topics[3]))),
    //         3 => Some((l.address, TransferEvent::MaybeERC20)),
    //         _ => None, // Log doesn't match ERC20 or ERC721 events.
    //     })
    //     .collect();
    // let token_count = tokens.len();
    // tokens.sort_unstable();
    // tokens.dedup();

    // println!("Logs: {}", logs.len());
    // println!("Unique contracts: {}", addresses.len());
    // println!("Tokens: {}", token_count);
    // println!("Unique Tokens: {}", tokens.len());

    task(&w3, start.into()..end.into()).await.unwrap();
}

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
enum TransferEvent {
    MaybeERC721(H256),
    MaybeERC20,
}

#[derive(Debug)]
enum TaskError {
    Web3(web3::Error),
    Erc165(erc165::Error),
}

impl From<web3::Error> for TaskError {
    fn from(e: web3::Error) -> Self {
        TaskError::Web3(e)
    }
}
impl From<erc165::Error> for TaskError {
    fn from(e: erc165::Error) -> Self {
        TaskError::Erc165(e)
    }
}

async fn contract_type<T: Transport>(
    w3: &Web3<T>,
    log: &Log,
) -> Result<ContractType, erc165::Error> {
    let addr = log.address;
    println!("Request type for {}", addr);
    if !erc165::is_eip165(w3, addr).await? {
        return if log.topics.len() == 3 && log.data.0.len() == 32 {
            Ok(ContractType::MaybeERC20)
        } else {
            Ok(ContractType::Unknown)
        };
    }

    if erc165::is_erc721_unchecked(w3, addr).await? {
        Ok(ContractType::ERC721 {
            metadata: erc165::is_erc721metadata_unchecked(w3, addr).await?,
            enumerable: erc165::is_erc721enumerable_unchecked(w3, addr).await?,
        })
    } else {
        Ok(ContractType::UnknownERC165)
    }
}

const PERSISTENCE_PATH: &'static str = "./contracts.json";

async fn task<T: Transport>(w3: &Web3<T>, range: Range<U64>) -> Result<(), TaskError> {
    // ERC721_TRANSFER_EVENT also catches ERC20 Transfers, as the only
    // difference is that the third param is indexed.
    let topic1 = vec![ERC721_TRANSFER_EVENT];
    let filter = FilterBuilder::default()
        .from_block(range.start.into())
        .to_block(range.end.into())
        .topics(Some(topic1), None, None, None)
        .build();
    let logs = w3.eth().logs(filter).await?;

    // Read cached contact type info from file
    let mut contracts = match File::open(PERSISTENCE_PATH) {
        Ok(f) => serde_json::from_reader(f).unwrap(),
        Err(_) => HashMap::new(),
    };

    for log in &logs[..40] {
        // Get type from cache or request it.
        let contract_type = match contracts.get(&log.address) {
            Some(t) => *t,
            None => {
                // We don't have it yet. This should always write to the HashMap, but just
                // in case we do modify it in paralell in the future this throws away the
                // just requested contract type (earliest write first). Both return the same
                // value unless the smart contract has weird behavior.
                let t = contract_type(w3, &log).await?;
                *contracts.entry(log.address).or_insert(t)
            }
        };

        match contract_type {
            ContractType::Unknown => {}
            ContractType::UnknownERC165 => {}
            ContractType::ERC721 { metadata, .. } => {
                // Only relevant if this isn't a token burn. It may still be burned in
                // the future (e.g. if we're processing old blocks), which means getting the
                // metadata will fail, there is nothing we can do about that unless we have
                // an archive node.
                if metadata && log.topics[2].0 != [0u8; 32] {
                    match erc721::metadata_token_uri(
                        w3,
                        log.address,
                        U256::from_big_endian(&log.topics[3].0),
                    )
                    .await
                    {
                        Ok(uri) => {
                            println!("URI: {}", uri);
                        }
                        Err(erc721::Error::Contract(web3::contract::Error::Api(
                            web3::Error::Rpc(e),
                        ))) if e.code.code() == 3 => {
                            println!("tokenUri reverted: {:?}", log);
                        }
                        Err(e) => panic!("Unexpected error: {:?}", e),
                    };
                }
            }
            ContractType::MaybeERC20 => {}
        }

        // println!("Contract type: {:?}", contract_type);
    }

    // Store contact type info back to file
    let f = File::create(PERSISTENCE_PATH).unwrap();
    serde_json::to_writer(f, &contracts).unwrap();

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
enum ContractType {
    Unknown,
    UnknownERC165,
    ERC721 { metadata: bool, enumerable: bool },
    MaybeERC20,
}
