//! Tests for the auction smart contract.
use std::collections::BTreeMap;

use cis3_nft_sponsored_txs::{
    ContractBalanceOfQueryParams, ContractBalanceOfQueryResponse, PermitMessage, PermitParam,
};
use concordium_cis2::{
    AdditionalData, BalanceOfQuery, BalanceOfQueryParams, Receiver, TokenAmountU8, TokenIdU32,
    TransferParams,
};
use concordium_smart_contract_testing::*;
use concordium_std::{AccountSignatures, CredentialSignatures, HashSha2256, SignatureEd25519};
use sponsored_tx_enabled_auction::*;

/// The tests accounts.
const ALICE: AccountAddress = AccountAddress([0; 32]);
const ALICE_ADDR: Address = Address::Account(AccountAddress([0; 32]));
const BOB: AccountAddress = AccountAddress([1; 32]);
const BOB_ADDR: Address = Address::Account(AccountAddress([1; 32]));
const CAROL: AccountAddress = AccountAddress([2; 32]);
const DAVE: AccountAddress = AccountAddress([3; 32]);

const SIGNER: Signer = Signer::with_one_key();
const ACC_INITIAL_BALANCE: Amount = Amount::from_ccd(10000);

const DUMMY_SIGNATURE: SignatureEd25519 = SignatureEd25519([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
]);

#[test]
fn test_add_item() {
    let (mut chain, _keypairs, auction_contract_address, _token_contract_address) =
        initialize_chain_and_auction();

    // Create the InitParameter.
    let parameter = AddItemParameter {
        name:        "MyItem".to_string(),
        end:         Timestamp::from_timestamp_millis(1000),
        start:       Timestamp::from_timestamp_millis(5000),
        token_id:    TokenIdU32(1),
        minimum_bid: TokenAmountU8(3),
    };

    let _update = chain
        .contract_update(
            SIGNER,
            ALICE,
            Address::Account(ALICE),
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::from_ccd(0),
                address:      auction_contract_address,
                receive_name: OwnedReceiveName::new_unchecked(
                    "sponsored_tx_enabled_auction.addItem".to_string(),
                ),
                message:      OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
            },
        )
        .expect("Should be able to add Item");

    // Invoke the view entrypoint and check that the tokens are owned by Alice.
    let invoke = chain
        .contract_invoke(ALICE, ALICE_ADDR, Energy::from(10000), UpdateContractPayload {
            amount:       Amount::zero(),
            receive_name: OwnedReceiveName::new_unchecked(
                "sponsored_tx_enabled_auction.view".to_string(),
            ),
            address:      auction_contract_address,
            message:      OwnedParameter::empty(),
        })
        .expect("Invoke view");

    // Check that the tokens are owned by Alice.
    let rv: ReturnParamView = invoke.parse_return_value().expect("View return value");

    assert_eq!(rv, ReturnParamView {
        item_states:   vec![(0, ItemState {
            auction_state:  AuctionState::NotSoldYet,
            highest_bidder: None,
            name:           "MyItem".to_string(),
            end:            Timestamp::from_timestamp_millis(1000),
            start:          Timestamp::from_timestamp_millis(5000),
            token_id:       TokenIdU32(1),
            creator:        ALICE,
            highest_bid:    TokenAmountU8(3),
        })],
        cis2_contract: ContractAddress::new(0, 0),
        counter:       1,
    });

    // Invoke the view entrypoint and check that the tokens are owned by Alice.
    let invoke = chain
        .contract_invoke(ALICE, ALICE_ADDR, Energy::from(10000), UpdateContractPayload {
            amount:       Amount::zero(),
            receive_name: OwnedReceiveName::new_unchecked(
                "sponsored_tx_enabled_auction.viewItemState".to_string(),
            ),
            address:      auction_contract_address,
            message:      OwnedParameter::from_serial(&0u16).expect("Serialize parameter"),
        })
        .expect("Invoke view");

    // Check that the tokens are owned by Alice.
    let rv: ItemState = invoke.parse_return_value().expect("ViewItemState return value");

    assert_eq!(rv, ItemState {
        auction_state:  AuctionState::NotSoldYet,
        highest_bidder: None,
        name:           "MyItem".to_string(),
        end:            Timestamp::from_timestamp_millis(1000),
        start:          Timestamp::from_timestamp_millis(5000),
        token_id:       TokenIdU32(1),
        creator:        ALICE,
        highest_bid:    TokenAmountU8(3),
    });
}

