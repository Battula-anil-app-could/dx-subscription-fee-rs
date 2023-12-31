dharitri_sc::imports!();

pub mod pair_proxy {
    #[dharitri_sc::proxy]
    pub trait PairProxy {
        #[view(getSafePriceByDefaultOffset)]
        fn get_safe_price_by_default_offset(
            &self,
            pair_address: ManagedAddress,
            input_payment: DctTokenPayment,
        ) -> DctTokenPayment;
    }
}

#[dharitri_sc::module]
pub trait PairActionsModule {
    #[only_owner]
    #[endpoint(addUsdcPair)]
    fn add_usdc_pair(&self, token_id: TokenIdentifier, pair_address: ManagedAddress) {
        require!(token_id.is_valid_dct_identifier(), "Invalid token ID");
        require!(
            self.blockchain().is_smart_contract(&pair_address),
            "Invalid pair address"
        );

        self.pair_address_for_token(&token_id).set(pair_address);
    }

    #[only_owner]
    #[endpoint(removeUsdcPair)]
    fn remove_usdc_pair(&self, token_id: TokenIdentifier) {
        self.pair_address_for_token(&token_id).clear();
    }

    fn get_price(&self, token_id: TokenIdentifier, amount: BigUint) -> Result<BigUint, ()> {
        let mapper = self.pair_address_for_token(&token_id);
        if mapper.is_empty() {
            return Result::Err(());
        }

        let pair_address = mapper.get();
        let price_query_address = self.price_query_address().get();
        let price: DctTokenPayment = self
            .pair_proxy(price_query_address)
            .get_safe_price_by_default_offset(
                pair_address,
                DctTokenPayment::new(token_id, 0, amount),
            )
            .execute_on_dest_context();

        Result::Ok(price.amount)
    }

    #[proxy]
    fn pair_proxy(&self, sc_address: ManagedAddress) -> pair_proxy::Proxy<Self::Api>;

    #[storage_mapper("pairAddressForToken")]
    fn pair_address_for_token(
        &self,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("priceQueryAddress")]
    fn price_query_address(&self) -> SingleValueMapper<ManagedAddress>;
}
