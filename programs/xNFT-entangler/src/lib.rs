use anchor_lang::{
    prelude::*,
    solana_program::{
        program::{invoke},
    }
};
use spl_token::amount_to_ui_amount;
use anchor_spl::{token::{Mint, Token,TokenAccount, Transfer, FreezeAccount, ThawAccount}, associated_token::AssociatedToken};
use xnft::{
    program::Xnft as XNFT,
    state::{Xnft}
};
pub mod utils;
use utils::*;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod x_nft_entangler {
    use super::*;
    pub fn create_entangler<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateEntangler<'info>>,
        _bump: u8,
        _reverse_bump: u8,
        token_a_escrow_bump: u8,
        token_b_escrow_bump: u8,
        price: Option<u64>,
        pays_every_time: bool,
    ) -> Result<()> {
        let treasury_mint = &ctx.accounts.treasury_mint;
        let payer = &ctx.accounts.payer;
        let transfer_authority = &ctx.accounts.transfer_authority;
        let authority = &ctx.accounts.authority;
        let mint_a = &ctx.accounts.mint_a;
        let metadata_a = &ctx.accounts.metadata_a;
        let master_edition_a = &ctx.accounts.master_edition_a;
        let mint_b = &ctx.accounts.mint_b;
        let metadata_b = &ctx.accounts.metadata_b;
        let master_edition_b = &ctx.accounts.master_edition_b;
        let token_a_escrow = &ctx.accounts.xnft_a_escrow;
        let token_b_escrow = &ctx.accounts.xnft_b_escrow;
        let token_b = &ctx.accounts.token_b;
        let entangled_pair = &mut ctx.accounts.xnft_entangler;
        let reverse_entangled_pair = &ctx.accounts.reverse_entangled_xnfts;
        let token_program = &ctx.accounts.token_program;
        let system_program = &ctx.accounts.system_program;
        let rent = &ctx.accounts.rent;


        let xnft_b = &mut ctx.accounts.xnft_b;

        if !reverse_entangled_pair.data_is_empty() {
            return Err(EntanglerError::EntangledPairExists.into());
        }

        entangled_pair.bump = *ctx
            .bumps
            .get("entangled_pair")
            .ok_or(EntanglerError::BumpSeedNotInHashMap)?;
        entangled_pair.escrow_a_bump = token_a_escrow_bump;
        entangled_pair.escrow_b_bump = token_b_escrow_bump;
        entangled_pair.price = price;
        entangled_pair.pays_every_time = pays_every_time;
        entangled_pair.authority = authority.key();
        entangled_pair.master_mint_b = mint_b.key();
        entangled_pair.xnft_a_escrow = token_a_escrow.key();
        entangled_pair.xnft_b_escrow = token_b_escrow.key();
        entangled_pair.treasury_mint = treasury_mint.key();
        entangled_pair.master_mint_a = mint_a.key();

        let edition_option_a = if master_edition_a.data_len() > 0 {
            Some(master_edition_a)
        } else {
            None
        };

        let edition_option_b = if master_edition_b.data_len() > 0 {
            Some(master_edition_b)
        } else {
            None
        };

        let (mint_a_supply, mint_a_decimals) = get_mint_details(&mint_a.to_account_info())?;
        let mint_a_ui_supply = amount_to_ui_amount(mint_a_supply, mint_a_decimals);
        require!(
            mint_a_supply == 1 || mint_a_ui_supply == 1.0,
            EntanglerError::MustHaveSupplyOne
        );

        let (mint_b_supply, mint_b_decimals) = get_mint_details(&mint_b.to_account_info())?;
        let mint_b_ui_supply = amount_to_ui_amount(mint_b_supply, mint_b_decimals);
        require!(
            mint_b_supply == 1 || mint_b_ui_supply == 1.0,
            EntanglerError::MustHaveSupplyOne
        );

        assert_metadata_valid(metadata_a, edition_option_a, &mint_a.key())?;
        assert_metadata_valid(metadata_b, edition_option_b, &mint_b.key())?;

        assert_is_ata(&token_b.to_account_info(), &payer.key(), &mint_b.key())?;

        let mint_a_key = mint_a.key();
        let mint_b_key = mint_b.key();
        let token_a_escrow_seeds = [
            "xnft-entangler".as_bytes(),
            mint_a_key.as_ref(),
            mint_b_key.as_ref(),
            "escrow".as_bytes(),
            "A".as_bytes(),
            &[token_a_escrow_bump],
        ];
        let token_b_escrow_seeds = [
            "xnft-entangler".as_bytes(),
            mint_a_key.as_ref(),
            mint_b_key.as_ref(),
            "escrow".as_bytes(),
            "B".as_bytes(),
            &[token_b_escrow_bump],
        ];

        create_program_token_account_if_not_present(
            token_a_escrow,
            system_program,
            payer,
            token_program,
            &mint_a.to_account_info(),
            &entangled_pair.to_account_info(),
            rent,
            &token_a_escrow_seeds,
            &[],
        )?;

        create_program_token_account_if_not_present(
            token_b_escrow,
            system_program,
            payer,
            token_program,
            &mint_b.to_account_info(),
            &entangled_pair.to_account_info(),
            rent,
            &token_b_escrow_seeds,
            &[],
        )?;

        invoke(
            &spl_token::instruction::transfer(
                token_program.key,
                &token_b.key(),
                &token_b_escrow.key(),
                &transfer_authority.key(),
                &[],
                mint_b_supply,
            )?,
            &[
                token_b.to_account_info(),
                token_b_escrow.to_account_info(),
                token_program.to_account_info(),
                transfer_authority.to_account_info(),
            ],
        )?;
        // the entangled nft which is given to the escrow is suspended for installation.
        xnft_b.suspended = true;

        Ok(())
    }

    pub fn update_entangler<'info>(
        ctx: Context<'_,'_,'_,'info, UpdateEntangler<'info>>,
        price: Option<u64>,
        pays_every_time: bool,
    ) -> Result<()> {
        let new_authority = &ctx.accounts.new_authority;
        let xnft_entangler = &mut ctx.accounts.xnft_entangler;

        xnft_entangler.authority = new_authority.key();
        xnft_entangler.pays_every_time = pays_every_time;
        xnft_entangler.price = price;
        Ok(())
    }


    pub fn swap_xnft(ctx: Context<SwapxNFT>) -> Result<()> {
        let treasury_mint = &ctx.accounts.treasury_mint;
        let payer = &ctx.accounts.payer;
        let payment_account = &ctx.accounts.payment_account;
        let payment_transfer_authority = &ctx.accounts.payment_transfer_authority;
        let token = &ctx.accounts.token;
        let xnft_mint = &ctx.accounts.xnft_mint;
        let replacement_token_metadata = &ctx.accounts.replacement_token_metadata;
        let replacement_token = &ctx.accounts.replacement_token;
        let replacement_xnft_mint = &ctx.accounts.replacement_xnft_mint;
        let transfer_authority = &ctx.accounts.transfer_authority;
        let token_a_escrow = &ctx.accounts.token_a_escrow;
        let token_b_escrow = &ctx.accounts.token_b_escrow;
        let xnft_entangler = &mut ctx.accounts.xnft_entangler;
        let token_program = &ctx.accounts.token_program;
        let system_program = &ctx.accounts.system_program;
        let ata_program = &ctx.accounts.ata_program;
        let rent = &ctx.accounts.rent;

        require!(token.mint == token_mint.key(), EntanglerError::InvalidMint);
        let token_mint_supply = token_mint.supply;
        if token.amount != token_mint_supply {
            return Err(EntanglerError::InvalidTokenAmount.into());
        }
        if replacement_token.data_is_empty() {
            make_ata(
                replacement_token.to_account_info(),
                payer.to_account_info(),
                replacement_xnft_mint.to_account_info(),
                payer.to_account_info(),
                ata_program.to_account_info(),
                token_program.to_account_info(),
                system_program.to_account_info(),
                rent.to_account_info(),
                &[],
            )?;
        }

        assert_is_ata(
            &replacement_token.to_account_info(),
            &payer.key(),
            &replacement_xnft_mint.key(),
        )?;

        let signer_seeds = [
            "xnft-entangler".as_bytes(),
            xnft_entangler.master_mint_a.as_ref(),
            xnft_entangler.master_mint_b.as_ref(),
            &[xnft_entangler.bump],
        ];
        let swap_from_escrow;
        let swap_to_escrow;
        if token.mint == xnft_entangler.master_mint_a {
            swap_from_escrow = token_a_escrow;
            swap_to_escrow = token_b_escrow;
            assert_metadata_valid(replacement_token_metadata, None, &entangled_pair.mint_b)?;
        } else if token.mint == xnft_entangler.master_mint_b {
            swap_from_escrow = token_b_escrow;
            swap_to_escrow = token_a_escrow;
            assert_metadata_valid(replacement_token_metadata, None, &entangled_pair.mint_a)?;
        } else {
            return Err(ErrorCode::InvalidMint.into());
        }

        if replacement_xnft_mint.key() != xnft_entangler.mint_a
            && replacement_xnft_mint.key() != xnft_entangler.mint_b
        {
            return Err(ErrorCode::InvalidMint.into());
        }

        Ok(())

    }
}

