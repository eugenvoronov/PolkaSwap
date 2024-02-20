//! Benchmarking setup for pallet-dex
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Dex;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	impl_benchmark_test_suite!(Dex, crate::mock::new_test_ext(), crate::mock::Test);
}
