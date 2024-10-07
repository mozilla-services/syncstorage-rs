use std::fmt;
use std::str::FromStr;

use actix_http::header::ACCESS_CONTROL_REQUEST_HEADERS;
use woothee::parser::{Parser, WootheeResult};

// List of valid user-agent attributes to keep, anything not in this
// list is considered 'Other'. We log the user-agent on connect always
// to retain the full string, but for DD more tags are expensive so we
// limit to these.
const VALID_UA_BROWSER: &[&str] = &["Chrome", "Firefox", "Safari", "Opera"];

// See dataset.rs in https://github.com/woothee/woothee-rust for the
// full list (WootheeResult's 'os' field may fall back to its 'name'
// field). Windows has many values and we only care that its Windows
const VALID_UA_OS: &[&str] = &["Firefox OS", "Linux", "Mac OSX"];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeviceFamily {
    Desktop,
    Phone,
    Tablet,
    Other,
}

impl fmt::Display for DeviceFamily {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!("{:?}", self).to_lowercase();
        write!(fmt, "{}", name)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OsFamily {
    Windows,
    MacOs,
    Linux,
    IOS,
    Android,
    ChromeOs,
    Other,
}

impl fmt::Display for OsFamily {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!("{:?}", self).to_lowercase();
        write!(fmt, "{}", name)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct DeviceInfo {
    pub device_family: DeviceFamily,
    pub os_family: OsFamily,
    pub firefox_version: u32,
}

impl DeviceInfo {
    /// Determine if the device is a desktop device based on either the form factor or OS.
    pub fn is_desktop(&self) -> bool {
        matches!(&self.device_family, DeviceFamily::Desktop)
            || matches!(
                &self.os_family,
                OsFamily::MacOs | OsFamily::Windows | OsFamily::Linux
            )
    }

    /// Determine if the device is a mobile phone based on either the form factor or OS.
    pub fn is_mobile(&self) -> bool {
        matches!(
            &self.device_family,
            DeviceFamily::Phone | DeviceFamily::Tablet
        ) || matches!(&self.os_family, OsFamily::Android | OsFamily::IOS)
    }

    /// Determine if the device is iOS based on either the form factor or OS.
    pub fn is_ios(&self) -> bool {
        matches!(
            &self.device_family,
            DeviceFamily::Phone | DeviceFamily::Tablet
        ) || matches!(&self.os_family, OsFamily::Android | OsFamily::IOS)
    }

    /// Determine if the device is an android (Fenix) device based on either the form factor or OS.
    pub fn is_fenix(&self) -> bool {
        matches!(
            &self.device_family,
            DeviceFamily::Phone | DeviceFamily::Tablet
        ) || matches!(&self.os_family, OsFamily::Android)
    }
}

pub fn get_device_info(user_agent: &str) -> Result<DeviceInfo> {
    let parser = Parser::new();
    let wresult = parser.parse(user_agent).unwrap_or_else(|| WootheeResult {
        name: "",
        category: "",
        os: "",
        os_version: "".into(),
        browser_type: "",
        version: "",
        vendor: "",
    });

    let firefox_version =
        u32::from_str(wresult.version.split(".").collect::<Vec<&str>>()[0]).unwrap_or_default();
    let os = wresult.os.to_lowercase();
    let os_family = match os.as_str() {
        _ if os.starts_with("windows") => OsFamily::Windows,
        "mac osx" => OsFamily::MacOs,
        "linux" => OsFamily::Linux,
        "iphone" => OsFamily::IOS,
        "android" => OsFamily::Android,
        "chromeos" => OsFamily::ChromeOs,
        _ => OsFamily::Other,
    };
    let device_family = match wresult.category {
        "pc" => DeviceFamily::Desktop,
        "smartphone" if os.as_str() == "ipad" => DeviceFamily::Tablet,
        "smartphone" => DeviceFamily::Phone,
        _ => DeviceFamily::Other,
    };
    Ok(DeviceInfo {
        device_family,
        os_family,
        firefox_version,
    })
}

pub fn parse_user_agent(agent: &str) -> (WootheeResult<'_>, &str, &str) {
    let parser = Parser::new();
    let wresult = parser.parse(agent).unwrap_or_else(|| WootheeResult {
        name: "",
        category: "",
        os: "",
        os_version: "".into(),
        browser_type: "",
        version: "",
        vendor: "",
    });

    // Determine a base os/browser for metrics' tags
    let metrics_os = if wresult.os.starts_with("Windows") {
        "Windows"
    } else if VALID_UA_OS.contains(&wresult.os) {
        wresult.os
    } else {
        "Other"
    };
    let metrics_browser = if VALID_UA_BROWSER.contains(&wresult.name) {
        wresult.name
    } else {
        "Other"
    };
    (wresult, metrics_os, metrics_browser)
}

#[cfg(test)]
mod tests {
    use super::parse_user_agent;

    #[test]
    fn test_linux() {
        let agent = r#"Mozilla/5.0 (X11; U; Linux i686; en-US; rv:1.9.1.2) Gecko/20090807 Mandriva Linux/1.9.1.2-1.1mud2009.1 (2009.1) Firefox/3.5.2 FirePHP/0.3,gzip(gfe),gzip(gfe)"#;
        let (ua_result, metrics_os, metrics_browser) = parse_user_agent(agent);
        assert_eq!(metrics_os, "Linux");
        assert_eq!(ua_result.os, "Linux");
        assert_eq!(metrics_browser, "Firefox");
    }

    #[test]
    fn test_windows() {
        let agent = r#"Mozilla/5.0 (Windows; U; Windows NT 6.1; en-US; rv:1.9.2.3) Gecko/20100401 Firefox/3.6.3 (.NET CLR 3.5.30729)"#;
        let (ua_result, metrics_os, metrics_browser) = parse_user_agent(agent);
        assert_eq!(metrics_os, "Windows");
        assert_eq!(ua_result.os, "Windows 7");
        assert_eq!(metrics_browser, "Firefox");
    }

    #[test]
    fn test_osx() {
        let agent =
            r#"Mozilla/5.0 (Macintosh; Intel Mac OS X 10.5; rv:2.1.1) Gecko/ Firefox/5.0.1"#;
        let (ua_result, metrics_os, metrics_browser) = parse_user_agent(agent);
        assert_eq!(metrics_os, "Mac OSX");
        assert_eq!(ua_result.os, "Mac OSX");
        assert_eq!(metrics_browser, "Firefox");
    }

    #[test]
    fn test_other() {
        let agent =
            r#"BlackBerry9000/4.6.0.167 Profile/MIDP-2.0 Configuration/CLDC-1.1 VendorID/102"#;
        let (ua_result, metrics_os, metrics_browser) = parse_user_agent(agent);
        assert_eq!(metrics_os, "Other");
        assert_eq!(ua_result.os, "BlackBerry");
        assert_eq!(metrics_browser, "Other");
        assert_eq!(ua_result.name, "UNKNOWN");
    }
}