#[derive(Accounts)]
#[instruction(_reverse_bump: u8, token_a_escrow_bump: u8, token_b_escrow_bump: u8)]
pub struct CreateEntangler<'info> {
    treasury_mint: Box<Account<'info, Mint>>,
    //mint of the xNFT
    mint_a: Box<Account<'info, Mint>>,
    //xNFT holding token account owned by the publisher
    #[account(mut)]
    token_a: Box<Account<'info, TokenAccount>>,
    //CHECK: verified through CPI to metaplex program
    metadata_a: UncheckedAccount<'info>,
    //CHECK: verified through CPI
    master_edition_a: UncheckedAccount<'info>,
    //mint of the xNFT
    mint_b: Box<Account<'info, Mint>>,
    //xNFT holding token account owned by the publisher
    #[account(mut)]
    token_b: Box<Account<'info, TokenAccount>>,
    //CHECK: verified through CPI
    metadata_b: UncheckedAccount<'info>,
    //CHECK: verified through CPI
    master_edition_b: UncheckedAccount<'info>,
    /// CHECK: Not dangerous. Account seeds checked in constraint.
    #[account(mut,seeds=["xnft-entangler".as_bytes(), mint_a.key().as_ref(), mint_b.key().as_ref(), "escrow".as_bytes(), "A".as_bytes()], bump=token_a_escrow_bump)]
    xnft_a_escrow: UncheckedAccount<'info>,
    /// CHECK: Not dangerous. Account seeds checked in constraint.
    #[account(mut,seeds=["xnft-entangler".as_bytes(), mint_a.key().as_ref(), mint_b.key().as_ref(), "escrow".as_bytes(), "B".as_bytes()], bump=token_b_escrow_bump)]
    xnft_b_escrow: UncheckedAccount<'info>,
    //xNFT pda account of A
    #[account(
        seeds = [
            "xnft".as_bytes(),
            master_edition_a.key().as_ref()
        ],
        seeds::program = xnft_program,
        bump = xnft_a.bump
    )]
    xnft_a: Box<Account<'info, Xnft>>,
    //xNFT pda account of B
    #[account(
        seeds = [
            "xnft".as_bytes(),
             master_edition_b.key().as_ref()
        ],
        seeds::program = xnft_program,
        bump = xnft_b.bump
    )]
    xnft_b: Box<Account<'info, Xnft>>,
    #[account(mut)]
    payer: Signer<'info>,
    transfer_authority: Signer<'info>,

    /// CHECK: Verified through CPI
    authority: UncheckedAccount<'info>,

    #[account(
        init, 
        payer = payer, 
        space = std::mem::size_of::<XNFTentangler>(),
        seeds = [
            "xnft-entangler".as_bytes(),
            xnft_a.to_account_info().key.to_bytes().as_ref(),
            xnft_b.to_account_info().key.to_bytes().as_ref(),
        ],
        bump
     )]
     xnft_entangler: Account<'info, XNFTentangler>,

    /// CHECK: Not dangerous. Account seeds checked in constraint.
    #[account(mut, seeds=["xnft-entangler".as_bytes(),xnft_a.to_account_info().key.to_bytes().as_ref(), xnft_b.to_account_info().key.to_bytes().as_ref() ], bump=_reverse_bump)]
    reverse_entangled_xnfts: UncheckedAccount<'info>,

    token_program: Program<'info, Token>,
    xnft_program: Program<'info, XNFT>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateEntangler<'info>{
    authority: Signer<'info>,
    //CHECK: the current authority can choose anyone for the new authority
    new_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [
            "xnft-entangler".as_bytes(), 
            xnft_entangler.xnft_a.as_ref(),
            xnft_entangler.xnft_b.as_ref(),
        ],
        bump = xnft_entangler.bump
    )]
    xnft_entangler: Account<'info, XNFTentangler>,
}

