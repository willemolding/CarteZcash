use zebra_chain::{parameters::Network, transparent};

#[cfg(test)]
mod test;
pub mod write;

// outputs locked with this script are considered burned and can be released on L1
// this script pushed false to the stack so funds can never be spent
pub fn mt_doom() -> transparent::Address {
    transparent::Address::from_pub_key_hash(Network::Mainnet, [0; 20])
}
