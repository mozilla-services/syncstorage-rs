<a id="server_fxa"></a>

# Mozilla Accounts Server

The Mozilla accounts server provides a centralized database of all user
accounts for accessing Mozilla-hosted services.

By default, Firefox will use Mozilla's hosted accounts server at
<https://accounts.firefox.com>. This configuration will work well for most
use cases, including for those who want to
[self-host a storage server](how-to/how-to-run-sync-server.md).

Users who want to minimize their dependency on Mozilla-hosted services may
also [self-host an accounts server](how-to/how-to-run-fxa.md), but this setup is
incompatible with other Mozilla-hosted services.

## Resources

- **Getting Started**:  
  <https://mozilla.github.io/ecosystem-platform/tutorials/development-setup>

- **Integration with FxA**:  
  <https://mozilla.github.io/ecosystem-platform/relying-parties/tutorials/integration-with-fxa>

- **API server code**:  
  <https://github.com/mozilla/fxa/blob/main/packages/fxa-auth-server/>

- **Web interface code**:  
  <https://github.com/mozilla/fxa/blob/main/packages/fxa-content-server/>
