use futures::{channel::mpsc, sink::Sink, Stream};
use maplit::hashmap;
use std::{collections::HashMap, time::Duration};
use tonic::{transport::channel::ClientTlsConfig, Status};
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
    SubscribeRequestFilterBlocksMeta, SubscribeRequestFilterTransactions, SubscribeUpdate,
};

use super::types::AccountsFilterMap;
use super::types::TransactionsFilterMap;
use crate::common::AnyResult;
use crate::streaming::common::StreamClientConfig as ClientConfig;
use crate::streaming::event_parser::common::filter::EventTypeFilter;
use crate::streaming::yellowstone_grpc::AccountFilter;
use crate::streaming::yellowstone_grpc::TransactionFilter;

/// Subscription manager
#[derive(Clone)]
pub struct SubscriptionManager {
    endpoint: String,
    x_token: Option<String>,
    config: ClientConfig,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new(endpoint: String, x_token: Option<String>, config: ClientConfig) -> Self {
        Self { endpoint, x_token, config }
    }

    /// Create gRPC connection
    pub async fn connect(&self) -> AnyResult<GeyserGrpcClient<impl Interceptor>> {
        let builder = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .x_token(self.x_token.clone())?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .max_decoding_message_size(self.config.connection.max_decoding_message_size)
            .connect_timeout(Duration::from_secs(self.config.connection.connect_timeout))
            .timeout(Duration::from_secs(self.config.connection.request_timeout));
        Ok(builder.connect().await?)
    }

    /// Create subscription request and return stream
    pub async fn subscribe_with_request(
        &self,
        transactions: Option<TransactionsFilterMap>,
        accounts: Option<AccountsFilterMap>,
        commitment: Option<CommitmentLevel>,
        event_type_filter: Option<&EventTypeFilter>,
    ) -> AnyResult<(
        impl Sink<SubscribeRequest, Error = mpsc::SendError>,
        impl Stream<Item = Result<SubscribeUpdate, Status>>,
        SubscribeRequest,
    )> {
        let blocks_meta =
            if event_type_filter.is_some() && event_type_filter.unwrap().include_block_event() {
                hashmap! { "".to_owned() => SubscribeRequestFilterBlocksMeta {} }
            } else if event_type_filter.is_none() {
                hashmap! { "".to_owned() => SubscribeRequestFilterBlocksMeta {} }
            } else {
                hashmap! {}
            };
        let subscribe_request = SubscribeRequest {
            accounts: accounts.unwrap_or_default(),
            transactions: transactions.unwrap_or_default(),
            blocks_meta,
            commitment: if let Some(commitment) = commitment {
                Some(commitment as i32)
            } else {
                Some(CommitmentLevel::Processed.into())
            },
            ..Default::default()
        };
        let mut client = self.connect().await?;
        let (sink, stream) = client.subscribe_with_request(Some(subscribe_request.clone())).await?;
        Ok((sink, stream, subscribe_request))
    }

    /// Create account subscription request and return stream
    pub fn subscribe_with_account_request(
        &self,
        account_filter: Vec<AccountFilter>,
        event_type_filter: Option<&EventTypeFilter>,
    ) -> Option<AccountsFilterMap> {
        if event_type_filter.is_some() && !event_type_filter.unwrap().include_account_event() {
            return None;
        }
        if account_filter.len() == 0 {
            return None;
        }
        let mut accounts = HashMap::new();
        for af in account_filter {
            accounts.insert(
                "".to_owned(),
                SubscribeRequestFilterAccounts {
                    account: af.account,
                    owner: af.owner,
                    filters: af.filters,
                    nonempty_txn_signature: None,
                },
            );
        }
        Some(accounts)
    }

    /// Generate subscription request filter
    pub fn get_subscribe_request_filter(
        &self,
        transaction_filter: Vec<TransactionFilter>,
        event_type_filter: Option<&EventTypeFilter>,
    ) -> Option<TransactionsFilterMap> {
        if event_type_filter.is_some() && !event_type_filter.unwrap().include_transaction_event() {
            return None;
        }
        let mut transactions = HashMap::new();
        for tf in transaction_filter {
            transactions.insert(
                "client".to_string(),
                SubscribeRequestFilterTransactions {
                    vote: Some(false),
                    failed: Some(false),
                    signature: None,
                    account_include: tf.account_include,
                    account_exclude: tf.account_exclude,
                    account_required: tf.account_required,
                },
            );
        }
        Some(transactions)
    }

    /// Get configuration
    pub fn get_config(&self) -> &ClientConfig {
        &self.config
    }
}
