solana_program::declare_id!("mTok58Lg4YfcmwqyrDHpf7ogp599WRhzb6PxjaBqAxS");

use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, rent::Rent,
    system_instruction, sysvar::Sysvar,
};
use spl_associated_token_account::instruction::create_associated_token_account;

#[track_caller]
#[inline(always)]
pub fn assert_with_msg(v: bool, err: impl Into<ProgramError>, msg: &str) -> ProgramResult {
    if v {
        Ok(())
    } else {
        let caller = std::panic::Location::caller();
        msg!("{}. \n{}", msg, caller);
        Err(err.into())
    }
}

pub mod accounts;
pub mod instruction;
pub mod token;
use accounts::{Close, InitializeAccount, InitializeMint, MintOrBurn, Transfer};
use instruction::ManagedTokenInstruction;
use token::{burn, close, freeze, initialize_mint, mint_to, thaw, transfer};

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

#[inline]
fn get_freeze_authority_seeds_checked(
    upstream_authority: &Pubkey,
    expected_key: &Pubkey,
) -> Result<Vec<Vec<u8>>, ProgramError> {
    let (freeze_key, seeds) = get_freeze_authority(upstream_authority);
    assert_with_msg(
        expected_key == &freeze_key,
        ProgramError::InvalidInstructionData,
        "Invalid freeze authority",
    )?;
    Ok(seeds)
}

#[inline]
fn get_freeze_authority(upstream_authority: &Pubkey) -> (Pubkey, Vec<Vec<u8>>) {
    let mut seeds = vec![upstream_authority.as_ref().to_vec()];
    let (freeze_key, bump) = Pubkey::find_program_address(
        &seeds.iter().map(|s| s.as_slice()).collect::<Vec<&[u8]>>(),
        &crate::id(),
    );
    seeds.push(vec![bump]);
    (freeze_key, seeds)
}

#[inline]
fn get_mint_authority_seeds_checked(
    mint: &Pubkey,
    expected_key: &Pubkey,
) -> Result<Vec<Vec<u8>>, ProgramError> {
    let (mint_authority_key, seeds) = get_mint_authority(mint);
    assert_with_msg(
        expected_key == &mint_authority_key,
        ProgramError::InvalidInstructionData,
        "Invalid mint authority",
    )?;
    Ok(seeds)
}

#[inline]
fn get_mint_authority(mint: &Pubkey) -> (Pubkey, Vec<Vec<u8>>) {
    let mut seeds = vec![mint.as_ref().to_vec()];
    let (mint_authority_key, bump) = Pubkey::find_program_address(
        &seeds.iter().map(|s| s.as_slice()).collect::<Vec<&[u8]>>(),
        &crate::id(),
    );
    seeds.push(vec![bump]);
    (mint_authority_key, seeds)
}

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = ManagedTokenInstruction::try_from_slice(instruction_data)?;
    match instruction {
        ManagedTokenInstruction::InitializeMint { decimals } => {
            msg!("ManagedTokenInstruction::InitializeMint");
            process_initialize_mint(accounts, decimals)
        }
        ManagedTokenInstruction::InitializeAccount => {
            msg!("ManagedTokenInstruction::InitializeAccount");
            process_initialize_account(accounts)
        }
        ManagedTokenInstruction::Transfer { amount } => {
            msg!("ManagedTokenInstruction::Transfer");
            process_transfer(accounts, amount)
        }
        ManagedTokenInstruction::MintTo { amount } => {
            msg!("ManagedTokenInstruction::MintTo");
            process_mint_to(accounts, amount)
        }
        ManagedTokenInstruction::Burn { amount } => {
            msg!("ManagedTokenInstruction::Burn");
            process_burn(accounts, amount)
        }
        ManagedTokenInstruction::CloseAccount => {
            msg!("ManagedTokenInstruction::CloseAccount");
            process_close(accounts)
        }
    }
}

