// Copyright 2018 Commonwealth Labs, Inc.
// This file is part of Edgeware.

// Edgeware is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Edgeware is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Edgeware.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate serde;

// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
#[macro_use]
extern crate hex_literal;
#[macro_use] extern crate parity_codec_derive;
#[macro_use] extern crate srml_support;


extern crate parity_codec as codec;
extern crate substrate_primitives as primitives;
#[cfg_attr(not(feature = "std"), macro_use)]
extern crate sr_std as rstd;
extern crate srml_support as runtime_support;
extern crate sr_primitives as runtime_primitives;
extern crate sr_io as runtime_io;

extern crate srml_balances as balances;
extern crate srml_system as system;

use rstd::prelude::*;
use runtime_support::dispatch::Result;
use primitives::ed25519;

pub mod delegation;
use delegation::{Module, Trait, RawEvent};

// Tests for Delegation Module
#[cfg(test)]
mod tests {
    use super::*;

    use system::{EventRecord, Phase};
    use runtime_io::with_externalities;
    use runtime_io::ed25519::Pair;
    use primitives::{H256, Blake2Hasher};
    // The testing primitives are very useful for avoiding having to work with signatures
    // or public keys. `u64` is used as the `AccountId` and no `Signature`s are requried.
    use runtime_primitives::{
        BuildStorage, traits::{BlakeTwo256}, testing::{Digest, DigestItem, Header}
    };


    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    impl_outer_event! {
        pub enum Event for Test {
            delegation<T>, balances<T>,
        }
    }

    impl_outer_dispatch! {
        pub enum Call for Test where origin: Origin {}
    }

    // For testing the module, we construct most of a mock runtime. This means
    // first constructing a configuration type (`Test`) which `impl`s each of the
    // configuration traits of modules we want to use.
    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;
    impl system::Trait for Test {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = H256;
        type Header = Header;
        type Event = Event;
        type Log = DigestItem;
    }

    impl balances::Trait for Test {
        type Balance = u64;
        type AccountIndex = u64;
        type OnFreeBalanceZero = ();
        type EnsureAccountLiquid = ();
        type Event = Event;
    }

    impl Trait for Test {
        type Event = Event;
    }

    pub type System = system::Module<Test>;
    pub type Delegation = Module<Test>;

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> sr_io::TestExternalities<Blake2Hasher> {
        let t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
        // We use default for brevity, but you can configure as desired if needed.
        t.into()
    }

    fn delegate_to(who: H256, to_account: H256, weight: u32) -> super::Result {
        Delegation::delegate_to(Origin::signed(who), to_account, weight)
    }

    fn undelegate_from(who: H256, from_account: H256, weight: u32) -> super::Result {
        Delegation::undelegate_from(Origin::signed(who), from_account, weight)
    }

    #[test]
    fn new_account_delegate_all_weight_should_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let to: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f61"));
            let public: H256 = pair.public().0.into();
            let to_public: H256 = to.public().0.into();
            let weight = 100;

            assert_ok!(delegate_to(public, to_public, weight));
            assert_eq!(System::events(), vec![
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: Event::delegation(RawEvent::Delegated(public, to_public, weight))
                }]
            );
        });
    }

    #[test]
    fn multi_delegate_should_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let public: H256 = pair.public().0.into();

            for i in 1..101 {
                let acct = H256::from(i);
                assert_ok!(delegate_to(public, acct, 1));
            }
        });
    }

    #[test]
    fn exceeding_delegation_limits_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let to: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f61"));
            let public: H256 = pair.public().0.into();
            let to_public: H256 = to.public().0.into();
            let weight = 100;
            let more_weight = 1;

            assert_ok!(delegate_to(public, to_public, weight));
            assert_eq!(delegate_to(public, to_public, more_weight), Err("Insufficient weight"));
        });
    }

    #[test]
    fn delegate_more_than_all_weight_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let to: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f61"));
            let public: H256 = pair.public().0.into();
            let to_public: H256 = to.public().0.into();
            let weight = 101;

            assert_eq!(delegate_to(public, to_public, weight), Err("Invalid weight"));
        });
    }

    #[test]
    fn delegate_to_oneself_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let public: H256 = pair.public().0.into();
            let weight = 10;

            assert_eq!(delegate_to(public, public, weight), Err("Invalid delegation action"));
        });
    }

    #[test]
    fn delegate_no_weight_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let public: H256 = pair.public().0.into();
            let weight = 0;

            assert_eq!(delegate_to(public, public, weight), Err("Invalid delegation action"));
        });
    }

    #[test]
    fn delegate_in_cycle_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let to: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f61"));
            let public: H256 = pair.public().0.into();
            let to_public: H256 = to.public().0.into();
            let weight = 100;

            assert_ok!(delegate_to(public, to_public, weight));
            assert_eq!(delegate_to(to_public, public, weight), Err("Invalid delegation due to a cycle"));
        });
    }

    #[test]
    fn new_account_delegate_and_undelegate_should_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let to: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f61"));
            let public: H256 = pair.public().0.into();
            let to_public: H256 = to.public().0.into();
            let weight = 100;

            assert_ok!(delegate_to(public, to_public, weight));
            assert_eq!(System::events(), vec![
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: Event::delegation(RawEvent::Delegated(public, to_public, weight))
                }]
            );

            assert_ok!(undelegate_from(public, to_public, weight));
            assert_eq!(System::events(), vec![
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: Event::delegation(RawEvent::Delegated(public, to_public, weight))
                },
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: Event::delegation(RawEvent::Undelegated(public, to_public, weight))
                }]
            );
        });
    }

    #[test]
    fn undelegate_from_nobody_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let public: H256 = pair.public().0.into();
            let weight = 100;

            assert_eq!(undelegate_from(public, H256::from(1), weight), Err("Delegate doesn't exist"));
        });
    }

    #[test]
    fn undelegate_from_onself_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let public: H256 = pair.public().0.into();
            let weight = 100;

            assert_eq!(undelegate_from(public, public, weight), Err("Invalid delegation action"));
        });
    }

    #[test]
    fn undelegate_with_invalid_weight_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let public: H256 = pair.public().0.into();
            let weight = 101;

            assert_eq!(undelegate_from(public, H256::from(1), weight), Err("Invalid weight"));
        });
    }

    #[test]
    fn undelegate_more_than_exists_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let public: H256 = pair.public().0.into();
            let weight = 99;
            let more_than_exists = 100;

            assert_ok!(delegate_to(public, H256::from(1), weight));
            assert_eq!(undelegate_from(public, H256::from(1), more_than_exists), Err("Invalid undelegation weight"));
        });
    }

    #[test]
    fn undelegate_less_than_exists_should_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);

            let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
            let public: H256 = pair.public().0.into();
            let weight = 99;

            assert_ok!(delegate_to(public, H256::from(1), weight));
            assert_ok!(undelegate_from(public, H256::from(1), 50));
            assert_ok!(undelegate_from(public, H256::from(1), 49));

            assert_eq!(System::events(), vec![
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: Event::delegation(RawEvent::Delegated(public, H256::from(1), weight))
                },
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: Event::delegation(RawEvent::Undelegated(public, H256::from(1), 50))
                },
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: Event::delegation(RawEvent::Undelegated(public, H256::from(1), 49))
                }]
            );
        });
    }
}