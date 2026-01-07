<a id="howto_configure_tls"></a>

# Configure your Sync server for TLS

Firefox for Android versions 39 and up request the following protocols and
cipher suites, depending on the Android OS version.

The use of **AES128** in preference to **AES256** is driven by power and CPU
concerns.

## Cipher Suites

### Android 20+

- `TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256`
- `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`
- `TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA256`
- `TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA`
- `TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384`
- `TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA384`
- `TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA`
- `TLS_DHE_RSA_WITH_AES_128_CBC_SHA`
- `TLS_RSA_WITH_AES_128_CBC_SHA`

### Android 11+

- `TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA`
- `TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA`
- `TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA`
- `TLS_DHE_RSA_WITH_AES_128_CBC_SHA`
- `TLS_RSA_WITH_AES_128_CBC_SHA`

### Android 9+ (Gingerbread)

- `TLS_DHE_RSA_WITH_AES_128_CBC_SHA`
- `TLS_DHE_DSS_WITH_AES_128_CBC_SHA`
- `TLS_DHE_RSA_WITH_AES_128_CBC_SHA`
- `TLS_RSA_WITH_AES_128_CBC_SHA`

## Protocols

Android API levels 9 through 15 support only **TLSv1.0**.  
Modern versions of Android support all versions of TLS, from **TLSv1.0**
through **TLSv1.2**.

We intend to eliminate **TLSv1.0** on suitable Android versions as soon as
possible.

No version of Firefox for Android beyond version 38 supports **SSLv3** for
Sync.
