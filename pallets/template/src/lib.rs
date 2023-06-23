// TODO 0: Why do we need the following line?
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// TODO 1: Why is it useful to write `pub` here?
pub use pallet::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	pallet_prelude::Weight,
	sp_runtime::traits::{CheckedDiv, Zero},
};

use scale_info::TypeInfo;

// TODO 2: Why do we typically have a `mock` module?
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// TODO 3: What is this and what does it do?
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// REF 0: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/primitives/src/types.rs#L99
pub type MarketId = u128;

// REF 1: https://github.com/zeitgeistpm/zeitgeist/blob/main/docs/diagrams/svg/market_state_diagram.svg
// REF 2: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/primitives/src/market.rs#L229-L251
#[derive(Decode, Encode, MaxEncodedLen, TypeInfo, Clone, Debug, PartialEq, Eq)]
pub enum MarketStatus {
	Active,
	Closed,
	Reported,
	Redeemed,
}

// REF 3: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/primitives/src/market.rs#L33-L66
#[derive(Decode, Encode, MaxEncodedLen, TypeInfo, Clone, Debug, PartialEq, Eq)]
pub struct Market<AccountId, BlockNumber, Balance> {
	pub creator: AccountId,
	pub bond: Balance,
	pub data: [u8; 32],
	pub end: BlockNumber,
	pub oracle: AccountId,
	pub oracle_outcome_report: Option<u8>,
	pub status: MarketStatus,
}

// REF 4: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/primitives/src/outcome_report.rs#L21-L36
#[derive(Decode, Encode, TypeInfo, Clone, Debug, PartialEq, Eq)]
pub struct Outcome<AccountId, Balance> {
	pub owner: AccountId,
	pub data: [u8; 32],
	pub price: Balance,
}

// TODO 4: What are `CheckedDiv + Zero` called?
// TODO 5: Why can't we just remove `CheckedDiv`?
// TODO 6: What does `CheckedDiv + Zero` mean for `Balance`?
impl<AccountId, Balance: CheckedDiv + Zero> Outcome<AccountId, Balance> {
	pub fn p(&self, t: Balance) -> Balance {
		self.price.checked_div(&t).unwrap_or_else(Zero::zero())
	}
}