#[test]
fn full_auction_flow_with_cis3_permit_function() {
    let (mut chain, keypairs, auction_contract_address, token_contract_address) =
        initialize_chain_and_auction();

    // Create the InitParameter.
    let parameter = AddItemParameter {
        name:        "MyItem".to_string(),
        end:         Timestamp::from_timestamp_millis(1000),
        start:       Timestamp::from_timestamp_millis(5000),
        token_id:    TokenIdU32(1),
        minimum_bid: TokenAmountU8(0),
    };

    let _update = chain
        .contract_update(
            SIGNER,
            ALICE,
            Address::Account(ALICE),
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::from_ccd(0),
                address:      auction_contract_address,
                receive_name: OwnedReceiveName::new_unchecked(
                    "sponsored_tx_enabled_auction.addItem".to_string(),
                ),
                message:      OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
            },
        )
        .expect("Should be able to add Item");

    let parameter = cis3_nft_sponsored_txs::MintParams {
        owner: concordium_smart_contract_testing::Address::Account(ALICE),
    };

    let _update = chain
        .contract_update(
            SIGNER,
            ALICE,
            Address::Account(ALICE),
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::from_ccd(0),
                address:      token_contract_address,
                receive_name: OwnedReceiveName::new_unchecked("cis3_nft.mint".to_string()),
                message:      OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
            },
        )
        .expect("Should be able to finalize");

    let additional_data = AdditionalDataIndex {
        item_index: 0u16,
    };

    // Check balances in state.
    let balance_of_alice_and_auction_contract =
        get_balances(&chain, auction_contract_address, token_contract_address);

    assert_eq!(balance_of_alice_and_auction_contract.0, [TokenAmountU8(1), TokenAmountU8(0)]);

    // Create input parameters for the `permit` transfer function.
    let transfer = concordium_cis2::Transfer {
        from:     ALICE_ADDR,
        to:       Receiver::Contract(
            auction_contract_address,
            OwnedEntrypointName::new_unchecked("bid".to_string()),
        ),
        token_id: TokenIdU32(1),
        amount:   ContractTokenAmount::from(1),
        data:     AdditionalData::from(to_bytes(&additional_data)),
    };
    let payload = TransferParams::from(vec![transfer]);

    // The `viewMessageHash` function uses the same input parameter `PermitParam` as
    // the `permit` function. The `PermitParam` type includes a `signature` and
    // a `signer`. Because these two values (`signature` and `signer`) are not
    // read in the `viewMessageHash` function, any value can be used and we choose
    // to use `DUMMY_SIGNATURE` and `ALICE` in the test case below.
    let signature_map = BTreeMap::from([(0u8, CredentialSignatures {
        sigs: BTreeMap::from([(0u8, concordium_std::Signature::Ed25519(DUMMY_SIGNATURE))]),
    })]);

    let mut permit_transfer_param = PermitParam {
        signature: AccountSignatures {
            sigs: signature_map,
        },
        signer:    ALICE,
        message:   PermitMessage {
            timestamp:        Timestamp::from_timestamp_millis(10_000_000_000),
            contract_address: ContractAddress::new(0, 0),
            entry_point:      OwnedEntrypointName::new_unchecked("transfer".into()),
            nonce:            0,
            payload:          to_bytes(&payload),
        },
    };

    // Get the message hash to be signed.
    let invoke = chain
        .contract_invoke(BOB, BOB_ADDR, Energy::from(10000), UpdateContractPayload {
            amount:       Amount::zero(),
            address:      token_contract_address,
            receive_name: OwnedReceiveName::new_unchecked("cis3_nft.viewMessageHash".to_string()),
            message:      OwnedParameter::from_serial(&permit_transfer_param)
                .expect("Should be a valid inut parameter"),
        })
        .expect("Should be able to query viewMessageHash");

    let message_hash: HashSha2256 =
        from_bytes(&invoke.return_value).expect("Should return a valid result");

    permit_transfer_param.signature = keypairs.sign_message(&to_bytes(&message_hash));

    // Transfer token with the permit function.
    let _update = chain
        .contract_update(
            Signer::with_one_key(),
            BOB,
            BOB_ADDR,
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::zero(),
                address:      token_contract_address,
                receive_name: OwnedReceiveName::new_unchecked("cis3_nft.permit".to_string()),
                message:      OwnedParameter::from_serial(&permit_transfer_param)
                    .expect("Should be a valid inut parameter"),
            },
        )
        .expect("Should be able to transfer token with permit");

    // Check balances in state.
    let balance_of_alice_and_auction_contract =
        get_balances(&chain, auction_contract_address, token_contract_address);

    assert_eq!(balance_of_alice_and_auction_contract.0, [TokenAmountU8(0), TokenAmountU8(1)]);

    // Invoke the view entrypoint and check that the tokens are owned by Alice.
    let item_state = view_item_state(&chain, auction_contract_address);

    // Check that item is not sold yet.
    assert_eq!(item_state.auction_state, AuctionState::NotSoldYet);

    // Increment the chain time by 100000 milliseconds.
    chain.tick_block_time(Duration::from_millis(100000)).expect("Increment chain time");

    let _update = chain
        .contract_update(
            SIGNER,
            ALICE,
            Address::Account(ALICE),
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::from_ccd(0),
                address:      auction_contract_address,
                receive_name: OwnedReceiveName::new_unchecked(
                    "sponsored_tx_enabled_auction.finalize".to_string(),
                ),
                message:      OwnedParameter::from_serial(&0u16).expect("Serialize parameter"),
            },
        )
        .expect("Should be able to finalize");

    // Invoke the view entrypoint and check that the tokens are owned by Alice.
    let item_state = view_item_state(&chain, auction_contract_address);

    assert_eq!(item_state.auction_state, AuctionState::Sold(ALICE));
}

