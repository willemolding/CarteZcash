use std::sync::Arc;

use zebra_chain::{block::Block, serialization::ZcashSerialize, transaction::Transaction};

use crate::proto::compact_formats::{
    ChainMetadata, CompactBlock, CompactSaplingOutput, CompactSaplingSpend, CompactTx,
};

// Copied from https://github.com/ZcashFoundation/zebra/blob/4579722833a78843569e4419d0e5dbe6f4041c1b/zebra-scan/src/service/scan_task/scan.rs#L437
// but had to be reimplemented here for our proto generated types

/// Converts a zebra block and meta data into a compact block.
pub fn block_to_compact(block: &Block, chain_metadata: ChainMetadata) -> CompactBlock {
    CompactBlock {
        height: block
            .coinbase_height()
            .expect("verified block should have a valid height")
            .0
            .into(),
        // TODO: performance: look up the block hash from the state rather than recalculating it
        hash: block.hash().bytes_in_display_order().to_vec(),
        prev_hash: block
            .header
            .previous_block_hash
            .bytes_in_display_order()
            .to_vec(),
        time: block
            .header
            .time
            .timestamp()
            .try_into()
            .expect("unsigned 32-bit times should work until 2105"),
        header: block
            .header
            .zcash_serialize_to_vec()
            .expect("verified block should serialize"),
        vtx: block
            .transactions
            .iter()
            .cloned()
            .enumerate()
            .map(transaction_to_compact)
            .collect(),
        chain_metadata: Some(chain_metadata),

        // The protocol version is used for the gRPC wire format, so it isn't needed here.
        proto_version: 0,
    }
}

/// Converts a zebra transaction into a compact transaction.
fn transaction_to_compact((index, tx): (usize, Arc<Transaction>)) -> CompactTx {
    CompactTx {
        index: index
            .try_into()
            .expect("tx index in block should fit in u64"),
        // TODO: performance: look up the tx hash from the state rather than recalculating it
        hash: tx.hash().bytes_in_display_order().to_vec(),

        // `fee` is not checked by the `scan_block` function. It is allowed to be unset.
        // <https://docs.rs/zcash_client_backend/latest/zcash_client_backend/proto/compact_formats/struct.CompactTx.html#structfield.fee>
        fee: 0,

        spends: tx
            .sapling_nullifiers()
            .map(|nf| CompactSaplingSpend {
                nf: <[u8; 32]>::from(*nf).to_vec(),
            })
            .collect(),

        // > output encodes the cmu field, ephemeralKey field, and a 52-byte prefix of the encCiphertext field of a Sapling Output
        //
        // <https://docs.rs/zcash_client_backend/latest/zcash_client_backend/proto/compact_formats/struct.CompactSaplingOutput.html>
        outputs: tx
            .sapling_outputs()
            .map(|output| CompactSaplingOutput {
                cmu: output.cm_u.to_bytes().to_vec(),
                ephemeral_key: output
                    .ephemeral_key
                    .zcash_serialize_to_vec()
                    .expect("verified output should serialize successfully"),
                ciphertext: output
                    .enc_ciphertext
                    .zcash_serialize_to_vec()
                    .expect("verified output should serialize successfully")
                    .into_iter()
                    .take(52)
                    .collect(),
            })
            .collect(),

        // `actions` is not checked by the `scan_block` function.
        actions: vec![],
    }
}