#[derive(Accounts)]
pub struct SwapxNFT<'info>{
    treasury_mint: Box<Account<'info, Mint>>,
    payer: Signer<'info>,
    //CHECK: verified through CPI
    #[account(mut)]
    payment_account: UncheckedAccount<'info>,
    /// CHECK: Verified through CPI
    payment_transfer_authority: UncheckedAccount<'info>,
    #[account(mut)]
    token: Account<'info, TokenAccount>,
    xnft_mint: Box<Account<'info, Mint>>,
    /// CHECK: Verified through CPI
    replacement_token_metadata: UncheckedAccount<'info>,
    replacement_xnft_mint: Box<Account<'info, Mint>>,
    /// CHECK: Verified through CPI
    #[account(mut)]
    replacement_token: UncheckedAccount<'info>,
    transfer_authority: Signer<'info>,
    /// CHECK: Not dangerous. Account seeds checked in constraint.
    #[account(mut,seeds=["xnft-entangler".as_bytes(), xnft_entangler.master_mint_a.as_ref(), xnft_entangler.master_mint_b.as_ref(), "escrow".as_bytes(), "A".as_bytes()], bump=xnft_entangler.escrow_a_bump)]
    token_a_escrow: UncheckedAccount<'info>,
    /// CHECK: Not dangerous. Account seeds checked in constraint.
    #[account(mut,seeds=["xnft-entangler".as_bytes(), xnft_entangler.master_mint_a.as_ref(), xnft_entangler.master_mint_b.as_ref(), "escrow".as_bytes(), "B".as_bytes()], bump=xnft_entangler.escrow_b_bump)]
    token_b_escrow: UncheckedAccount<'info>,
    #[account(mut, seeds=["xnft-entangler".as_bytes(), xnft_entangler.master_mint_a.as_ref(), xnft_entangler.master_mint_b.as_ref()], bump=xnft_entangler.bump, has_one=treasury_mint)]
    xnft_entangler: Account<'info, XNFTentangler>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    ata_program: Program<'info, AssociatedToken>,
    rent: Sysvar<'info, Rent>,
}