#[test]
fn full_auction_flow_with_cis3_transfer_function() {
    let (mut chain, _keypair, auction_contract_address, token_contract_address) =
        initialize_chain_and_auction();

    // Create the InitParameter.
    let parameter = AddItemParameter {
        name:        "MyItem".to_string(),
        end:         Timestamp::from_timestamp_millis(1000),
        start:       Timestamp::from_timestamp_millis(5000),
        token_id:    TokenIdU32(1),
        minimum_bid: TokenAmountU8(0),
    };

    let _update = chain
        .contract_update(
            SIGNER,
            ALICE,
            Address::Account(ALICE),
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::from_ccd(0),
                address:      auction_contract_address,
                receive_name: OwnedReceiveName::new_unchecked(
                    "sponsored_tx_enabled_auction.addItem".to_string(),
                ),
                message:      OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
            },
        )
        .expect("Should be able to add Item");

    let parameter = cis3_nft_sponsored_txs::MintParams {
        owner: concordium_smart_contract_testing::Address::Account(ALICE),
    };

    let _update = chain
        .contract_update(
            SIGNER,
            ALICE,
            Address::Account(ALICE),
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::from_ccd(0),
                address:      token_contract_address,
                receive_name: OwnedReceiveName::new_unchecked("cis3_nft.mint".to_string()),
                message:      OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
            },
        )
        .expect("Should be able to finalize");

    // Check balances in state.
    let balance_of_alice_and_auction_contract =
        get_balances(&chain, auction_contract_address, token_contract_address);

    assert_eq!(balance_of_alice_and_auction_contract.0, [TokenAmountU8(1), TokenAmountU8(0)]);

    let additional_data = AdditionalDataIndex {
        item_index: 0u16,
    };

    // Transfer one token from Alice to bid function in auction.
    let transfer_params = TransferParams::from(vec![concordium_cis2::Transfer {
        from:     ALICE_ADDR,
        to:       Receiver::Contract(
            auction_contract_address,
            OwnedEntrypointName::new_unchecked("bid".to_string()),
        ),
        token_id: TokenIdU32(1),
        amount:   TokenAmountU8(1),
        data:     AdditionalData::from(to_bytes(&additional_data)),
    }]);

    let _update = chain
        .contract_update(SIGNER, ALICE, ALICE_ADDR, Energy::from(10000), UpdateContractPayload {
            amount:       Amount::zero(),
            receive_name: OwnedReceiveName::new_unchecked("cis3_nft.transfer".to_string()),
            address:      token_contract_address,
            message:      OwnedParameter::from_serial(&transfer_params).expect("Transfer params"),
        })
        .expect("Transfer tokens");

    // Invoke the view entrypoint and check that the tokens are owned by Alice.
    let item_state = view_item_state(&chain, auction_contract_address);

    // Check that item is not sold yet.
    assert_eq!(item_state.auction_state, AuctionState::NotSoldYet);

    // Check balances in state.
    let balance_of_alice_and_auction_contract =
        get_balances(&chain, auction_contract_address, token_contract_address);

    assert_eq!(balance_of_alice_and_auction_contract.0, [TokenAmountU8(0), TokenAmountU8(1)]);

    // Increment the chain time by 100000 milliseconds.
    chain.tick_block_time(Duration::from_millis(100000)).expect("Increment chain time");

    let _update = chain
        .contract_update(
            SIGNER,
            ALICE,
            Address::Account(ALICE),
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::from_ccd(0),
                address:      auction_contract_address,
                receive_name: OwnedReceiveName::new_unchecked(
                    "sponsored_tx_enabled_auction.finalize".to_string(),
                ),
                message:      OwnedParameter::from_serial(&0u16).expect("Serialize parameter"),
            },
        )
        .expect("Should be able to finalize");

    // Invoke the view entrypoint and check that the tokens are owned by Alice.
    let item_state = view_item_state(&chain, auction_contract_address);

    // Check that item is not sold yet.
    assert_eq!(item_state.auction_state, AuctionState::Sold(ALICE));
}

