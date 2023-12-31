use auto_farm::common::{
    address_to_id_mapper::{AddressId, AddressToIdMapper},
    unique_payments::UniquePayments,
};

dharitri_sc::imports!();

#[dharitri_sc::module]
pub trait FeesModule {
    #[only_owner]
    #[endpoint(addAcceptedFeesTokens)]
    fn add_accepted_fees_tokens(
        &self,
        accepted_tokens: MultiValueEncoded<MoaxOrDctTokenIdentifier>,
    ) {
        for token in accepted_tokens {
            require!(token.is_valid(), "Invalid token");

            let _ = self.accepted_fees_tokens().insert(token);
        }
    }

    #[payable("*")]
    #[endpoint]
    fn deposit(&self) {
        let (payment_token, payment_amount) = self.call_value().moax_or_single_fungible_dct();
        require!(payment_amount > 0, "No payment");
        require!(
            self.accepted_fees_tokens().contains(&payment_token),
            "Invalid payment token"
        );

        let caller = self.blockchain().get_caller();
        let caller_id = self.user_id().get_id_or_insert(&caller);
        self.add_user_payment(
            caller_id,
            MoaxOrDctTokenPayment::new(payment_token, 0, payment_amount),
            self.user_deposited_fees(caller_id),
        );
    }

    #[endpoint(withdrawFunds)]
    fn withdraw_funds(
        &self,
        tokens_to_withdraw: MultiValueEncoded<MultiValue2<MoaxOrDctTokenIdentifier, BigUint>>,
    ) -> MultiValue2<BigUint, ManagedVec<DctTokenPayment>> {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_id().get_id_non_zero(&caller);

        let user_fees_mapper = self.user_deposited_fees(caller_id);
        let mut all_user_tokens = user_fees_mapper.get().into_payments();
        let mut moax_amount = BigUint::zero();
        let mut output_payments = ManagedVec::new();
        for pair in tokens_to_withdraw {
            let (token_id, amount) = pair.into_tuple();

            if token_id.is_moax() {
                let moax_mapper = self.user_deposited_moax(caller_id);
                let user_moax_amount = moax_mapper.get();
                if user_moax_amount >= amount {
                    self.send().direct_moax(&caller, &amount);
                    moax_mapper.set(&user_moax_amount - &amount);

                    moax_amount += amount;
                }

                continue;
            }

            let mut opt_found_token_index = None;
            for (index, user_payment) in all_user_tokens.iter().enumerate() {
                if user_payment.token_identifier == token_id && user_payment.amount >= amount {
                    output_payments.push(DctTokenPayment::new(
                        token_id.unwrap_dct(),
                        0,
                        amount.clone(),
                    ));
                    opt_found_token_index = Some(index);
                    break;
                }
            }

            if opt_found_token_index.is_none() {
                continue;
            }

            let token_index = unsafe { opt_found_token_index.unwrap_unchecked() };
            let mut token_info = all_user_tokens.get(token_index);
            if token_info.amount == amount {
                all_user_tokens.remove(token_index);
            } else {
                token_info.amount -= amount;
                let _ = all_user_tokens.set(token_index, &token_info);
            }
        }

        if !output_payments.is_empty() {
            self.send().direct_multi(&caller, &output_payments);
        }

        user_fees_mapper.set(&UniquePayments::new_from_unique_payments(all_user_tokens));

        (moax_amount, output_payments).into()
    }

    fn add_user_payment(
        &self,
        caller_id: AddressId,
        payment: MoaxOrDctTokenPayment,
        dest_mapper: SingleValueMapper<UniquePayments<Self::Api>>,
    ) {
        if payment.token_identifier.is_moax() {
            self.user_deposited_moax(caller_id)
                .update(|deposited_moax| *deposited_moax += payment.amount);

            return;
        }

        if dest_mapper.is_empty() {
            let user_fees = UniquePayments::<Self::Api>::new_from_unique_payments(
                ManagedVec::from_single_item(payment.unwrap_dct()),
            );

            dest_mapper.set(&user_fees);
        } else {
            dest_mapper.update(|fees| {
                fees.add_payment(payment.unwrap_dct());
            });
        }
    }

    #[view(getAcceptedFeesTokens)]
    #[storage_mapper("acceptedFeesTokens")]
    fn accepted_fees_tokens(&self) -> UnorderedSetMapper<MoaxOrDctTokenIdentifier>;

    #[storage_mapper("userId")]
    fn user_id(&self) -> AddressToIdMapper<Self::Api>;

    #[view(getUserDepositedFees)]
    #[storage_mapper("userDepositedFees")]
    fn user_deposited_fees(
        &self,
        user_id: AddressId,
    ) -> SingleValueMapper<UniquePayments<Self::Api>>;

    #[view(getUserDepositedMoax)]
    #[storage_mapper("userDepositedMoax")]
    fn user_deposited_moax(&self, user_id: AddressId) -> SingleValueMapper<BigUint>;
}
