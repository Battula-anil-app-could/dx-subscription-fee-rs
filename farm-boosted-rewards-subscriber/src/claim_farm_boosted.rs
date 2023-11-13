dharitri_sc::imports!();
dharitri_sc::derive_imports!();

mod claim_farm_boosted_rewards_proxy {
    dharitri_sc::imports!();

    #[dharitri_sc::proxy]
    pub trait ClaimFarmBoostedRewardsProxy {
        #[endpoint(claimBoostedRewards)]
        fn claim_boosted_rewards(&self, user: ManagedAddress) -> DctTokenPayment<Self::Api>;
    }
}

#[derive(ManagedVecItem, Clone)]
pub struct AdditionalFarmData {
    pub dummy_data: u8,
}

#[dharitri_sc::module]
pub trait ClaimFarmBoostedRewardsModule {
    fn claim_farm_boosted_rewards(
        &self,
        farm_address: ManagedAddress,
        user: ManagedAddress,
    ) -> DctTokenPayment {
        self.farm_proxy_obj(farm_address)
            .claim_boosted_rewards(user)
            .execute_on_dest_context()
    }

    #[proxy]
    fn farm_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> claim_farm_boosted_rewards_proxy::Proxy<Self::Api>;

    #[view(getEnergyThreshold)]
    #[storage_mapper("energyThreshold")]
    fn energy_threshold(&self) -> SingleValueMapper<BigUint>;
}