/// Test a sequence of bids and finalizations:
/// 0. Auction is initialized.
/// 1. Alice successfully bids 1 CCD.
/// 2. Alice successfully bids 2 CCD, highest
/// bid becomes 2 CCD. Alice gets her 1 CCD refunded.
/// 3. Bob successfully bids 3 CCD, highest
/// bid becomes 3 CCD. Alice gets her 2 CCD refunded.
/// 4. Alice tries to bid 3 CCD, which matches the current highest bid, which
/// fails.
/// 5. Alice tries to bid 3.5 CCD, which is below the minimum raise
/// threshold of 1 CCD.
/// 6. Someone tries to finalize the auction before
/// its end time. Attempt fails.
/// 7. Someone tries to bid after the auction has ended (but before it has been
/// finalized), which fails.
/// 8. Dave successfully finalizes the auction after
/// its end time. Carol (the owner of the contract) collects the highest bid
/// amount.
/// 9. Attempts to subsequently bid or finalize fail.
// #[test]
// fn test_multiple_scenarios() {
//     let (mut chain, contract_address) = initialize_chain_and_auction();

//     // 1. Alice successfully bids 1 CCD.
//     let _update_1 = chain
//         .contract_update(
//             SIGNER,
//             ALICE,
//             Address::Account(ALICE),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::from_ccd(1),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.bid".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect("Alice successfully bids 1 CCD");

//     // 2. Alice successfully bids 2 CCD, highest
//     // bid becomes 2 CCD. Alice gets her 1 CCD refunded.
//     let update_2 = chain
//         .contract_update(
//             SIGNER,
//             ALICE,
//             Address::Account(ALICE),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::from_ccd(2),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.bid".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect("Alice successfully bids 2 CCD");
//     // Check that 1 CCD is transferred back to ALICE.
//     assert_eq!(update_2.account_transfers().collect::<Vec<_>>()[..], [(
//         contract_address,
//         Amount::from_ccd(1),
//         ALICE
//     )]);

//     // 3. Bob successfully bids 3 CCD, highest
//     // bid becomes 3 CCD. Alice gets her 2 CCD refunded.
//     let update_3 = chain
//         .contract_update(
//             SIGNER,
//             BOB,
//             Address::Account(BOB),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::from_ccd(3),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.bid".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect("Bob successfully bids 3 CCD");
//     // Check that 2 CCD is transferred back to ALICE.
//     assert_eq!(update_3.account_transfers().collect::<Vec<_>>()[..], [(
//         contract_address,
//         Amount::from_ccd(2),
//         ALICE
//     )]);

