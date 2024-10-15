use std::fmt;
use std::str::FromStr;

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
pub enum Platform {
    FirefoxDesktop,
    Fenix,
    FirefoxIOS,
    Other,
}

impl fmt::Display for Platform {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!("{:?}", self).to_lowercase();
        write!(fmt, "{}", name)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeviceFamily {
    Desktop,
    Mobile,
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
    pub platform: Platform,
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
        matches!(&self.device_family, DeviceFamily::Mobile)
            && matches!(&self.os_family, OsFamily::Android | OsFamily::IOS)
    }

    /// Determine if the device is iOS based on either the form factor or OS.
    pub fn is_ios(&self) -> bool {
        matches!(&self.device_family, DeviceFamily::Mobile)
            && matches!(&self.os_family, OsFamily::IOS)
    }

    /// Determine if the device is an android (Fenix) device based on either the form factor or OS.
    pub fn is_fenix(&self) -> bool {
        matches!(&self.device_family, DeviceFamily::Mobile)
            && matches!(&self.os_family, OsFamily::Android)
    }
}

pub fn get_device_info(user_agent: &str) -> DeviceInfo {
    let mut w_result = Parser::new()
        .parse(user_agent)
        .unwrap_or_else(|| WootheeResult {
            name: "",
            category: "",
            os: "",
            os_version: "".into(),
            browser_type: "",
            version: "",
            vendor: "",
        });

    // Current Firefox-iOS logic outputs the `user_agent` in the following formats:
    // Firefox-iOS-Sync/108.1b24234 (iPad; iPhone OS 16.4.1) (Firefox)
    // OR
    // Firefox-iOS-FxA/24
    // Both contain prefix `Firefox-iOS` and are not successfully parsed by Woothee.
    // This custom logic accomodates the current state (Q4 - 2024)
    // This may be a discussion point for future client-side adjustment to have a more standardized
    // user_agent string.
    if user_agent.to_lowercase().starts_with("firefox-ios") {
        w_result.name = "firefox";
        w_result.category = "smartphone";
        w_result.os = "iphone";
    }

    // NOTE: Firefox on iPads report back the Safari "desktop" UA
    // (e.g. `Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_4) AppleWebKit/605.1.15
    //        (KHTML, like Gecko) Version/13.1 Safari/605.1.15)`
    // therefore we have to accept that one. This does mean that we may presume
    // that a mac safari UA is an iPad.
    if w_result.name.to_lowercase() == "safari" && !user_agent.to_lowercase().contains("firefox/") {
        w_result.name = "firefox";
        w_result.category = "smartphone";
        w_result.os = "ipad";
    }

    let os = w_result.os.to_lowercase();
    let os_family = match os.as_str() {
        _ if os.starts_with("windows") => OsFamily::Windows,
        "mac osx" => OsFamily::MacOs,
        "linux" => OsFamily::Linux,
        "iphone" | "ipad" => OsFamily::IOS,
        "android" => OsFamily::Android,
        _ => OsFamily::Other,
    };

    let device_family = match w_result.category {
        "pc" => DeviceFamily::Desktop,
        "smartphone" if os.as_str() == "ipad" => DeviceFamily::Tablet,
        "smartphone" => DeviceFamily::Mobile,
        _ => DeviceFamily::Other,
    };

    let platform = match device_family {
        DeviceFamily::Desktop => Platform::FirefoxDesktop,
        DeviceFamily::Mobile => match os_family {
            OsFamily::IOS => Platform::FirefoxIOS,
            OsFamily::Android => Platform::Fenix,
            _ => Platform::Other,
        },
        DeviceFamily::Tablet => match os_family {
            OsFamily::IOS => Platform::FirefoxIOS,
            _ => Platform::Other,
        },
        DeviceFamily::Other => Platform::Other,
    };

    let firefox_version =
        u32::from_str(w_result.version.split('.').collect::<Vec<&str>>()[0]).unwrap_or_default();

    DeviceInfo {
        platform,
        device_family,
        os_family,
        firefox_version,
    }
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
    use crate::server::user_agent::{DeviceFamily, OsFamily, Platform};

    use super::{get_device_info, parse_user_agent};

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

    #[test]
    fn test_windows_desktop() {
        let user_agent = r#"Firefox/130.0.1 (Windows NT 10.0; Win64; x64) FxSync/1.132.0.20240913135723.desktop"#;
        let device_info = get_device_info(user_agent);
        assert_eq!(device_info.platform, Platform::FirefoxDesktop);
        assert_eq!(device_info.device_family, DeviceFamily::Desktop);
        assert_eq!(device_info.os_family, OsFamily::Windows);
        assert_eq!(device_info.firefox_version, 130);
    }

    #[test]
    fn test_macos_desktop() {
        let user_agent =
            r#"Firefox/130.0.1 (Intel Mac OS X 10.15) FxSync/1.132.0.20240913135723.desktop"#;
        let device_info = get_device_info(user_agent);
        assert_eq!(device_info.platform, Platform::FirefoxDesktop);
        assert_eq!(device_info.device_family, DeviceFamily::Desktop);
        assert_eq!(device_info.os_family, OsFamily::MacOs);
        assert_eq!(device_info.firefox_version, 130);
    }

    #[test]
    fn test_fenix() {
        let user_agent = r#"Mozilla/5.0 (Android 13; Mobile; rv:130.0) Gecko/130.0 Firefox/130.0"#;
        let device_info = get_device_info(user_agent);
        assert_eq!(device_info.platform, Platform::Fenix);
        assert_eq!(device_info.device_family, DeviceFamily::Mobile);
        assert_eq!(device_info.os_family, OsFamily::Android);
        assert_eq!(device_info.firefox_version, 130);
    }

    #[test]
    fn test_firefox_ios() {
        let user_agent = r#"Firefox-iOS-FxA/24"#;
        let device_info = get_device_info(user_agent);
        assert_eq!(device_info.platform, Platform::FirefoxIOS);
        assert_eq!(device_info.device_family, DeviceFamily::Mobile);
        assert_eq!(device_info.os_family, OsFamily::IOS);
    }

    #[test]
    fn test_firefox_ios_alternate_user_agent() {
        let user_agent = r#"Firefox-iOS-Sync/115.0b32242 (iPhone; iPhone OS 17.7) (Firefox)"#;
        let device_info = get_device_info(user_agent);
        assert_eq!(device_info.platform, Platform::FirefoxIOS);
        assert_eq!(device_info.device_family, DeviceFamily::Mobile);
        assert_eq!(device_info.os_family, OsFamily::IOS);
    }
}
