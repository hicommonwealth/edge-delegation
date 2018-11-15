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
use system::ensure_signed;
use runtime_support::{StorageValue, StorageMap, Parameter};
use runtime_support::dispatch::Result;

pub trait Trait: balances::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        pub fn delegate_to(origin, to: T::AccountId, weight: T::Balance) -> Result {
            let _sender = ensure_signed(origin)?;
            ensure!(<WeightOf<T>>::get(_sender.clone()) >= weight, "Insufficient weight");

            let curr_weight = <WeightOf<T>>::get(_sender.clone());
            let new_weight = curr_weight - weight;
            <WeightOf<T>>::insert(_sender.clone(), new_weight);

            let mut delegates = <DelegatesOf<T>>::get(_sender.clone());

            // Check if delegate already exists and increase delegated weight
            if delegates.iter().any(|d| d.0 == to.clone()) {
                let index = delegates.iter().position(|d| d.0 == to.clone()).unwrap();
                let mut delegate_record = delegates.remove(index);
                delegate_record.1 += weight;
                delegates.push((to, delegate_record.1));
            } else {
                delegates.push((to, weight));
            }

            <DelegatesOf<T>>::insert(_sender.clone(), delegates);
            Ok(())
        }

        pub fn undelegate_from(origin, from: T::AccountId, weight: T::Balance) -> Result {
            let _sender = ensure_signed(origin)?;

            ensure!(<DelegatesOf<T>>::get(_sender.clone()).iter().any(|d| d.0 == from), "Delegate doesn't exist");


            let curr_weight = <WeightOf<T>>::get(_sender.clone());

            let mut delegates = <DelegatesOf<T>>::get(_sender.clone());
            let index = delegates.iter().position(|d| d.0 == from.clone()).unwrap();

            ensure!(delegates[index].1 >= weight, "Invalid undelegation weight");

            let mut delegate_record = delegates.remove(index);
            if delegate_record.1 > weight {
                delegate_record.1 -= weight;
                delegates.push(delegate_record);
            }

            let new_weight = curr_weight + weight;
            <WeightOf<T>>::insert(_sender.clone(), new_weight);
            <DelegatesOf<T>>::insert(_sender.clone(), delegates);
            Ok(())
        }
    }
}

/// An event in this module.
decl_event!(
    pub enum Event<T> where <T as system::Trait>::AccountId, <T as balances::Trait>::Balance {
        Delegated(AccountId, AccountId, Balance),
        Undelegated(AccountId, AccountId, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as IdentityStorage {
        /// The amount of undelegated weight for an account
        pub WeightOf get(weight_of): map T::AccountId => T::Balance;
        /// The map of weights an account is delegating to
        pub DelegatesOf get(delegates_of): map T::AccountId => Vec<(T::AccountId, T::Balance)>;
    }
}