pub trait WeightInfo {
	fn do_something() -> Weight;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::traits::{CheckedSub, Saturating},
		traits::{
			BalanceStatus, Currency, ExistenceRequirement, ReservableCurrency, WithdrawReasons,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;

	impl<T: Config> WeightInfo for Pallet<T> {
		fn do_something() -> Weight {
			Weight::from(1_000_000_000u64)
		}
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
	pub type MarketOf<T> = Market<AccountIdOf<T>, BlockNumberFor<T>, BalanceOf<T>>;
	pub type OutcomesOf<T> =
		BoundedVec<Outcome<AccountIdOf<T>, BalanceOf<T>>, <T as Config>::MaxOutcomes>;
	pub type CacheSize = frame_support::pallet_prelude::ConstU32<64>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: ReservableCurrency<Self::AccountId>;

		// REF 5: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/prediction-markets/src/lib.rs#L783-L794 
		#[pallet::constant]
		type CreatorBond: Get<BalanceOf<Self>>;

		type DestroyOrigin: EnsureOrigin<Self::Origin>;

		#[pallet::constant]
		type MarketCreatorClearStorageTime: Get<Self::BlockNumber>;

		#[pallet::constant]
		type MaxOutcomes: Get<u32>;

		#[pallet::constant]
		type MinMarketPeriod: Get<Self::BlockNumber>;

		type PalletId: Get<PalletId>;

		type WeightInfo: WeightInfo;
	}

	// TODO 7: What does this do?
	#[pallet::type_value]
	pub fn DefaultMarketCounter<T: Config>() -> MarketId {
		1u128
	}

	// REF 6: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/market-commons/src/lib.rs#L128-L133
	#[pallet::storage]
	#[pallet::getter(fn market_counter)]
	pub type MarketCounter<T: Config> =
		StorageValue<_, MarketId, ValueQuery, DefaultMarketCounter<T>>;

	// REF 7: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/market-commons/src/lib.rs#L230-L231
	#[pallet::storage]
	pub type Markets<T: Config> =
		StorageMap<_, Blake2_128Concat, MarketId, MarketOf<T>, ValueQuery>;

	#[pallet::storage]
	pub type Outcomes<T: Config> =
		StorageMap<_, Blake2_128Concat, MarketId, OutcomesOf<T>, ValueQuery>;

	// REF 8: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/prediction-markets/src/lib.rs#L2003-L2010
	#[pallet::storage]
	pub type MarketIdsPerCloseBlock<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		BoundedVec<MarketId, CacheSize>,
		ValueQuery,
	>;

	#[pallet::event]
    	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T> {
		MarketCreated { market_id: MarketId, creator: T::AccountId },
		MarketDestroyed { market_id: MarketId },
		OutcomeBought { market_id: MarketId, outcome_index: u8, buyer: T::AccountId },
		MarketsToCloseNextBlock { market_ids: Vec<MarketId> },
		MarketClosed { market_id: MarketId },
		MarketReported { market_id: MarketId, oracle_report_outcome: u8 },
		MarketRedeemed { market_id: MarketId, winner_outcome: u8, winner: T::AccountId },
		HighestOutcome { market_id: MarketId, highest_outcome: Option<u8> },
	}

	#[pallet::error]
	pub enum Error<T> {
		StorageOverflow(u8),
		InvalidOutcomeIndex,
		MarketNotFound,
		PriceTooLow,
		OutcomeAmountTooLow,
		InsufficientBuyerBalance,
		BelowMinMarketPeriod,
		MarketNotActive,
		CallerNotOracle,
		OutcomeAlreadyReported,
		OutcomeNotReportedYet,
		InvalidMarketStatus,
		InsufficientCreatorBalance,
		OnlyMarketCreatorAllowedYet,
		Invalid,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// REF 9: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/prediction-markets/src/lib.rs#L1859
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let mut total_weight = Weight::zero();

			// TODO 8: What comes to your mind when you see the `total_weight` calculation?
			total_weight = total_weight.saturating_add(T::DbWeight::get().reads(1));
			let market_ids = <MarketIdsPerCloseBlock<T>>::get(n);
			for market_id in market_ids {
				total_weight = total_weight.saturating_add(T::DbWeight::get().reads(1));
				if let Ok(mut market) = <Markets<T>>::get(market_id) {
					// TODO 9: Why could this `debug_assert!` be useful here?
					debug_assert!(market.status == MarketStatus::Active, "MarketIdsPerCloseBlock should only contain active markets! Invalid market id: {:?}", market_id);
					market.status = MarketStatus::Closed;
					total_weight = total_weight.saturating_add(T::DbWeight::get().writes(1));
					<Markets<T>>::insert(market_id, market);
					deposit_event(Event::MarketClosed { market_id });
				};
			}
			total_weight = total_weight.saturating_add(T::DbWeight::get().writes(1));
			<MarketIdsPerCloseBlock<T>>::remove(n);

			total_weight
		}

		fn on_finalize(n: T::BlockNumber) {
			// TODO 10: What should be kept in mind, when using `on_finalize`?
			Self::on_finalize_impl(n);
		}

		fn on_idle(_n: T::BlockNumber, mut remaining_weight: Weight) -> Weight {
			if let Some(count) = remaining_weight.checked_div(T::WeightInfo::do_something()) {
				let consumed_weight = Self::emit_highest_outcomes(count);
				remaining_weight = remaining_weight.saturating_sub(consumed_weight);
			}

			remaining_weight
		}

		fn integrity_test() {
			assert!(
				T::MaxOutcomes::get() <= u8::MAX as u32,
				"The maximum of outcomes should be less than 255!"
			);
			assert!(
				!T::MinMarketPeriod::get().is_zero(),
				"The minimum market period should not be zero!"
			);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// REF 10: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/prediction-markets/src/lib.rs#L768-L779
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn create_market(
			origin: OriginFor<T>,
			data: [u8; 32],
			#[pallet::compact] outcome_amount: u8,
			end: T::BlockNumber,
			oracle: T::AccountId,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let bond = T::CreatorBond::get();
			// TODO 11:: Why do we check `can_reserve` here? Why not just using `reserve` alone?
			ensure!(T::Currency::can_reserve(&who, bond), Error::<T>::InsufficientCreatorBalance);

			ensure!(!outcome_amount.is_zero(), Error::<T>::OutcomeAmountTooLow);

			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(
				end.saturating_sub(now) >= T::MinMarketPeriod::get(),
				Error::<T>::BelowMinMarketPeriod
			);

			let market_id = Self::market_counter();
			let new_counter = market_id.checked_add(1).ok_or(Error::<T>::StorageOverflow(0u8))?;

			debug_assert!(!Markets::<T>::contains_key(market_id));

			let mut outcomes = Outcomes::<T>::get(market_id);
			for i in 0..outcome_amount {
				let outcome = Outcome { owner: who.clone(), data: [i; 32], price: Zero::zero() };
				outcomes.push(outcome).map_err(|_| Error::<T>::StorageOverflow(1u8))?;
			}

			let market = Market {
				creator: who.clone(),
				// TODO 12: Why do we like to store the bond in the market? We could have just used
				// `T::CreatorBond::get()` for the unreserve call.
				bond,
				data,
				end,
				oracle,
				oracle_outcome_report: None,
				status: MarketStatus::Active,
			};

			MarketIdsPerCloseBlock::<T>::mutate(end, |prev_market_ids| {
				prev_market_ids
					.try_push(market_id)
					.map_err(|_| <Error<T>>::StorageOverflow(2u8))?;
			})?;

			// TODO 13: Why could we want to reserve the bond here?
			T::Currency::reserve(&who, bond)?;

			<Outcomes<T>>::insert(market_id, outcomes);
			<Markets<T>>::insert(market_id, market);
			<MarketCounter<T>>::put(new_counter);

			Self::deposit_event(Event::MarketCreated { market_id, creator: who });

			Ok(())
		}

		// TODO 14: What does `Pays::No` mean? Why is it only placed here?
		// TODO 15: What does `DispatchClass::Operational` mean? Why is it only placed here?
		// REF 11: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/prediction-markets/src/lib.rs#L323-L326
		#[pallet::call_index(0)]
		#[pallet::weight((T::WeightInfo::do_something(), DispatchClass::Operational, Pays::No))]
		pub fn destroy_market(
			origin: OriginFor<T>,
			#[pallet::compact] market_id: MarketId,
		) -> DispatchResultWithPostInfo {
			// TODO 16: Why didn't I use `ensure_root(origin)?;` here?
			T::DestroyOrigin::ensure_origin(origin)?;

			ensure!(Markets::<T>::contains_key(market_id), Error::<T>::MarketNotFound);

			Markets::<T>::remove(market_id);
			Outcomes::<T>::remove(market_id);

			Self::deposit_event(Event::MarketDestroyed { market_id });

			Ok(())
		}

		// TODO 17: What could be done instead of `Pays::Yes` to get the same effect?
		// TODO 18: What does `DispatchClass::Normal` mean?
		// TODO 19: Why could this `transactional` be useful here? Why is not used in other calls?
		// REF 12: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/prediction-markets/src/lib.rs#L614-L618
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::do_something(), DispatchClass::Normal, Pays::Yes)]
		#[frame_support::transactional]
		pub fn buy_outcome(
			origin: OriginFor<T>,
			#[pallet::compact] market_id: MarketId,
			#[pallet::compact] outcome_index: u8,
			#[pallet::compact] price: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let buyer_balance = T::Currency::free_balance(&who);
			let new_buyer_balance =
				buyer_balance.checked_sub(&price).ok_or(Error::<T>::InsufficientBuyerBalance)?;
			T::Currency::ensure_can_withdraw(
				&who,
				price,
				WithdrawReasons::TRANSFER,
				new_buyer_balance,
			)?;

			let market = <Markets<T>>::get(market_id).ok_or(Error::<T>::MarketNotFound)?;
			ensure!(matches!(market.status == MarketStatus::Active), Error::<T>::MarketNotActive);

			let mut outcomes = Outcomes::<T>::get(market_id);
			let mut outcome = outcomes
				.get_mut(outcome_index)
				.ok_or(Error::<T>::InvalidOutcomeIndex)?;
			ensure!(outcome.price < price, Error::<T>::PriceTooLow);

			let market_account = Self::market_account(market_id);

			let refund_previous_buyer = || -> DispatchResult {
				let previous_buyer = &outcome.owner;
				T::Currency::transfer(
					&market_account,
					&previous_buyer,
					outcome.price,
					ExistenceRequirement::AllowDeath,
				)?;
				Ok(())
			};

			if !outcome.price.is_zero() {
				refund_previous_buyer()?;
			}

			T::Currency::transfer(&who, &market_account, price, ExistenceRequirement::AllowDeath)?;

			outcome.owner = who.clone();
			outcome.price = price;

			<Outcomes<T>>::insert(market_id, outcomes);

			Self::deposit_event(Event::OutcomeBought { market_id, outcome_index, buyer: who });

			Ok(())
		}

