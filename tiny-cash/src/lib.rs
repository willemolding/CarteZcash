use orchard::{
    keys::{FullViewingKey, IncomingViewingKey, PreparedIncomingViewingKey, Scope},
    note::{ExtractedNoteCommitment, Nullifier},
    note_encryption::OrchardDomain,
};
use zcash_note_encryption::{
    try_note_decryption, EphemeralKeyBytes, ShieldedOutput, ENC_CIPHERTEXT_SIZE,
};
use zebra_chain::{
    amount::{Amount, NonNegative},
    orchard::Action,
    transaction::Memo,
};

#[cfg(test)]
mod test;
pub mod write;

// outputs send to this address cannot be recovered and are considered burned
pub fn mt_doom_address() -> orchard::Address {
    FullViewingKey::from_bytes(&[0_u8; 96])
        .unwrap()
        .address_at(0_usize, Scope::External)
}

pub fn mt_doom_ivk() -> IncomingViewingKey {
    FullViewingKey::from_bytes(&[0_u8; 96])
        .unwrap()
        .to_ivk(Scope::External)
}

// Attempt to decrypt action. It it was encrypted to Mt Doom address, return the amount and memo
pub fn extract_burn_info(action: &Action) -> Option<(Amount<NonNegative>, Memo)> {
    try_note_decryption(
        &OrchardDomain::for_compact_action(&dummy_action()),
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

fn dummy_action() -> orchard::note_encryption::CompactAction {
    orchard::note_encryption::CompactAction::from_parts(
        Nullifier::from_bytes(&[0_u8; 32]).unwrap(),
        ExtractedNoteCommitment::from_bytes(&[0_u8; 32]).unwrap(),
        EphemeralKeyBytes([0_u8; 32]),
        [0_u8; 52],
    )
}
