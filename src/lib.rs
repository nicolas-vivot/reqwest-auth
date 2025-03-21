//! # reqwest-auth
//!
//! A reqwest middleware to fill-in the authorization header using a token source.
//!
//! Uses the `token-source` crate to provide a common interface for token sources.

#![warn(missing_docs)]

use anyhow::anyhow;
use http::Extensions;
use reqwest_middleware::reqwest::header::HeaderValue;
use reqwest_middleware::reqwest::header::AUTHORIZATION;
use reqwest_middleware::reqwest::Request;
use reqwest_middleware::reqwest::Response;
use reqwest_middleware::Error;
use reqwest_middleware::Middleware;
use reqwest_middleware::Next;
use std::sync::Arc;
use token_source::TokenSource;

/// AuthorizationHeaderMiddleware
///
/// Provided a [TokenSource](token_source::TokenSource) implementation, this middleware
/// will set the Authorization header of the request with the token value obtained from this
/// token source.
///
/// The token source is expected to provide a valid token (e.g including renewal), or an error if the token
/// could not be obtained.
///
/// # How to use
///
/// ```rust
///  use reqwest_middleware::ClientBuilder;
///  use token_source::{TokenSource, TokenSourceProvider};
///  use std::sync::Arc;
///  use reqwest_auth::AuthorizationHeaderMiddleware;
///  
///  // In real cases you should have a token source provider
///  // that provides a token source implementation.
///  // Here we are using a simple example with a hardcoded token value.
///
///  // For demonstration purposes.
///  #[derive(Debug)]
///  struct MyTokenSource {
///    pub token: String,
///  }
///
///  // For demonstration purposes.
///  #[async_trait::async_trait]
///  impl TokenSource for MyTokenSource {
///    async fn token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
///       Ok(self.token.clone())
///    }
///  }
///
///  // For demonstration purposes.
///  #[derive(Debug)]
///  struct MyTokenProvider {
///    pub ts: Arc<MyTokenSource>,
///  }
///
///  // For demonstration purposes.
///  impl TokenSourceProvider for MyTokenProvider {
///    fn token_source(&self) -> Arc<dyn TokenSource> {
///      self.ts.clone()
///    }
///  }
///
///  // For demonstration purposes.
///  let ts_provider = MyTokenProvider {
///    ts: Arc::new(MyTokenSource {
///      token: "Bearer my-token".to_string(),
///    }),
///  };
///
///  // Create the middleware from the token source
///  let auth_middleware = AuthorizationHeaderMiddleware::from(ts_provider.token_source());
///
///  // Create your reqwest client with the middleware
///  let client = ClientBuilder::new(reqwest::Client::default())
///    // Ideally, the authorization middleware should come last,
///    // especially if you are using a retry middleware as well.
///    // This way, your retry requests will benefit from the renewals of the token,
///    // as long as your token source implementation is able to renew the token.
///    .with(auth_middleware)
///    .build();
/// ```
pub struct AuthorizationHeaderMiddleware {
    ts: Arc<dyn TokenSource>,
}

impl From<Arc<dyn TokenSource>> for AuthorizationHeaderMiddleware {
    fn from(ts: Arc<dyn TokenSource>) -> Self {
        Self { ts }
    }
}

impl From<Box<dyn TokenSource>> for AuthorizationHeaderMiddleware {
    fn from(ts: Box<dyn TokenSource>) -> Self {
        Self { ts: ts.into() }
    }
}

#[async_trait::async_trait]
impl Middleware for AuthorizationHeaderMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        // Obtain (or regenerate) an auth token from the token source
        let auth_token = self
            .ts
            .token()
            .await
            .map_err(|e| Error::Middleware(anyhow!(e.to_string())))?;

        // Set the Authorization header with the auth token
        // Note: any previous value of the Authorization header will be overwritten
        req.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_str(auth_token.as_str())
                .map_err(|e| Error::Middleware(anyhow!(format!("Invalid auth token value: {e}"))))?,
        );

        // Chain to next middleware in the stack
        next.run(req, extensions).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use http::Extensions;
    use reqwest_middleware::reqwest;
    use reqwest_middleware::ClientBuilder;
    use reqwest_middleware::Middleware;
    use token_source::{TokenSource, TokenSourceProvider};

    use super::AuthorizationHeaderMiddleware;
    use reqwest_middleware::reqwest::header::HeaderValue;
    use reqwest_middleware::reqwest::header::AUTHORIZATION;
    use reqwest_middleware::reqwest::Request;
    use reqwest_middleware::reqwest::Response;
    use reqwest_middleware::Next;

    #[derive(Debug)]
    struct MyTokenSource {
        pub token: String,
    }

    #[async_trait::async_trait]
    impl TokenSource for MyTokenSource {
        async fn token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.token.clone())
        }
    }

    #[derive(Debug)]
    struct MyTokenProvider {
        pub ts: Arc<MyTokenSource>,
    }

    impl TokenSourceProvider for MyTokenProvider {
        fn token_source(&self) -> Arc<dyn TokenSource> {
            self.ts.clone()
        }
    }

    /// A simple middleware to verify the Authorization header
    /// is set correctly.
    ///
    /// For testing purposes only.
    struct VerificationMiddleware {
        expected: &'static str,
    }

    #[async_trait::async_trait]
    impl Middleware for VerificationMiddleware {
        async fn handle(
            &self,
            req: Request,
            extensions: &mut Extensions,
            next: Next<'_>,
        ) -> reqwest_middleware::Result<Response> {
            // Verify the Authorization header is set correctly
            let token_value = req
                .headers()
                .get(AUTHORIZATION)
                .expect("Authorization header should be set");
            assert_eq!(token_value, &HeaderValue::from_static(self.expected));

            // Chain to next middleware in the stack
            next.run(req, extensions).await
        }
    }

    #[async_std::test]
    async fn test_middleware() {
        // Given - the Authorization middleware & test verification one
        let token_value = "Bearer my-token";
        let ts_provider = MyTokenProvider {
            ts: Arc::new(MyTokenSource {
                token: token_value.to_string(),
            }),
        };
        let auth_middleware = AuthorizationHeaderMiddleware::from(ts_provider.token_source());
        let verification_middleware = VerificationMiddleware { expected: token_value };

        let client = ClientBuilder::new(reqwest::Client::default())
            // Authorization should come first
            .with(auth_middleware)
            // Verification should come next
            .with(verification_middleware)
            .build();

        // When - making a request
        // Then - the Authorization header has been set correctly
        let _ = client
            .get("https://github.com/nicolas-vivot/reqwest-auth/CODE_OF_CONDUCT.md")
            .send()
            .await;
    }
}