		// TODO 20: What could the users do, if the oracle is not honest? What is done at Zeitgeist to solve this problem?
		// REF 13: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/prediction-markets/src/lib.rs#L1271-L1275
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn report_as_oracle(
			origin: OriginFor<T>,
			#[pallet::compact] market_id: MarketId,
			#[pallet::compact] outcome_index: u8,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let mut market = <Markets<T>>::get(market_id).ok_or(Error::<T>::MarketNotFound)?;

			ensure!(market.oracle_outcome_report.is_none(), Error::<T>::OutcomeAlreadyReported);
			ensure!(market.status == MarketStatus::Closed, Error::<T>::InvalidMarketStatus);
			ensure!(market.oracle == who, Error::<T>::CallerNotOracle);

			<Markets<T>>::insert(market_id, market);
			market.oracle_outcome_report = Some(outcome_index);
			market.status = MarketStatus::Reported;

			Self::deposit_event(Event::MarketReported {
				market_id,
				oracle_report_outcome: outcome_index,
			});

			Ok(())
		}

		// REF 14: https://github.com/zeitgeistpm/zeitgeist/blob/c347f33c37838797be7323a52ed64b6ef14d4241/zrml/prediction-markets/src/lib.rs#L1092-L1095
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn redeem(
			origin: OriginFor<T>,
			#[pallet::compact] market_id: Vec<MarketId>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			let mut market = <Markets<T>>::get(market_id).ok_or(Error::<T>::MarketNotFound)?;

			let reported_index =
				market.oracle_outcome_report.ok_or(Error::<T>::OutcomeNotReportedYet)?;
			debug_assert!(market.status == MarketStatus::Reported);

			let outcomes = <Outcomes<T>>::get(market_id);
			let outcome =
				outcomes.get(reported_index as usize).ok_or(Error::<T>::InvalidOutcomeIndex)?;

			let winner = &outcome.owner;

			let market_account = Self::market_account(market_id);
			let reward = T::Currency::free_balance(&market_account);
			T::Currency::transfer(
				&market_account,
				reward,
				winner,
				ExistenceRequirement::AllowDeath,
			)?;

			market.status = MarketStatus::Redeemed;
			<Markets<T>>::insert(market_id, market);

			Self::deposit_event(Event::MarketRedeemed {
				market_id,
				winner_outcome: reported_index,
				winner: *winner,
			});

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn clear_storage(
			origin: OriginFor<T>,
			#[pallet::compact] market_id: MarketId,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let market = <Markets<T>>::get(market_id).ok_or(Error::<T>::MarketNotFound)?;
			ensure!(market.status == MarketStatus::Redeemed, Error::<T>::InvalidMarketStatus);

			let now = <frame_system::Pallet<T>>::block_number();
			let end = market.end;
			if now.saturating_sub(end) <= T::MarketCreatorClearStorageTime::get() {
				ensure!(market.creator == who, Error::<T>::OnlyMarketCreatorAllowedYet);
			}

			if who != market.creator {
				// TODO 21: Why don't I use a question mark operator here?
				let res = T::Currency::repatriate_reserved(
					&market.creator,
					&who,
					market.bond,
					BalanceStatus::Free,
				);
				debug_assert!(res.is_ok());
			} else {
				T::Currency::unreserve(&market.creator, market.bond);
			}

			<Markets<T>>::remove(market_id);
			<Outcomes<T>>::remove(market_id);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn on_finalize_impl(n: T::BlockNumber) {
			let next_block = n.saturating_add(One::one());
			let market_ids_to_close_next_block = <MarketIdsPerCloseBlock<T>>::get(next_block);
			if market_ids_to_close_next_block.is_empty() {
				return;
			}
			Self::deposit_event(Event::MarketsToCloseNextBlock(market_ids_to_close_next_block));
		}

		// TODO 22: What could be the purpose of this function?
		pub fn g(o: OutcomesOf<T>, i: u8) -> Result<BalanceOf<T>, DispatchError> {
			use frame_support::sp_runtime::SaturatedConversion;
			let t = o
				.iter()
				.map(|j| j.price.saturated_into::<u128>())
				.sum::<u128>()
				.saturated_into::<BalanceOf<T>>();
			let u = o.get(i as usize).ok_or(Error::<T>::Invalid)?;
			Ok(u.p(t))
		}

		pub fn emit_highest_outcomes(count: u64) -> Weight {
			let mut total_weight = Weight::zero();
			for (market_id, outcomes) in <Outcomes<T>>::iter().take(count) {
				let highest_outcome = outcomes
					.iter()
					.enumerate()
					.max_by_key(|(_, outcome, _)| outcome.price)
					.map(|(index, _, _)| index as u8);
				Self::deposit_event(Event::HighestOutcome { market_id, highest_outcome });

				total_weight = total_weight.saturating_add(T::WeightInfo::do_something());
			}
			total_weight
		}

		pub fn market_account(market_id: MarketId) -> AccountIdOf<T> {
			use frame_support::sp_runtime::traits::AccountIdConversion;
			T::PalletId::get().into_sub_account_truncating(market_id)
		}
	}

	impl<T> MarketApi for Pallet<T>
	{
		type MarketId = MarketId;
		type AccountId = T::AccountId;
		type Balance = BalanceOf<T>;
		type BlockNumber = T::BlockNumber;

		fn get_market(market_id: &Self::MarketId) -> Result<(Weight, MarketOf<T>), DispatchError> {
			let weight = T::DbWeight::get().reads(1);
			let market = <Markets<T>>::get(market_id).ok_or(Error::<T>::MarketNotFound)?;
			Ok((weight, market))
		}
	}
}

// TODO 23: Imagine this trait is defined outside of this pallet. Why could this be useful?
trait MarketApi {
	type MarketId;
	type AccountId;
	type Balance;
    	type BlockNumber;

	fn get_market(
		market_id: &Self::MarketId,
	) -> Result<
		(
			frame_support::pallet_prelude::Weight,
			Market<Self::AccountId, Self::Balance>,
		),
		frame_support::pallet_prelude::DispatchError,
	>;
}