#[account]
#[derive(Debug)]
pub struct XNFTentangler{
    treasury_mint: Pubkey,
    xnft_a: Pubkey,
    xnft_b: Pubkey,
    master_mint_a: Pubkey,
    master_mint_b: Pubkey,
    xnft_a_escrow: Pubkey,
    xnft_b_escrow: Pubkey,
    authority: Pubkey,
    bump: u8,
    escrow_a_bump: u8,
    escrow_b_bump: u8,
    price: Option<u64>,
    paid: bool,
    pays_every_time: bool
}

#[error_code]
pub enum EntanglerError{
    #[msg("PublicKeyMismatch")]
    PublicKeyMismatch,
    #[msg("InvalidMintAuthority")]
    InvalidMintAuthority,
    #[msg("UninitializedAccount")]
    UninitializedAccount,
    #[msg("IncorrectOwner")]
    IncorrectOwner,
    #[msg("PublicKeysShouldBeUnique")]
    PublicKeysShouldBeUnique,
    #[msg("StatementFalse")]
    StatementFalse,
    #[msg("NotRentExempt")]
    NotRentExempt,
    #[msg("NumericalOverflow")]
    NumericalOverflow,
    #[msg("Derived key invalid")]
    DerivedKeyInvalid,
    #[msg("Metadata doesn't exist")]
    MetadataDoesntExist,
    #[msg("Edition doesn't exist")]
    EditionDoesntExist,
    #[msg("Invalid token amount")]
    InvalidTokenAmount,
    #[msg("This token is not a valid mint for this entangled pair")]
    InvalidMint,
    #[msg("This pair already exists as it's reverse")]
    EntangledPairExists,
    #[msg("Must have supply one!")]
    MustHaveSupplyOne,
    #[msg("Bump seed not in hash map")]
    BumpSeedNotInHashMap,
}