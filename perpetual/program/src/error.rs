use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum PerpetualSwapError {
    #[error("ExpectedAmountMismatch")]
    ExpectedAmountMismatch,
    #[error("InvalidInstruction")]
    InvalidInstruction,
    #[error("AlreadyInUse")]
    AlreadyInUse,
    #[error("ExpectedMint")]
    ExpectedMint,
    #[error("InvalidProgramAddress")]
    InvalidProgramAddress,
    #[error("InvalidOwner")]
    InvalidOwner,
    #[error("DifferentCollateralUsed")]
    DifferentCollateralUsed,
    #[error("InvalidSupply")]
    InvalidSupply,
    #[error("InvalidFreezeAuthority")]
    InvalidFreezeAuthority,
    #[error("IncorrectPoolMint")]
    IncorrectPoolMint,
    #[error("IncorrectTokenProgramId")]
    IncorrectTokenProgramId,
    #[error("InvalidMints")]
    InvalidMints,
    #[error("InvalidAccountKeys")]
    InvalidAccountKeys,
    #[error("WouldBeLiquidated")]
    WouldBeLiquidated,
    #[error("InsufficientMargin")]
    InsufficientMargin,
    #[error("InvalidTransferTime")]
    InvalidTransferTime,
    #[error("ExpectedAccount")]
    ExpectedAccount,
}

impl From<PerpetualSwapError> for ProgramError {
    fn from(e: PerpetualSwapError) -> Self {
        ProgramError::Custom(e as u32)
    }
}