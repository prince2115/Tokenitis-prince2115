use crate::state::{Token, TransformMetadata};
use crate::tokenitis_instruction::TokenitisInstruction;
use crate::{state::Transform, state::SEED};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_token::instruction::AuthorityType;
use std::collections::BTreeMap;

pub struct CreateTransform<'a> {
    program_id: Pubkey,
    accounts: CreateTransformAccounts<'a>,
    args: CreateTransformArgs,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct CreateTransformArgs {
    pub metadata: TransformMetadata,
    pub inputs: BTreeMap<Pubkey, Token>,
    pub outputs: BTreeMap<Pubkey, Token>,
}

// deserialize accounts instead of storing as account info
struct CreateTransformAccounts<'a> {
    token_program: &'a AccountInfo<'a>,
    state: &'a AccountInfo<'a>,
    initializer: &'a AccountInfo<'a>,
    token_accounts: Vec<&'a AccountInfo<'a>>,
}

impl<'a> CreateTransform<'a> {
    pub fn new(
        program_id: Pubkey,
        accounts: &'a [AccountInfo<'a>],
        args: CreateTransformArgs,
    ) -> Result<Self, ProgramError> {
        let accounts = &mut accounts.iter();

        let token_program = next_account_info(accounts)?;
        let state = next_account_info(accounts)?;
        let initializer = next_account_info(accounts)?;

        let mut token_accounts: Vec<&AccountInfo> = Vec::new();
        for _ in 0..(args.inputs.len() + args.outputs.len()) {
            token_accounts.push(next_account_info(accounts)?)
        }

        Ok(CreateTransform {
            program_id,
            accounts: CreateTransformAccounts {
                token_program,
                state,
                initializer,
                token_accounts,
            },
            args,
        })
    }
}

impl TokenitisInstruction for CreateTransform<'_> {
    fn validate(&self) -> ProgramResult {
        Ok(())
    }

    // input account should be empty token account
    // output account should be an account with entire token supply
    fn execute(&mut self) -> ProgramResult {
        let accounts = &self.accounts;
        let (pda, _nonce) = Pubkey::find_program_address(&[SEED], &self.program_id);

        for token_account in &accounts.token_accounts {
            let change_authority_ix = spl_token::instruction::set_authority(
                accounts.token_program.key,
                token_account.key,
                Some(&pda),
                AuthorityType::AccountOwner,
                accounts.initializer.key,
                &[accounts.initializer.key],
            )?;

            invoke(
                &change_authority_ix,
                &[
                    (*token_account).clone(),
                    accounts.initializer.clone(),
                    accounts.token_program.clone(),
                ],
            )?;
        }

        let state = Transform::deserialize(&mut &**accounts.state.data.borrow())?;
        if state.initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        let state = Transform {
            initialized: true,
            metadata: self.args.metadata.clone(),
            inputs: self.args.inputs.clone(),
            outputs: self.args.outputs.clone(),
        };
        state.serialize(&mut &mut accounts.state.data.borrow_mut()[..])?;

        Ok(())
    }
}