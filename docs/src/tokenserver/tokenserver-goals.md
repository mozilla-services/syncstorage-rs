<a id="tokenserver"></a>

# Goal of the Service

**Please Note**: BrowserID has been removed from Mozilla Accounts, and therefore
has also been removed from later versions of Tokenserver. Discussion of
BrowserID presented here is for historic purposes only.

Here’s the challenge we face. Current login for Sync looks like this:

1. Provide username and password
2. Log into LDAP with that username and password and retrieve the user’s Sync node
3. Check the Sync node against the accessed URL and use that to configure where
   the user’s data is stored

This solution works well for centralized login. It is fast, has a minimal number
of steps, and caches data centrally. The node-assignment system is lightweight,
since both the client and server cache the result, and it supports multiple
applications via the `/node/<app>` API protocol.

However, this approach breaks down when centralized login is not available.
Adding support for Firefox Accounts (FxA) authentication to the SyncStorage
protocol introduces this situation.

We will receive valid requests from users who do not yet have an account in FxA.
On the first request, we may not even know whether the node-assignment server has
ever encountered the user before.

As a result, the system must satisfy a number of requirements. Not all are
strict must-haves, but all must be considered when designing the system:

- Support multiple services (not necessarily centralized)
- Assign users to different machines as a service scales, or otherwise
  distribute them
- Consistently route a user back to the same server once assigned
- Provide operations with some control over user allocation
- Offer recovery options if a particular node fails
- Handle exhaustion attacks (e.g., an attacker auto-approving usernames until
  all nodes are full)
- Support future enhancements such as bucketed assignment
- Scale indefinitely

## Assumptions

- A **Login Server** maintains the secret for all **Service Nodes** for a given
  service
- Any webhead in a cluster can receive calls to all service nodes in that cluster
- The **Login Server** initially supports only BrowserID, but may support other
  authentication protocols in the future, provided authentication can be done
  in a single call
- All servers are time-synchronized
- The token expiration value is fixed per application  
  (e.g., 30 minutes for Sync, 2 hours for another service)
- The **Login Server** maintains a whitelist of domains for BrowserID
  verifications

## Documentation Content

- [APIs](tokenserver-api.md)
- [User Flow](user-flow.md)

## Resources

- Tokenserver is a part of Syncstorage-rs repository: <https://github.com/mozilla-services/syncstorage-rs/tree/master/syncserver/src/tokenserver>
- Tokenserver Database code: <https://github.com/mozilla-services/syncstorage-rs/blob/master/tokenserver-db/src/lib.rs>
    - See Postgres or MySQL specific implementation details in [tokenserver-mysql](https://github.com/mozilla-services/syncstorage-rs/tree/master/tokenserver-mysql) and [tokenserver-postgres](https://github.com/mozilla-services/syncstorage-rs/tree/master/tokenserver-postgres).
    - Shared implementation details are in [tokenserver-common](https://github.com/mozilla-services/syncstorage-rs/tree/master/tokenserver-common) and configuration in [tokenserver-settings](https://github.com/mozilla-services/syncstorage-rs/tree/master/tokenserver-settings).