//     // 4. Alice tries to bid 3 CCD, which matches the current highest bid, which
//     // fails.
//     let update_4 = chain
//         .contract_update(
//             SIGNER,
//             ALICE,
//             Address::Account(ALICE),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::from_ccd(3),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.bid".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect_err("Alice tries to bid 3 CCD");
//     // Check that the correct error is returned.
//     let rv: BidError = update_4.parse_return_value().expect("Return value is valid");
//     assert_eq!(rv, BidError::BidBelowCurrentBid);

//     // 5. Alice tries to bid 3.5 CCD, which is below the minimum raise threshold of
//     // 1 CCD.
//     let update_5 = chain
//         .contract_update(
//             SIGNER,
//             ALICE,
//             Address::Account(ALICE),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::from_micro_ccd(3_500_000),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.bid".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect_err("Alice tries to bid 3.5 CCD");
//     // Check that the correct error is returned.
//     let rv: BidError = update_5.parse_return_value().expect("Return value is valid");
//     assert_eq!(rv, BidError::BidBelowMinimumRaise);

//     // 6. Someone tries to finalize the auction before
//     // its end time. Attempt fails.
//     let update_6 = chain
//         .contract_update(
//             SIGNER,
//             DAVE,
//             Address::Account(DAVE),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::zero(),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.finalize".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect_err("Attempt to finalize auction before end time");
//     // Check that the correct error is returned.
//     let rv: FinalizeError = update_6.parse_return_value().expect("Return value is valid");
//     assert_eq!(rv, FinalizeError::AuctionStillActive);

//     // Increment the chain time by 1001 milliseconds.
//     chain.tick_block_time(Duration::from_millis(1001)).expect("Increment chain time");

//     // 7. Someone tries to bid after the auction has ended (but before it has been
//     // finalized), which fails.
//     let update_7 = chain
//         .contract_update(
//             SIGNER,
//             DAVE,
//             Address::Account(DAVE),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::from_ccd(10),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.bid".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect_err("Attempt to bid after auction has reached the endtime");
//     // Check that the return value is `BidTooLate`.
//     let rv: BidError = update_7.parse_return_value().expect("Return value is valid");
//     assert_eq!(rv, BidError::BidTooLate);

//     // 8. Dave successfully finalizes the auction after its end time.
//     let update_8 = chain
//         .contract_update(
//             SIGNER,
//             DAVE,
//             Address::Account(DAVE),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::zero(),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.finalize".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect("Dave successfully finalizes the auction after its end time");

//     // Check that the correct amount is transferred to Carol.
//     assert_eq!(update_8.account_transfers().collect::<Vec<_>>()[..], [(
//         contract_address,
//         Amount::from_ccd(3),
//         CAROL
//     )]);

//     // 9. Attempts to subsequently bid or finalize fail.
//     let update_9 = chain
//         .contract_update(
//             SIGNER,
//             ALICE,
//             Address::Account(ALICE),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::from_ccd(1),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.bid".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect_err("Attempt to bid after auction has been finalized");
//     // Check that the return value is `AuctionAlreadyFinalized`.
//     let rv: BidError = update_9.parse_return_value().expect("Return value is valid");
//     assert_eq!(rv, BidError::AuctionAlreadyFinalized);

//     let update_10 = chain
//         .contract_update(
//             SIGNER,
//             ALICE,
//             Address::Account(ALICE),
//             Energy::from(10000),
//             UpdateContractPayload {
//                 amount:       Amount::zero(),
//                 address:      contract_address,
//                 receive_name:
// OwnedReceiveName::new_unchecked("sponsored_tx_enabled_auction.finalize".to_string()),
// message:      OwnedParameter::empty(),             },
//         )
//         .expect_err("Attempt to finalize auction after it has been finalized");
//     let rv: FinalizeError = update_10.parse_return_value().expect("Return value is valid");
//     assert_eq!(rv, FinalizeError::AuctionAlreadyFinalized);
// }

/// Get the `ItemState` at index 0.
fn view_item_state(chain: &Chain, auction_contract_address: ContractAddress) -> ItemState {
    // Invoke the view entrypoint and check that the tokens are owned by Alice.
    let invoke = chain
        .contract_invoke(ALICE, ALICE_ADDR, Energy::from(10000), UpdateContractPayload {
            amount:       Amount::zero(),
            receive_name: OwnedReceiveName::new_unchecked(
                "sponsored_tx_enabled_auction.viewItemState".to_string(),
            ),
            address:      auction_contract_address,
            message:      OwnedParameter::from_serial(&0u16).expect("Serialize parameter"),
        })
        .expect("Invoke view");

    invoke.parse_return_value().expect("BalanceOf return value")
}

