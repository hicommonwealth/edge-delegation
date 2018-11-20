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

        pub fn delegate_to(origin, to: T::AccountId) -> Result {
            let _sender = ensure_signed(origin)?;
            // Check sender is not delegating to itself
            ensure!(_sender.clone() != to.clone(), "Invalid delegation action");
            // Check that no delegation cycle exists
            ensure!(!Self::has_delegation_cycle(_sender.clone(), to.clone()), "Invalid delegation due to a cycle");
            // Update the delegate to Some(delegate)
            <DelegatesOf<T>>::insert(_sender.clone(), to.clone());
            // Fire delegation event
            Self::deposit_event(RawEvent::Delegated(_sender.clone(), to.clone()));

            Ok(())
        }

        pub fn undelegate_from(origin, from: T::AccountId) -> Result {
            let _sender = ensure_signed(origin)?;
            // Check sender is not delegating to itself
            ensure!(_sender.clone() != from.clone(), "Invalid delegation action");
            // Update the delegate to the sender, None type throws an error due to missing Trait bound
            <DelegatesOf<T>>::insert(_sender.clone(), _sender.clone());
            // Fire delegation event
            Self::deposit_event(RawEvent::Undelegated(_sender.clone(), from.clone()));

            Ok(())
        }

        /// Delegate a fraction X/100 of an account's voting weight to the "to" account.
        /// Ensures that X is valid and that an account has enough remaining weight to
        /// delegate.
        pub fn weighted_delegate_to(origin, to: T::AccountId, weight: u32) -> Result {
            let _sender = ensure_signed(origin)?;
            // Check sender is not delegating to itself
            ensure!(_sender.clone() != to.clone(), "Invalid delegation action");
            // Check valid weight
            ensure!(weight <= 100 && weight > 0, "Invalid weight");
            // Check that no delegation cycle exists
            ensure!(!Self::weighted_has_delegation_cycle(_sender.clone(), to.clone()), "Invalid delegation due to a cycle");

            // Since weights are initialized to zero, check if we haven't delegated yet
            let mut curr_weight = <WeightOf<T>>::get(_sender.clone());
            if <WeightedDelegatesOf<T>>::get(_sender.clone()).len() > 0 {
                // Ensure account has enough delegatable weight if already delegating
                ensure!(<WeightOf<T>>::get(_sender.clone()) >= weight,
                        "Insufficient weight");    
            } else {
                curr_weight = 100;
            }
            
            // Set new weight of account by subtracting delegated weight
            let new_weight = curr_weight - weight;
            <WeightOf<T>>::insert(_sender.clone(), new_weight);

            // Check if delegate already exists and increase delegated weight
            let mut delegates = <WeightedDelegatesOf<T>>::get(_sender.clone());
            if delegates.iter().any(|d| d.0 == to.clone()) {
                let index = delegates.iter().position(|d| d.0 == to.clone()).unwrap();
                // Remove record to increment weight
                let mut delegate_record = delegates.remove(index);
                // Increment weight
                delegate_record.1 += weight;
                // Add updated delegate back to list of delegates
                delegates.push((to.clone(), delegate_record.1));
            } else {
                delegates.push((to.clone(), weight));
            }

            // Update set of delegates
            <WeightedDelegatesOf<T>>::insert(_sender.clone(), delegates);
            // Fire delegation event
            Self::deposit_event(RawEvent::WeightedDelegated(_sender.clone(), to.clone(), weight));
            Ok(())
        }

        /// Undelegate a fraction X/100 of an account's voting weight to the "to"
        /// account. Ensures that X is valid and that an account has enough remaining
        /// weight to undelegate.
        pub fn weighted_undelegate_from(origin, from: T::AccountId, weight: u32) -> Result {
            let _sender = ensure_signed(origin)?;
            // Check sender is not undelegating from itself
            ensure!(_sender.clone() != from.clone(), "Invalid delegation action");
            // Check valid weight
            ensure!(weight <= 100 && weight > 0, "Invalid weight");
            // Check that sender is delegating to target account
            ensure!(<WeightedDelegatesOf<T>>::get(_sender.clone()).iter().any(|d| d.0 == from),
                    "Delegate doesn't exist");


            let curr_weight = <WeightOf<T>>::get(_sender.clone());

            let mut delegates = <WeightedDelegatesOf<T>>::get(_sender.clone());
            let index = delegates.iter().position(|d| d.0 == from.clone()).unwrap();

            // Check that undelegation weight is $\leq$ to current delegated weight
            ensure!(delegates[index].1 >= weight, "Invalid undelegation weight");

            // Remove record and update if undelegating leaves non-zero weight
            let mut delegate_record = delegates.remove(index);
            if delegate_record.1 > weight {
                delegate_record.1 -= weight;
                delegates.push(delegate_record);
            }

            // Update weight of account by adding the undelegated weight back
            let new_weight = curr_weight + weight;
            <WeightOf<T>>::insert(_sender.clone(), new_weight);
            // Update the set of delegates of the sender
            <WeightedDelegatesOf<T>>::insert(_sender.clone(), delegates);
            // Fire undelegation event
            Self::deposit_event(RawEvent::WeightedUndelegated(_sender.clone(), from.clone(), weight));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    /// Implement rudimentary DFS to find if "to"'s delegation ever leads to "from"
    pub fn has_delegation_cycle(from: T::AccountId, to: T::AccountId) -> bool {
        // Loop over delegation path of "to" to check if "from" exists
        let mut curr = to.clone();
        while Self::delegates_of(curr.clone()).is_some() {
            match Self::delegates_of(curr.clone()) {
                Some(delegate) => {
                    if delegate.clone() == from.clone() {
                        return true;
                    }

                    curr = delegate.clone();
                },
                None => (),
            }
        }

        return false;
    }

    /// Implement rudimentary DFS to find if "to"'s delegation ever leads to "from"
    pub fn weighted_has_delegation_cycle(from: T::AccountId, to: T::AccountId) -> bool {
        // Create data structures
        let mut stack: Vec<T::AccountId> = vec![to.clone()];
        let mut seen: Vec<T::AccountId> = vec![];
        seen.push(to.clone());

        // Loop over all delegates of "to" to see if a cycle exists back to "from"
        // i.e. if "from" delegates to "to" will there be a cycle back to "from"
        while !stack.is_empty() {
            match stack.pop() {
                Some(elt) => {
                    let delegates = <WeightedDelegatesOf<T>>::get(elt.clone());
                    for d in delegates {
                        // Check if delegate is from
                        if d.0.clone() == from.clone() {
                            return true;
                        }

                        // Otherwise push delegates of node onto stack
                        if !seen.contains(&d.0.clone()) {
                            stack.push(d.0.clone());
                            // Mark delegate as seen
                            seen.push(d.0.clone());
                        }
                    }
                },
                None => ()
            }
        }

        return false;
    }

    pub fn find_rounded_weights(accounts: Vec<T::AccountId>, votes: Vec<u64>) -> Vec<(T::AccountId, u64)> {
        unimplemented!()
    }

    fn get_source_nodes(accounts: Vec<T::AccountId>) -> Vec<T::AccountId> {
        let mut bit_string: Vec<u32> = accounts.iter().map(|_| 1).collect();
        for a in accounts.clone() {
            let mut delegates: Vec<T::AccountId> = Self::weighted_delegates_of(a.clone())
                .iter()
                .map(|elt| elt.clone().0)
                .collect();

            for delegate in delegates.clone() {
                let index = delegates
                    .iter()
                    .position(|d| d == &delegate.clone())
                    .unwrap();

                bit_string[index] = 0;
            }
        }
        
        let mut result: Vec<T::AccountId> = vec![];
        for (i, elt) in bit_string.iter().enumerate() {
            if elt == &1 {
                result.push(accounts[i].clone());
            }
        }

        return result;
    }
}

/// An event in this module.
decl_event!(
    pub enum Event<T> where <T as system::Trait>::AccountId {
        Delegated(AccountId, AccountId),
        Undelegated(AccountId, AccountId),
        WeightedDelegated(AccountId, AccountId, u32),
        WeightedUndelegated(AccountId, AccountId, u32),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as IdentityStorage {
        /// The map of strict delegates for each account
        pub DelegatesOf get(delegates_of): map T::AccountId => Option<T::AccountId>;
        /// The amount of undelegated weight for an account
        pub WeightOf get(weight_of): map T::AccountId => u32;
        /// The map of weights an account is delegating to
        pub WeightedDelegatesOf get(weighted_delegates_of): map T::AccountId => Vec<(T::AccountId, u32)>;
    }
}
