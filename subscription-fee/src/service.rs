use auto_farm::common::address_to_id_mapper::{AddressId, AddressToIdMapper, NULL_ID};

dharitri_sc::imports!();
dharitri_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct ServiceInfo<M: ManagedTypeApi> {
    pub sc_address: ManagedAddress<M>,
    pub opt_payment_token: Option<MoaxOrDctTokenIdentifier<M>>,
    pub amount: BigUint<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub enum SubscriptionType {
    None,
    Daily,
    Weekly,
    Monthly,
}

#[dharitri_sc::module]
pub trait ServiceModule: crate::fees::FeesModule {
    /// Arguments are pairs of sc_address, opt_payment_token and payment_amount
    #[endpoint(registerService)]
    fn register_service(
        &self,
        args: MultiValueEncoded<
            MultiValue3<ManagedAddress, Option<MoaxOrDctTokenIdentifier>, BigUint>,
        >,
    ) {
        require!(!args.is_empty(), "No arguments provided");

        let service_address = self.blockchain().get_caller();
        let existing_service_id = self.service_id().get_id(&service_address);
        require!(existing_service_id == NULL_ID, "Service already registered");

        let mut services = ManagedVec::<Self::Api, _>::new();
        for arg in args {
            let (sc_address, opt_payment_token, amount) = arg.into_tuple();
            require!(
                self.blockchain().is_smart_contract(&sc_address) && !sc_address.is_zero(),
                "Invalid SC address"
            );

            if let Some(token_id) = &opt_payment_token {
                require!(
                    self.accepted_fees_tokens().contains(token_id),
                    "Invalid token ID"
                );
            }

            services.push(ServiceInfo {
                sc_address,
                opt_payment_token,
                amount,
            });
        }

        self.pending_service_info(&service_address)
            .update(|existing_services| existing_services.extend(services.iter()));
        let _ = self.pending_services().insert(service_address);
    }

    #[endpoint(unregisterService)]
    fn unregister_service(&self) {
        let service_address = self.blockchain().get_caller();
        let service_id = self.service_id().remove_by_address(&service_address);
        if service_id != NULL_ID {
            self.service_info(service_id).clear();
        }

        let _ = self.pending_services().swap_remove(&service_address);
        self.pending_service_info(&service_address).clear();
    }

    #[only_owner]
    #[endpoint(approveService)]
    fn approve_service(&self, service_address: ManagedAddress) {
        require!(
            self.pending_services().contains(&service_address),
            "Unknown service"
        );

        let service_id = self.service_id().insert_new(&service_address);
        let service_info = self.pending_service_info(&service_address).take();
        self.service_info(service_id).set(&service_info);

        let _ = self.pending_services().swap_remove(&service_address);
    }

    /// subscribe with pair of service_id, service index, subscription type
    #[endpoint]
    fn subscribe(
        &self,
        services: MultiValueEncoded<MultiValue3<AddressId, usize, SubscriptionType>>,
    ) {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_id().get_id_non_zero(&caller);

        for pair in services {
            let (service_id, service_index, subscription_type) = pair.into_tuple();
            let service_options = self.service_info(service_id).get();
            require!(
                service_index < service_options.len(),
                "Invalid service index"
            );
            require!(
                !matches!(subscription_type, SubscriptionType::None),
                "Invalid subscription type"
            );

            self.subscription_type(caller_id, service_id, service_index)
                .set(subscription_type);
            let _ = self
                .subscribed_users(service_id, service_index)
                .insert(caller_id);
        }
    }

    /// unsubscribe from the given services, by providing the service_id and service indexes
    #[endpoint]
    fn unsubscribe(&self, services: MultiValueEncoded<MultiValue2<AddressId, usize>>) {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_id().get_id_non_zero(&caller);

        for pair in services {
            let (service_id, service_index) = pair.into_tuple();
            let service_options = self.service_info(service_id).get();
            require!(
                service_index < service_options.len(),
                "Invalid service index"
            );

            self.subscription_type(caller_id, service_id, service_index)
                .clear();
            let _ = self
                .subscribed_users(service_id, service_index)
                .swap_remove(&caller_id);
        }
    }

    #[storage_mapper("serviceId")]
    fn service_id(&self) -> AddressToIdMapper<Self::Api>;

    #[view(getPendingServices)]
    #[storage_mapper("pendingServices")]
    fn pending_services(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("pendingServiceInfo")]
    fn pending_service_info(
        &self,
        service_address: &ManagedAddress,
    ) -> SingleValueMapper<ManagedVec<ServiceInfo<Self::Api>>>;

    // one service may have multiple options
    #[view(getServiceInfo)]
    #[storage_mapper("serviceInfo")]
    fn service_info(
        &self,
        service_id: AddressId,
    ) -> SingleValueMapper<ManagedVec<ServiceInfo<Self::Api>>>;

    #[view(getSubscribedUsers)]
    #[storage_mapper("subscribedUsers")]
    fn subscribed_users(
        &self,
        service_id: AddressId,
        service_index: usize,
    ) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("subscriptionType")]
    fn subscription_type(
        &self,
        user_id: AddressId,
        service_id: AddressId,
        service_index: usize,
    ) -> SingleValueMapper<SubscriptionType>;
}
