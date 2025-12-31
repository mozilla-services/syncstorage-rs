# Architecture

A high-level architectural overview of the Sync Service which includes Sync and Tokenserver.

# Syncstorage
![image](assets/sync-architecture.png)

# Tokenserver
![image](assets/tokenserver-architecture.png)

The intent of this file is inspired by a very sensible [blog post](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html) many developers are familiar with regarding the necessity to illustrate systems with clarity. Given Sync's complexity and interrelationships with other architectures, this 

# Storage-Client Relationship
```mermaid
graph TD

  %% ===== Client-side storage =====
  DB[("DB")]
  BookmarksMirror[("Bookmarks Mirror")]
  LoginStorage[("Login Manager Storage")]
  AutofillStorage[("Form Autofill Storage")]
  XPIDB[("XPI Database")]
  CredentialStorage[("Credential Storage")]

  %% ===== Client components =====
  Places["Places"]

    subgraph LoginManager["Login Manager"]
  Tracker
  Store
end
  TabbedBrowser["Tabbed Browser"]
  AddonManager["Add-on Manager"]
  ExtensionBridge["Extension Storage Bridge"]

  %% ===== Sync engines =====
  Bookmarks["Bookmarks"]
  History["History"]
  subgraph Passwords["Passwords"]
pwTracker["Tracker"]
pwStore["Store"]
end
  CreditCards["Credit cards"]
  Addresses["Addresses"]
  subgraph OpenTabs["Open tabs"]
Tracker
Store
end
  Addons["Add-ons"]
  Clients["Clients"]

  %% ===== Sync internals =====
  subgraph Sync["Sync"]
  HTTPClient["HTTP Client"]
  TokenClient["Token Server Client"]
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
  pwStore --> LoginManager
  LoginManager --> pwTracker


  AutofillStorage --> CreditCards
  AutofillStorage --> Addresses

  TabbedBrowser --> OpenTabs
  AddonManager --> Addons
  XPIDB --> AddonManager
  ExtensionBridge --> Clients

  %% ===== Sync engine wiring =====
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