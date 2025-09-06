use airdrop_core::{AirdropClaim, AirdropProof, BatchAirdropInput, BatchAirdropOutput};
use bonsol_interface::instructions::{execute_v1, CallbackConfig, ExecutionConfig, InputRef};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::entrypoint;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::AccountMeta,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

solana_program::declare_id!("2zBRw2sEXvjskx7w1w9hqdFEMZWy7KipQ6jKPfwjpnL6");

const AIRDROP_IMAGE_ID: &str = "800d6ba99bc80c911ef3fb45e60c520fb14fe88d704455e3e775d66356e1c15a";

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct AirdropTree {
    pub merkle_root: [u8; 32],
    pub total_amount: u64,
    pub total_claimed: u64,
    pub claimed_bitmap: Vec<u8>,
}

impl AirdropTree {
    fn new(merkle_root: [u8; 32], total_amount: u64) -> Self {
        Self {
            merkle_root,
            total_amount,
            total_claimed: 0,
            claimed_bitmap: vec![0; BITMAP_SIZE],
        }
    }
}

const MAX_CLAIMS: usize = 10_000;
const BITMAP_SIZE: usize = (MAX_CLAIMS + 7) / 8;
const AIRDROP_TREE_SIZE: usize = 52 + BITMAP_SIZE;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum AirdropInstruction {
    Initialize {
        merkle_root: [u8; 32],
        total_amount: u64,
    },
    RequestAirdrop {
        execution_id: String,
        batch: BatchAirdropInput,
    },
    Callback {
        execution_id: String,
        result: BatchAirdropOutput,
    },
}

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = AirdropInstruction::try_from_slice(instruction_data)?;

    match instruction {
        AirdropInstruction::Initialize {
            merkle_root,
            total_amount,
        } => initialize(program_id, accounts, merkle_root, total_amount),
        AirdropInstruction::RequestAirdrop {
            execution_id,
            batch,
        } => request_airdrop(program_id, accounts, execution_id, batch),
        AirdropInstruction::Callback {
            execution_id,
            result,
        } => callback(accounts, execution_id, result),
    }
}

/// Initialize an airdrop tree account
fn initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    merkle_root: [u8; 32],
    total_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let airdrop_tree_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let rent = Rent::get()?;
    let lamports_required = rent.minimum_balance(AIRDROP_TREE_SIZE);

    invoke(
        &system_instruction::create_account(
            payer.key,
            airdrop_tree_account.key,
            lamports_required,
            AIRDROP_TREE_SIZE as u64,
            program_id,
        ),
        &[
            payer.clone(),
            airdrop_tree_account.clone(),
            system_program.clone(),
        ],
    )?;

    let airdrop_tree = AirdropTree::new(merkle_root, total_amount);

    let mut data = airdrop_tree_account.try_borrow_mut_data()?;
    let serialized = airdrop_tree.try_to_vec()?;
    data[..serialized.len()].copy_from_slice(&serialized);

    Ok(())
}

fn request_airdrop(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    execution_id: String,
    batch: BatchAirdropInput,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let airdrop_tree_account = next_account_info(account_info_iter)?;

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get the airdrop tree data
    let data = airdrop_tree_account.try_borrow_data()?;
    let airdrop_tree_data = AirdropTree::try_from_slice(&data)?;

    if airdrop_tree_data.merkle_root != batch.merkle_root {
        return Err(ProgramError::InvalidInstructionData);
    }

    drop(data);

    let requester = payer;
    let payer = requester;
    let tip_in_lamports = 1000;
    let current_slot = Clock::get()?.slot;
    let expiration = current_slot + 100;
    let inputs = batch.try_to_vec()?;
    let inputs = vec![InputRef::public(&inputs)];

    // Create the Bonsol execution instruction
    let execution_config = ExecutionConfig {
        verify_input_hash: false,
        input_hash: None,
        forward_output: true,
    };

    let bonsol_instruction = execute_v1(
        requester.key,
        payer.key,
        AIRDROP_IMAGE_ID,
        &execution_id,
        inputs,
        tip_in_lamports,
        expiration,
        execution_config,
        Some(CallbackConfig {
            program_id: *program_id,
            instruction_prefix: vec![2],
            extra_accounts: vec![AccountMeta {
                pubkey: *airdrop_tree_account.key,
                is_signer: false,
                is_writable: false,
            }],
        }),
        None, // default prover version
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    invoke(&bonsol_instruction, accounts)?;
    msg!("Bonsol instruction executed successfully");

    Ok(())
}

fn callback(
    accounts: &[AccountInfo],
    execution_id: String,
    result: BatchAirdropOutput,
) -> ProgramResult {
    msg!("Callback received for execution ID: {}", execution_id);

    let account_info_iter = &mut accounts.iter();
    let airdrop_tree_account = next_account_info(account_info_iter)?;

    let data = airdrop_tree_account.try_borrow_data()?;
    let mut airdrop_tree = AirdropTree::try_from_slice(&data)?;
    drop(data);

    if result.merkle_root != airdrop_tree.merkle_root {
        return Err(ProgramError::InvalidInstructionData);
    }

    for claim in result.verified_claims.iter() {
        // Check if already claimed
        let byte_index = (claim.claim_id / 8) as usize;
        let bit_index = (claim.claim_id % 8) as u8;

        if byte_index >= BITMAP_SIZE {
            msg!("Claim ID {} exceeds maximum claims", claim.claim_id);
            return Err(ProgramError::InvalidArgument);
        }

        if airdrop_tree.claimed_bitmap[byte_index] & (1 << bit_index) != 0 {
            msg!("Claim ID {} already claimed", claim.claim_id);
            return Err(ProgramError::InvalidArgument);
        }

        // Mark as claimed
        airdrop_tree.claimed_bitmap[byte_index] |= 1 << bit_index;
    }

    // Update totals
    airdrop_tree.total_claimed = airdrop_tree
        .total_claimed
        .checked_add(result.total_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Verify we haven't exceeded total allocation
    if airdrop_tree.total_claimed > airdrop_tree.total_amount {
        msg!(
            "Total claimed {} exceeds total amount {}",
            airdrop_tree.total_claimed,
            airdrop_tree.total_amount
        );
        return Err(ProgramError::InsufficientFunds);
    }

    // Write updated state back to account
    let mut data = airdrop_tree_account.try_borrow_mut_data()?;
    let serialized = airdrop_tree.try_to_vec()?;
    data[..serialized.len()].copy_from_slice(&serialized);

    msg!(
        "Successfully processed {} claims for total amount: {}",
        result.verified_claims.len(),
        result.total_amount
    );

    // TODO: Implement actual token transfers to recipients
    // This would require:
    // 1. Token program account passed in accounts
    // 2. Source token account (treasury)
    // 3. Destination token accounts for each recipient
    // 4. Transfer instructions for each verified claim

    Ok(())
}
