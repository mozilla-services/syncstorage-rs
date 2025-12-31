# Architecture

A high-level architectural overview of the Sync Service which includes Sync and Tokenserver.

## Syncstorage
![image](assets/sync-architecture.png)

Below is an illustration of a highly-simplified Sync flow:

```mermaid
graph LR

  SignIn["Sign in to FxA"]
  FxA[("FxA")]
  OAuth["Sync client gets OAuth token"]
  PresentToken["OAuth Token presented to Tokenserver"]
  Tokenserver[("Tokenserver")]
  AssignNode["Tokenserver assigns storage node"]
  InfoCollections["info/collections: Do we even need to sync?"]

  MetaGlobal["meta/global: Do we need to start over?"]
  CryptoKeys["crypto/keys: Get keys"]

  GetStorage["GET storage/<collection>: Fetch new data"]
  PostStorage["POST storage/<collection>: Upload new data"]

  %% Main flow
  SignIn --> FxA
  FxA --> OAuth 
  OAuth --> PresentToken
  PresentToken --> Tokenserver
  Tokenserver --> AssignNode
  AssignNode --> InfoCollections

  %% Decision / metadata path
  InfoCollections --> MetaGlobal
  MetaGlobal --> CryptoKeys

  %% Sync operations
  CryptoKeys --> GetStorage
  CryptoKeys --> PostStorage
```

### Storage-Client Relationship

This high-level diagram illustrates the standard Sync collections and their relationships.

```mermaid
graph TD

  %% ===== Storage =====
  DB[("DB")]
  BookmarksMirror[("Bookmarks Mirror")]
  LoginStorage[("Login Manager Storage")]
  AutofillStorage[("Form Autofill Storage")]
  XPIDB[("XPI Database")]
  CredentialStorage[("Credential Storage")]

  %% ===== Client components =====
  Places["Places"]
  LoginManager["Login Manager"]
  TabbedBrowser["Tabbed Browser"]
  AddonManager["Add-on Manager"]
  ExtensionBridge["Extension Storage Bridge"]

  %% ===== Sync engines =====
  Bookmarks["Bookmarks"]
  History["History"]
  Passwords["Passwords"]

  CreditCards["Credit cards"]
  Addresses["Addresses"]
  OpenTabs["Open tabs"]

  Addons["Add-ons"]
  Clients["Clients"]

  %% ===== Sync internals =====
  subgraph Sync["Sync"]
  HTTPClient["HTTP Client"]
  TokenClient["Tokenserver Client"]
  end

  %% ===== Storage =====
  SyncStorage[("Sync Storage Server")]
  TokenServer[("Tokenserver")]
  PushService["Push Service"]

  subgraph FirefoxAccounts["Firefox Accounts Service"]
  PushIntegration["Push Integration"]
  FxAHTTP["HTTP Clients"]
  end
  subgraph Accounts
  MozillaPush[("Mozilla Push Server")]
  FxAAuth[("FxA Auth Server")]
  FxAOAuth[("FxA OAuth Server")]
  FxAProfile[("FxA Profile Server")]
  end

  %% ===== Relationships =====
  DB --> Places
  BookmarksMirror --> Places
  Places --> Bookmarks
  Places --> History
  LoginStorage <--> LoginManager

  AutofillStorage --> CreditCards
  AutofillStorage --> Addresses

  TabbedBrowser --> OpenTabs
  AddonManager --> Addons
  XPIDB --> AddonManager
  ExtensionBridge --> Clients

  %% ===== Sync engine / Collections =====
  Bookmarks --> Sync
  History --> Sync
  Passwords --> Sync
  CreditCards --> Sync
  Addresses --> Sync
  OpenTabs --> Sync
  Addons --> Sync
  Clients --> Sync
  HTTPClient --> Sync
  TokenClient <--> TokenServer
  SyncStorage <--> HTTPClient

  %% ===== Push & Accounts =====
  FirefoxAccounts --> PushIntegration
  FirefoxAccounts --> FxAHTTP
  FxAAuth <--> MozillaPush

  PushIntegration --> PushService
  FxAHTTP --> FxAAuth
  FxAHTTP --> FxAOAuth
  FxAHTTP --> FxAProfile
  CredentialStorage --> FirefoxAccounts
```

## Tokenserver
![image](assets/tokenserver-architecture.png)

The intent of this file is inspired by a very sensible [blog post](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html) many developers are familiar with regarding the necessity to illustrate systems with clarity. Given Sync's complexity and interrelationships with other architectures, this 

