use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ContractType {
    Unknown,
    UnknownERC165,
    ERC721 { metadata: bool, enumerable: bool },
    MaybeERC20,
}
