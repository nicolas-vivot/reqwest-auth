# reqwest-auth

A reqwest middleware preparing the authorization header.

[![Crates.io](https://img.shields.io/crates/v/reqwest-auth.svg)](https://crates.io/crates/reqwest-auth)
[![Docs.rs](https://docs.rs/reqwest-auth/badge.svg)](https://docs.rs/reqwest-auth)
![CI](https://github.com/nicolas-vivot/reqwest-auth/actions/workflows/ci.yaml/badge.svg?branch=main)
[![GitHub](https://img.shields.io/github/license/nicolas-vivot/reqwest-auth)](https://github.com/nicolas-vivot/reqwest-auth/blob/main/LICENSE)

## How it works

The middleware relies on the [token source][link-token-source] crate common traits, more specifically the [TokenSource][link-token-source-code] one.
If you do not know what [token source][link-token-source] offers, please have a look at its documentation.

Long story short, the TokenSource is responsible for managing your tokens and their lifetime. (for example using `google-cloud-rust/auth`)
The middleware will use this source to obtain a token and update the AUTHORIZATION header of requests with its value.

Important: it is recommended to keep the `AuthorizationHeaderMiddleware` as the last one in your middleware chain if you need subsequent middlewares to benefit from fresh tokens.
One typical example is when you are using the `reqwest-retry` middleware, the authorization one should come after.

## Use case

This crate is for you if:
* You are already using (or thinking about it) [reqwest][link-reqwest] and/or [reqwest-middleware][link-reqwest-middleware].
* You need to authenticate your request by providing tokens in the HTTP authorization.
* You do not want to hard code or DIY it as it is bothersome.

Example:

```rust
  let ts_provider = ...; // You should build your own or use an existing one.

  // Let say you are using the retry middleware as well
  let retry_middleware = ...;
  // Create the middleware from the token source
  let auth_middleware = AuthorizationHeaderMiddleware::from(ts_provider.token_source());

  // Create your reqwest client with the middleware
  let client = ClientBuilder::new(reqwest::Client::default())
    .with(retry_middleware)
    // Ideally, the authorization middleware should come last,
    // especially if you are using a retry middleware as well.
    // This way, your retry requests will benefit from the renewals of the token,
    // as long as your token source implementation is able to renew the token.
    .with(auth_middleware)
    .build();
```


[link-token-source]: https://github.com/nicolas-vivot/token-source
[link-token-source-code]: https://github.com/nicolas-vivot/token-source/blob/main/src/lib.rs#L28
[link-reqwest]: https://github.com/seanmonstar/reqwest
[link-reqwest-middleware]: https://github.com/TrueLayer/reqwest-middleware
