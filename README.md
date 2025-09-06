# Bonsol Batch Airdrop

A zkVM-powered solution using **Bonsol** to efficiently verify multiple Merkle proofs on Solana, reducing gas costs by ??% for batch airdrop claims.

## Problem Statement

Traditional on-chain Merkle proof verification for airdrops is expensive:
- Each proof requires ~20 hash operations (for a tree of depth 20)
- Each hash costs ~30 compute units on Solana
- Processing 50 claims = 30,000+ compute units
- During popular airdrops, this creates network congestion and high costs

## Solution

Using **Bonsol** - a Solana-native verifiable computation framework built on RISC Zero zkVM:
1. Collect multiple airdrop claims (50-100)
2. Deploy zkProgram to Bonsol that verifies all Merkle proofs off-chain
3. Bonsol generates a compact Groth16 proof (~256 bytes)
4. Verify only the ZK proof on Solana (~200k CU for any batch size)

**Result: ??% reduction in on-chain compute costs + constant verification cost regardless of batch size**

## Architecture with Bonsol

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│    Users    │─────►│   Solana    │─────►│   Bonsol    │─────►│  RISC Zero  │
│             │      │   Program   │      │   Network   │      │    zkVM     │
└─────────────┘      └─────────────┘      └─────────────┘      └─────────────┘
     │                      │                    │                     │
     │ Submit claims        │ Request proof     │ Execute zkProgram   │
     └──────────────────────┼────────────────────┼─────────────────────┘
                            │                    │ Generate Groth16
                            ◄────────────────────┤
                            │ Verify on-chain    │
```

### Key Advantages of Bonsol

- **Native Solana Integration**: Direct on-chain verification with <200k CU
- **Groth16 Proofs**: Converts large STARK proofs to compact 256-byte SNARKs
- **Input Commitment**: Enforces hash commitment over inputs to prevent cheating
- **Composability**: Can interact with other Solana programs in same transaction

### Prerequisites

- Rust 1.81.0
- Solana CLI 2.3.8
- Bonsol CLI
- RISC Zero toolchain

### Resources

- [Bonsol Documentation](https://bonsol.gitbook.io/docs)
- [Bonsol GitHub](https://github.com/bonsol-collective/bonsol)
- [RISC Zero Documentation](https://docs.risczero.com)
- [Solana Documentation](https://docs.solana.com)
