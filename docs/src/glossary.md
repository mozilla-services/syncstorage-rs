# Glossary

**Auth Token**  
Used to identify the user after starting a session. Contains the user application ID and the expiration date.

**Cluster**  
Group of webheads and storage devices that make up a set of Service Nodes.

**Generation Number**  
An integer that may be included in an identity certificate.  
The issuing server increases this value whenever the user changes their password. By rejecting assertions with a generation number lower than the previously seen maximum for that user, the Login Server can reject assertions generated using an old password.

**Hawk Auth**  
An HTTP authentication method using a message authentication code (MAC) algorithm to provide cryptographic verification of portions of HTTP requests.  

See <https://github.com/hueniverse/hawk/>

**HKDF**  
HMAC-based Key Derivation Function, a method for deriving multiple secret keys from a single master secret.  

See <https://tools.ietf.org/html/rfc5869>

**Login Server**  
Used to authenticate user, returns tokens that can be used to authenticate to our services.

**Master Secret**  
A secret shared between Login Server and Service Node.  
Never used directly, only for deriving other secrets.

**Node**  
A URL that identifies a service, like `http://phx345`.

**Node Assignment Server**  
A service that can attribute to a user a node.

**Service**  
A service Mozilla provides, like **Sync**.

**Service Node**  
A server that contains the service, and can be mapped to several Nodes (URLs).

**Signing Secret**  
Derived from the master secret, used to sign the auth token.

**Token Secret**  
Derived from the master secret and auth token, used as **secret**.  
This is the only secret shared with the client and is different for each auth token.

**User DB**  
A database that keeps the user/node relation.

**Weave**  
The original code name for the Firefox Sync service and project.