/// Get the `TOKEN_1` balances for Alice and the auction contract.
fn get_balances(
    chain: &Chain,
    auction_contract_address: ContractAddress,
    token_contract_address: ContractAddress,
) -> ContractBalanceOfQueryResponse {
    let balance_of_params: ContractBalanceOfQueryParams = BalanceOfQueryParams {
        queries: vec![
            BalanceOfQuery {
                token_id: TokenIdU32(1),
                address:  ALICE_ADDR,
            },
            BalanceOfQuery {
                token_id: TokenIdU32(1),
                address:  Address::from(auction_contract_address),
            },
        ],
    };

    let invoke = chain
        .contract_invoke(ALICE, ALICE_ADDR, Energy::from(10000), UpdateContractPayload {
            amount:       Amount::zero(),
            receive_name: OwnedReceiveName::new_unchecked("cis3_nft.balanceOf".to_string()),
            address:      token_contract_address,
            message:      OwnedParameter::from_serial(&balance_of_params)
                .expect("BalanceOf params"),
        })
        .expect("Invoke balanceOf");

    invoke.parse_return_value().expect("BalanceOf return value")
}

/// Setup auction and chain.
///
/// Carol is the owner of the auction, which ends at `1000` milliseconds after
/// the unix epoch. The 'microCCD per euro' exchange rate is set to `1_000_000`,
/// so 1 CCD = 1 euro.
fn initialize_chain_and_auction() -> (Chain, AccountKeys, ContractAddress, ContractAddress) {
    let mut chain = Chain::builder()
        .micro_ccd_per_euro(
            ExchangeRate::new(1_000_000, 1).expect("Exchange rate is in valid range"),
        )
        .build()
        .expect("Exchange rate is in valid range");

    let rng = &mut rand::thread_rng();

    let keypairs = AccountKeys::singleton(rng);

    let balance = AccountBalance {
        total:  ACC_INITIAL_BALANCE,
        staked: Amount::zero(),
        locked: Amount::zero(),
    };

    // Create some accounts accounts on the chain.
    chain.create_account(Account::new_with_keys(ALICE, balance, (&keypairs).into()));
    chain.create_account(Account::new(BOB, ACC_INITIAL_BALANCE));
    chain.create_account(Account::new(CAROL, ACC_INITIAL_BALANCE));
    chain.create_account(Account::new(DAVE, ACC_INITIAL_BALANCE));

    // Load and deploy the module.
    let module = module_load_v1("../cis3-nft-sponsored-txs/concordium-out/module.wasm.v1")
        .expect("Module exists");
    let deployment = chain.module_deploy_v1(SIGNER, CAROL, module).expect("Deploy valid module");

    // Create the InitParameter.
    let parameter = ContractAddress::new(0, 0);

    // Initialize the auction contract.
    let token = chain
        .contract_init(SIGNER, CAROL, Energy::from(10000), InitContractPayload {
            amount:    Amount::zero(),
            mod_ref:   deployment.module_reference,
            init_name: OwnedContractName::new_unchecked("init_cis3_nft".to_string()),
            param:     OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
        })
        .expect("Initialize auction");

    // Load and deploy the module.
    let module = module_load_v1("concordium-out/module.wasm.v1").expect("Module exists");
    let deployment = chain.module_deploy_v1(SIGNER, CAROL, module).expect("Deploy valid module");

    // Create the InitParameter.
    let parameter = token.contract_address;

    // Initialize the auction contract.
    let init_auction = chain
        .contract_init(SIGNER, CAROL, Energy::from(10000), InitContractPayload {
            amount:    Amount::zero(),
            mod_ref:   deployment.module_reference,
            init_name: OwnedContractName::new_unchecked(
                "init_sponsored_tx_enabled_auction".to_string(),
            ),
            param:     OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
        })
        .expect("Initialize auction");

    (chain, keypairs, init_auction.contract_address, token.contract_address)
}