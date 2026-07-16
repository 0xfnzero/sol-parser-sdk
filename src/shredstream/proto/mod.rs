//! ShredStream protobuf 定义

pub mod shredstream {
    //! ShredStream gRPC 服务定义

    use tonic::codegen::*;

    /// Entry 订阅请求
    #[derive(Clone, Copy, PartialEq, Eq, Hash, ::prost::Message)]
    pub struct SubscribeEntriesRequest {}

    /// Entry 数据
    #[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
    pub struct Entry {
        /// 槽位号
        #[prost(uint64, tag = "1")]
        pub slot: u64,
        /// 序列化的 Entry 数据（Vec<SolanaEntry> 的 bincode 编码）
        #[prost(bytes = "vec", tag = "2")]
        pub entries: ::prost::alloc::vec::Vec<u8>,
    }

    /// ShredStream Proxy 客户端
    #[derive(Debug, Clone)]
    pub struct ShredstreamProxyClient<T> {
        inner: tonic::client::Grpc<T>,
    }

    impl ShredstreamProxyClient<tonic::transport::Channel> {
        /// 连接到指定端点
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }

    impl<T> ShredstreamProxyClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::Body>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + std::marker::Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + std::marker::Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }

        /// 设置最大解码消息大小
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }

        /// 设置最大编码消息大小
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }

        /// 订阅 Entry 流
        pub async fn subscribe_entries(
            &mut self,
            request: impl tonic::IntoRequest<SubscribeEntriesRequest>,
        ) -> std::result::Result<tonic::Response<tonic::codec::Streaming<Entry>>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::unknown(format!("Service was not ready: {}", e.into()))
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/shredstream.ShredstreamProxy/SubscribeEntries",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("shredstream.ShredstreamProxy", "SubscribeEntries"));
            self.inner.server_streaming(req, path, codec).await
        }
    }
}

pub use shredstream::{Entry, ShredstreamProxyClient, SubscribeEntriesRequest};
