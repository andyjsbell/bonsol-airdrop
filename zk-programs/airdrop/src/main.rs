use airdrop_core::{verify_airdrop_proof, BatchAirdropInput, BatchAirdropOutput, Hasher};
use risc0_zkvm::{
    guest::env,
    sha::{Impl, Sha256},
};

struct Risc0Hasher;

impl Hasher for Risc0Hasher {
    fn hash_bytes(data: &[u8]) -> [u8; 32] {
        let hasher = Impl::hash_bytes(data);
        hasher.as_bytes().try_into().unwrap()
    }
}

fn main() {
    // Read batch from host
    let input: BatchAirdropInput = env::read();

    // Track verified claims and total amount
    let mut verified_claims = Vec::new();
    let mut total_amount = 0u64;

    for proof in &input.proofs {
        if verify_airdrop_proof::<Risc0Hasher>(&input.merkle_root, proof) {
            total_amount = total_amount
                .checked_add(proof.claim.amount)
                .expect("Amount overflow");

            verified_claims.push(proof.claim.clone());
        } else {
            // TODO - track failed proofs too
            println!("Failed claim {:?}", proof.claim);
        }
    }

    // Output the results
    let output = BatchAirdropOutput {
        merkle_root: input.merkle_root,
        total_amount,
        verified_claims,
    };

    env::commit(&output);
}
