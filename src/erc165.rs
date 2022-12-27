use hex_literal::hex;
use web3::{
    types::{Address, Bytes, CallRequest},
    Transport, Web3,
};

/// `supportsInterface(bytes4)` (EIP-165)
const FN_SUPPORTS_INTERFACE: [u8; 4] = hex!("01ffc9a7");

const INTERFACE_ERC165: [u8; 4] = FN_SUPPORTS_INTERFACE;

#[derive(Debug)]
pub enum Error {
    NotSupported,
    Web3Error(web3::Error),
}

pub async fn supports_interface_unchecked<T: Transport>(
    w3: &Web3<T>,
    addr: Address,
    interface_id: &[u8; 4],
) -> Result<bool, Error> {
    let mut data = vec![0u8; 4 + 32];
    data[0..4].copy_from_slice(&FN_SUPPORTS_INTERFACE);
    data[4..8].copy_from_slice(interface_id);

    let req = CallRequest::builder()
        .to(addr)
        .data(data.into())
        .gas(30_000.into())
        .build();
    match w3.eth().call(req, None).await {
        Ok(Bytes(ret)) => {
            // Result must be 32 bytes and only the last bit is allowed to be 1
            if ret.len() != 32 || !ret[..31].eq(&[0; 31]) || ret[31] & 0xfe != 0 {
                Ok(false)
            } else {
                Ok(ret[31] & 1 == 1)
            }
        }
        Err(web3::Error::Rpc(e)) if e.code.code() == -32000 => Ok(false),
        Err(e) => Err(Error::Web3Error(e)),
    }
}

pub async fn supports_interface<T: Transport>(
    w3: &Web3<T>,
    addr: Address,
    interface_id: &[u8; 4],
) -> Result<bool, Error> {
    Ok(is_eip165(w3, addr).await? && supports_interface_unchecked(w3, addr, interface_id).await?)
}

pub async fn is_eip165<T: Transport>(w3: &Web3<T>, addr: Address) -> Result<bool, Error> {
    Ok(
        supports_interface_unchecked(w3, addr, &INTERFACE_ERC165).await?
            && !supports_interface_unchecked(w3, addr, &hex!("ffffffff")).await?,
    )
}

#[macro_export]
macro_rules! make_is_interface {
    ($fn_name:tt, $interface_id:expr) => {
        pub async fn $fn_name<T: Transport>(w3: &Web3<T>, addr: Address) -> Result<bool, Error> {
            supports_interface(w3, addr, &hex!($interface_id)).await
        }
    };
}
#[macro_export]
macro_rules! make_is_interface_unchecked {
    ($fn_name:tt, $interface_id:expr) => {
        pub async fn $fn_name<T: Transport>(w3: &Web3<T>, addr: Address) -> Result<bool, Error> {
            supports_interface_unchecked(w3, addr, &hex!($interface_id)).await
        }
    };
}

make_is_interface!(is_erc721, "80ac58cd");
make_is_interface!(is_erc721metadata, "5b5e139f");
make_is_interface!(is_erc721enumerable, "780e9d63");
make_is_interface_unchecked!(is_erc721_unchecked, "80ac58cd");
make_is_interface_unchecked!(is_erc721metadata_unchecked, "5b5e139f");
make_is_interface_unchecked!(is_erc721enumerable_unchecked, "780e9d63");
