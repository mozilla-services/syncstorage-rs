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

pub fn parse_user_agent(agent: &str) -> (WootheeResult<'_>, &str, &str) {
    let parser = Parser::new();
    let wresult = parser.parse(&agent).unwrap_or_else(|| WootheeResult {
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
        let (ua_result, metrics_os, metrics_browser) = parse_user_agent(&agent);
        assert_eq!(metrics_os, "Linux");
        assert_eq!(ua_result.os, "Linux");
        assert_eq!(metrics_browser, "Firefox");
    }

    #[test]
    fn test_windows() {
        let agent = r#"Mozilla/5.0 (Windows; U; Windows NT 6.1; en-US; rv:1.9.2.3) Gecko/20100401 Firefox/3.6.3 (.NET CLR 3.5.30729)"#;
        let (ua_result, metrics_os, metrics_browser) = parse_user_agent(&agent);
        assert_eq!(metrics_os, "Windows");
        assert_eq!(ua_result.os, "Windows 7");
        assert_eq!(metrics_browser, "Firefox");
    }

    #[test]
    fn test_osx() {
        let agent =
            r#"Mozilla/5.0 (Macintosh; Intel Mac OS X 10.5; rv:2.1.1) Gecko/ Firefox/5.0.1"#;
        let (ua_result, metrics_os, metrics_browser) = parse_user_agent(&agent);
        assert_eq!(metrics_os, "Mac OSX");
        assert_eq!(ua_result.os, "Mac OSX");
        assert_eq!(metrics_browser, "Firefox");
    }

    #[test]
    fn test_other() {
        let agent =
            r#"BlackBerry9000/4.6.0.167 Profile/MIDP-2.0 Configuration/CLDC-1.1 VendorID/102"#;
        let (ua_result, metrics_os, metrics_browser) = parse_user_agent(&agent);
        assert_eq!(metrics_os, "Other");
        assert_eq!(ua_result.os, "BlackBerry");
        assert_eq!(metrics_browser, "Other");
        assert_eq!(ua_result.name, "UNKNOWN");
    }
}
