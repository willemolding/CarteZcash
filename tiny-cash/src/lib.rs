use zebra_chain::{parameters::Network, transparent};

#[cfg(test)]
mod test;
pub mod write;

// outputs locked with this script are considered burned and can be released on L1
// this script pushed false to the stack so funds can never be spent
pub fn mt_doom() -> transparent::Address {
    transparent::Address::from_pub_key_hash(Network::Mainnet, [0; 20])
}

/// force initialization of the Orchard verifying key.
/// This is an expensive but one-off operation
/// if it isn't forced by falling this function
/// it will be initialized the first time a shielded transaction is verified
pub fn initialize_halo2() {
    lazy_static::initialize(&zebra_consensus::halo2::VERIFYING_KEY);
}