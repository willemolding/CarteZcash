use orchard::{
    keys::{FullViewingKey, IncomingViewingKey, PreparedIncomingViewingKey, Scope, SpendingKey},
    note::{ExtractedNoteCommitment, Nullifier},
    note_encryption::OrchardDomain,
};
use zcash_note_encryption::{
    try_note_decryption, EphemeralKeyBytes, ShieldedOutput, ENC_CIPHERTEXT_SIZE,
};
use zebra_state::IntoDisk;

pub use zebra_chain::{
    amount::{self, Amount, NonNegative},
    block,
    orchard::Action,
    parameters, serialization, transaction,
    transaction::Memo,
    transparent,
};
pub use zebra_state::SemanticallyVerifiedBlock;

pub mod service;
#[cfg(test)]
mod test;

// outputs send to this address cannot be recovered and are considered burned
pub fn mt_doom_address() -> orchard::Address {
    mt_doom().address_at(0_usize, Scope::External)
}

pub fn mt_doom_ivk() -> IncomingViewingKey {
    mt_doom().to_ivk(Scope::External)
}

fn mt_doom() -> FullViewingKey {
    // TODO: This is a disaster waiting to happen. Anyone can spend from this address rather than no one!
    // We want an address with a known FVK but an known Spending key. This actually should be pretty easy to do
    let sk = SpendingKey::from_bytes([0x0; 32]).unwrap();
    FullViewingKey::from(&sk)
}

// Attempt to decrypt action. It it was encrypted to Mt Doom address, return the amount and memo
pub fn extract_burn_info(action: &Action) -> Option<(Amount<NonNegative>, Memo)> {
    try_note_decryption(
        &OrchardDomain::for_compact_action(&&compact_action_from(action)),
        &PreparedIncomingViewingKey::new(&mt_doom_ivk()),
        &DecryptableAction(action.clone()),
    )
    .map(|(note, _, memo)| {
        let memo = memo[..].try_into().unwrap();
        (Amount::try_from(note.value().inner()).unwrap(), memo)
    })
}

struct DecryptableAction(Action);

impl ShieldedOutput<OrchardDomain, ENC_CIPHERTEXT_SIZE> for DecryptableAction {
    fn ephemeral_key(&self) -> EphemeralKeyBytes {
        EphemeralKeyBytes(self.0.ephemeral_key.into())
    }

    fn cmstar_bytes(&self) -> [u8; 32] {
        self.0.cm_x.into()
    }

    fn enc_ciphertext(&self) -> &[u8; ENC_CIPHERTEXT_SIZE] {
        &self.0.enc_ciphertext.0
    }
}

/// force initialization of the Orchard verifying key.
/// This is an expensive but one-off operation
/// if it isn't forced by falling this function
/// it will be initialized the first time a shielded transaction is verified
pub fn initialize_halo2() {
    lazy_static::initialize(&zebra_consensus::halo2::VERIFYING_KEY);
}

fn compact_action_from(action: &Action) -> orchard::note_encryption::CompactAction {
    orchard::note_encryption::CompactAction::from_parts(
        Nullifier::from_bytes(&action.nullifier.as_bytes()).unwrap(),
        ExtractedNoteCommitment::from_bytes(&action.cm_x.into()).unwrap(),
        EphemeralKeyBytes(action.ephemeral_key.into()),
        action.enc_ciphertext.0[..52].try_into().unwrap(),
    )
}
