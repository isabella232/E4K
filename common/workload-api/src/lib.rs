// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct X509svidRequest {}
/// The `X509SVIDResponse` message carries a set of X.509 SVIDs and their
/// associated information. It also carries a set of global CRLs, and a
/// TTL to inform the workload when it should check back next.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct X509svidResponse {
    /// A list of X509SVID messages, each of which includes a single
    /// SPIFFE Verifiable Identity Document, along with its private key
    /// and bundle.
    #[prost(message, repeated, tag = "1")]
    pub svids: ::prost::alloc::vec::Vec<X509svid>,
    /// ASN.1 DER encoded
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub crl: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// CA certificate bundles belonging to foreign Trust Domains that the
    /// workload should trust, keyed by the SPIFFE ID of the foreign
    /// domain. Bundles are ASN.1 DER encoded.
    #[prost(map = "string, bytes", tag = "3")]
    pub federated_bundles:
        ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::vec::Vec<u8>>,
}
/// The X509SVID message carries a single SVID and all associated
/// information, including CA bundles.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct X509svid {
    /// The SPIFFE ID of the SVID in this entry
    #[prost(string, tag = "1")]
    pub spiffe_id: ::prost::alloc::string::String,
    /// ASN.1 DER encoded certificate chain. MAY include intermediates,
    /// the leaf certificate (or SVID itself) MUST come first.
    #[prost(bytes = "vec", tag = "2")]
    pub x509_svid: ::prost::alloc::vec::Vec<u8>,
    /// ASN.1 DER encoded PKCS#8 private key. MUST be unencrypted.
    #[prost(bytes = "vec", tag = "3")]
    pub x509_svid_key: ::prost::alloc::vec::Vec<u8>,
    /// CA certificates belonging to the Trust Domain
    /// ASN.1 DER encoded
    #[prost(bytes = "vec", tag = "4")]
    pub bundle: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Jwtsvid {
    #[prost(string, tag = "1")]
    pub spiffe_id: ::prost::alloc::string::String,
    /// Encoded using JWS Compact Serialization
    #[prost(string, tag = "2")]
    pub svid: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JwtsvidRequest {
    #[prost(string, repeated, tag = "1")]
    pub audience: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// SPIFFE ID of the JWT being requested
    /// If not set, all IDs will be returned
    #[prost(string, tag = "2")]
    pub spiffe_id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JwtsvidResponse {
    #[prost(message, repeated, tag = "1")]
    pub svids: ::prost::alloc::vec::Vec<Jwtsvid>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JwtBundlesRequest {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JwtBundlesResponse {
    /// JWK sets, keyed by trust domain URI
    #[prost(map = "string, bytes", tag = "1")]
    pub bundles:
        ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::vec::Vec<u8>>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidateJwtsvidRequest {
    #[prost(string, tag = "1")]
    pub audience: ::prost::alloc::string::String,
    /// Encoded using JWS Compact Serialization
    #[prost(string, tag = "2")]
    pub svid: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidateJwtsvidResponse {
    #[prost(string, tag = "1")]
    pub spiffe_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub claims: ::core::option::Option<::prost_types::Struct>,
}
#[doc = r" Generated client implementations."]
pub mod spiffe_workload_api_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct SpiffeWorkloadApiClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl SpiffeWorkloadApiClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> SpiffeWorkloadApiClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + Send + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> SpiffeWorkloadApiClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<http::Request<tonic::body::BoxBody>>>::Error:
                Into<StdError> + Send + Sync,
        {
            SpiffeWorkloadApiClient::new(InterceptedService::new(inner, interceptor))
        }
        #[doc = r" Compress requests with `gzip`."]
        #[doc = r""]
        #[doc = r" This requires the server to support it otherwise it might respond with an"]
        #[doc = r" error."]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        #[doc = r" Enable decompressing responses with `gzip`."]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        #[doc = " JWT-SVID Profile"]
        pub async fn fetch_jwtsvid(
            &mut self,
            request: impl tonic::IntoRequest<super::JwtsvidRequest>,
        ) -> Result<tonic::Response<super::JwtsvidResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/SpiffeWorkloadAPI/FetchJWTSVID");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn fetch_jwt_bundles(
            &mut self,
            request: impl tonic::IntoRequest<super::JwtBundlesRequest>,
        ) -> Result<
            tonic::Response<tonic::codec::Streaming<super::JwtBundlesResponse>>,
            tonic::Status,
        > {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/SpiffeWorkloadAPI/FetchJWTBundles");
            self.inner
                .server_streaming(request.into_request(), path, codec)
                .await
        }
        pub async fn validate_jwtsvid(
            &mut self,
            request: impl tonic::IntoRequest<super::ValidateJwtsvidRequest>,
        ) -> Result<tonic::Response<super::ValidateJwtsvidResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/SpiffeWorkloadAPI/ValidateJWTSVID");
            self.inner.unary(request.into_request(), path, codec).await
        }
        #[doc = " X.509-SVID Profile"]
        #[doc = " Fetch all SPIFFE identities the workload is entitled to, as"]
        #[doc = " well as related information like trust bundles and CRLs. As"]
        #[doc = " this information changes, subsequent messages will be sent."]
        pub async fn fetch_x509svid(
            &mut self,
            request: impl tonic::IntoRequest<super::X509svidRequest>,
        ) -> Result<tonic::Response<tonic::codec::Streaming<super::X509svidResponse>>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/SpiffeWorkloadAPI/FetchX509SVID");
            self.inner
                .server_streaming(request.into_request(), path, codec)
                .await
        }
    }
}
#[doc = r" Generated server implementations."]
pub mod spiffe_workload_api_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with SpiffeWorkloadApiServer."]
    #[async_trait]
    pub trait SpiffeWorkloadApi: Send + Sync + 'static {
        #[doc = " JWT-SVID Profile"]
        async fn fetch_jwtsvid(
            &self,
            request: tonic::Request<super::JwtsvidRequest>,
        ) -> Result<tonic::Response<super::JwtsvidResponse>, tonic::Status>;
        #[doc = "Server streaming response type for the FetchJWTBundles method."]
        type FetchJWTBundlesStream: futures_core::Stream<Item = Result<super::JwtBundlesResponse, tonic::Status>>
            + Send
            + 'static;
        async fn fetch_jwt_bundles(
            &self,
            request: tonic::Request<super::JwtBundlesRequest>,
        ) -> Result<tonic::Response<Self::FetchJWTBundlesStream>, tonic::Status>;
        async fn validate_jwtsvid(
            &self,
            request: tonic::Request<super::ValidateJwtsvidRequest>,
        ) -> Result<tonic::Response<super::ValidateJwtsvidResponse>, tonic::Status>;
        #[doc = "Server streaming response type for the FetchX509SVID method."]
        type FetchX509SVIDStream: futures_core::Stream<Item = Result<super::X509svidResponse, tonic::Status>>
            + Send
            + 'static;
        #[doc = " X.509-SVID Profile"]
        #[doc = " Fetch all SPIFFE identities the workload is entitled to, as"]
        #[doc = " well as related information like trust bundles and CRLs. As"]
        #[doc = " this information changes, subsequent messages will be sent."]
        async fn fetch_x509svid(
            &self,
            request: tonic::Request<super::X509svidRequest>,
        ) -> Result<tonic::Response<Self::FetchX509SVIDStream>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct SpiffeWorkloadApiServer<T: SpiffeWorkloadApi> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: SpiffeWorkloadApi> SpiffeWorkloadApiServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for SpiffeWorkloadApiServer<T>
    where
        T: SpiffeWorkloadApi,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/SpiffeWorkloadAPI/FetchJWTSVID" => {
                    #[allow(non_camel_case_types)]
                    struct FetchJWTSVIDSvc<T: SpiffeWorkloadApi>(pub Arc<T>);
                    impl<T: SpiffeWorkloadApi> tonic::server::UnaryService<super::JwtsvidRequest>
                        for FetchJWTSVIDSvc<T>
                    {
                        type Response = super::JwtsvidResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::JwtsvidRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).fetch_jwtsvid(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FetchJWTSVIDSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/SpiffeWorkloadAPI/FetchJWTBundles" => {
                    #[allow(non_camel_case_types)]
                    struct FetchJWTBundlesSvc<T: SpiffeWorkloadApi>(pub Arc<T>);
                    impl<T: SpiffeWorkloadApi>
                        tonic::server::ServerStreamingService<super::JwtBundlesRequest>
                        for FetchJWTBundlesSvc<T>
                    {
                        type Response = super::JwtBundlesResponse;
                        type ResponseStream = T::FetchJWTBundlesStream;
                        type Future =
                            BoxFuture<tonic::Response<Self::ResponseStream>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::JwtBundlesRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).fetch_jwt_bundles(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FetchJWTBundlesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/SpiffeWorkloadAPI/ValidateJWTSVID" => {
                    #[allow(non_camel_case_types)]
                    struct ValidateJWTSVIDSvc<T: SpiffeWorkloadApi>(pub Arc<T>);
                    impl<T: SpiffeWorkloadApi>
                        tonic::server::UnaryService<super::ValidateJwtsvidRequest>
                        for ValidateJWTSVIDSvc<T>
                    {
                        type Response = super::ValidateJwtsvidResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ValidateJwtsvidRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).validate_jwtsvid(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ValidateJWTSVIDSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/SpiffeWorkloadAPI/FetchX509SVID" => {
                    #[allow(non_camel_case_types)]
                    struct FetchX509SVIDSvc<T: SpiffeWorkloadApi>(pub Arc<T>);
                    impl<T: SpiffeWorkloadApi>
                        tonic::server::ServerStreamingService<super::X509svidRequest>
                        for FetchX509SVIDSvc<T>
                    {
                        type Response = super::X509svidResponse;
                        type ResponseStream = T::FetchX509SVIDStream;
                        type Future =
                            BoxFuture<tonic::Response<Self::ResponseStream>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::X509svidRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).fetch_x509svid(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FetchX509SVIDSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: SpiffeWorkloadApi> Clone for SpiffeWorkloadApiServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: SpiffeWorkloadApi> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: SpiffeWorkloadApi> tonic::transport::NamedService for SpiffeWorkloadApiServer<T> {
        const NAME: &'static str = "SpiffeWorkloadAPI";
    }
}