pub fn process_initialize_mint(accounts: &[AccountInfo], decimals: u8) -> ProgramResult {
    let InitializeMint {
        mint,
        payer,
        upstream_authority,
        system_program,
        token_program,
    } = InitializeMint::load(accounts)?;
    let space = spl_token::state::Mint::LEN;
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint.key,
            Rent::get()?.minimum_balance(space),
            space as u64,
            token_program.key,
        ),
        &[payer.clone(), mint.clone(), system_program.clone()],
    )?;
    let (mint_authority, _) = get_mint_authority(mint.key);
    let (freeze_key, _) = get_freeze_authority(upstream_authority.key);
    initialize_mint(&freeze_key, &mint_authority, mint, token_program, decimals)
}

pub fn process_initialize_account(accounts: &[AccountInfo]) -> ProgramResult {
    let InitializeAccount {
        token_account,
        owner,
        payer,
        upstream_authority,
        freeze_authority,
        mint,
        system_program,
        associated_token_program,
        token_program,
    } = InitializeAccount::load(accounts)?;
    invoke(
        &create_associated_token_account(payer.key, owner.key, mint.key, token_program.key),
        &[
            associated_token_program.clone(),
            payer.clone(),
            owner.clone(),
            token_account.clone(),
            mint.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;
    let seeds = get_freeze_authority_seeds_checked(upstream_authority.key, freeze_authority.key)?;
    freeze(freeze_authority, mint, token_account, token_program, &seeds)
}

pub fn process_transfer(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let Transfer {
        src_account,
        dst_account,
        mint,
        owner,
        upstream_authority,
        freeze_authority,
        token_program,
    } = Transfer::load(accounts)?;
    let seeds = get_freeze_authority_seeds_checked(upstream_authority.key, freeze_authority.key)?;
    thaw(freeze_authority, mint, src_account, token_program, &seeds)?;
    thaw(freeze_authority, mint, dst_account, token_program, &seeds)?;
    transfer(src_account, dst_account, owner, token_program, amount)?;
    freeze(freeze_authority, mint, dst_account, token_program, &seeds)?;
    freeze(freeze_authority, mint, src_account, token_program, &seeds)
}

pub fn process_mint_to(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let MintOrBurn {
        mint,
        token_account,
        owner: mint_authority,
        upstream_authority,
        freeze_authority,
        token_program,
    } = MintOrBurn::load(accounts)?;
    let freeze_authority_seeds =
        get_freeze_authority_seeds_checked(upstream_authority.key, freeze_authority.key)?;
    let mint_authority_seeds = get_mint_authority_seeds_checked(mint.key, mint_authority.key)?;
    thaw(
        freeze_authority,
        mint,
        token_account,
        token_program,
        &freeze_authority_seeds,
    )?;
    mint_to(
        mint,
        token_account,
        mint_authority,
        token_program,
        amount,
        &mint_authority_seeds,
    )?;
    freeze(
        freeze_authority,
        mint,
        token_account,
        token_program,
        &freeze_authority_seeds,
    )
}

pub fn process_burn(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let MintOrBurn {
        mint,
        token_account,
        owner,
        upstream_authority,
        freeze_authority,
        token_program,
    } = MintOrBurn::load(accounts)?;
    let seeds = get_freeze_authority_seeds_checked(upstream_authority.key, freeze_authority.key)?;
    thaw(freeze_authority, mint, token_account, token_program, &seeds)?;
    burn(mint, token_account, owner, token_program, amount)?;
    freeze(freeze_authority, mint, token_account, token_program, &seeds)
}

pub fn process_close(accounts: &[AccountInfo]) -> ProgramResult {
    let Close {
        token_account,
        dst_account,
        mint,
        owner,
        upstream_authority,
        freeze_authority,
        token_program,
    } = Close::load(accounts)?;
    let seeds = get_freeze_authority_seeds_checked(upstream_authority.key, freeze_authority.key)?;
    thaw(freeze_authority, mint, token_account, token_program, &seeds)?;
    close(token_account, dst_account, owner, token_program)
}