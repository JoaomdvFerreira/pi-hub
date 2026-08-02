use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceType {
    RaspberryPi,
    LinuxServer,
    MiniPc,
    Nas,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceService {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub device_type: DeviceType,
    pub monitoring_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval_seconds: Option<u32>,
    pub notifications_enabled: bool,
    pub services: Vec<DeviceService>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug)]
pub struct DeviceValidationError(pub String);

impl Device {
    pub fn validate(&self) -> Result<(), DeviceValidationError> {
        if self.name.trim().is_empty() {
            return Err(DeviceValidationError("name must not be empty".into()));
        }
        validate_host(&self.host)?;
        validate_ssh_port(self.ssh_port)?;
        validate_ssh_username(&self.ssh_username)?;
        for service in &self.services {
            validate_service(service)?;
        }
        Ok(())
    }
}

fn validate_service(service: &DeviceService) -> Result<(), DeviceValidationError> {
    if service.name.trim().is_empty() {
        return Err(DeviceValidationError(
            "service name must not be empty".into(),
        ));
    }
    validate_service_url(&service.url)
}

pub(crate) fn validate_service_url(raw_url: &str) -> Result<(), DeviceValidationError> {
    let parsed = url::Url::parse(raw_url)
        .map_err(|_| DeviceValidationError(format!("service url '{raw_url}' is not a valid URL")))?;
    if parsed.scheme() == "http" || parsed.scheme() == "https" {
        Ok(())
    } else {
        Err(DeviceValidationError(format!(
            "service url '{raw_url}' must use the http or https scheme"
        )))
    }
}

pub(crate) fn validate_ssh_port(port: u16) -> Result<(), DeviceValidationError> {
    if port == 0 {
        return Err(DeviceValidationError(
            "sshPort must be between 1 and 65535".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_ssh_username(username: &str) -> Result<(), DeviceValidationError> {
    let valid = !username.is_empty()
        && username.len() <= 32
        && username
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && username
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if valid {
        Ok(())
    } else {
        Err(DeviceValidationError(
            "sshUsername must be a valid POSIX username (lowercase letters, digits, '_' or '-', starting with a letter or '_')".into(),
        ))
    }
}

const FORBIDDEN_HOST_CHARS: &str = "\"'`$&|;<>(){}\\!*?~#";

pub(crate) fn validate_host(host: &str) -> Result<(), DeviceValidationError> {
    if host.trim().is_empty() {
        return Err(DeviceValidationError("host must not be empty".into()));
    }
    if host
        .chars()
        .any(|c| c.is_whitespace() || FORBIDDEN_HOST_CHARS.contains(c))
    {
        return Err(DeviceValidationError(
            "host contains characters that are not allowed".into(),
        ));
    }
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Ok(());
    }
    let is_valid_hostname = host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
                && label.chars().last().is_some_and(|c| c.is_ascii_alphanumeric())
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    if is_valid_hostname {
        Ok(())
    } else {
        Err(DeviceValidationError(
            "host must be a valid hostname or IP address".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device() -> Device {
        Device {
            id: "pi5-ha".into(),
            name: "Raspberry Pi 5".into(),
            host: "raspberrypi5.tail3f2a.ts.net".into(),
            ssh_port: 22,
            ssh_username: "joao".into(),
            description: Some("Home Assistant server".into()),
            device_type: DeviceType::RaspberryPi,
            monitoring_enabled: true,
            refresh_interval_seconds: None,
            notifications_enabled: true,
            services: Vec::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn valid_device_passes() {
        assert!(sample_device().validate().is_ok());
    }

    #[test]
    fn accepts_ipv4_host() {
        let mut device = sample_device();
        device.host = "192.168.1.42".into();
        assert!(device.validate().is_ok());
    }

    #[test]
    fn accepts_ipv6_host() {
        let mut device = sample_device();
        device.host = "fe80::1".into();
        assert!(device.validate().is_ok());
    }

    #[test]
    fn rejects_empty_host() {
        let mut device = sample_device();
        device.host = "".into();
        assert!(device.validate().is_err());
    }

    #[test]
    fn rejects_host_with_shell_metacharacters() {
        let mut device = sample_device();
        device.host = "pi5; rm -rf /".into();
        assert!(device.validate().is_err());
    }

    #[test]
    fn rejects_host_with_invalid_label() {
        let mut device = sample_device();
        device.host = "-badstart.example.com".into();
        assert!(device.validate().is_err());
    }

    #[test]
    fn rejects_zero_port() {
        let mut device = sample_device();
        device.ssh_port = 0;
        assert!(device.validate().is_err());
    }

    #[test]
    fn accepts_max_port() {
        let mut device = sample_device();
        device.ssh_port = 65535;
        assert!(device.validate().is_ok());
    }

    #[test]
    fn rejects_uppercase_username() {
        let mut device = sample_device();
        device.ssh_username = "Joao".into();
        assert!(device.validate().is_err());
    }

    #[test]
    fn rejects_username_starting_with_digit() {
        let mut device = sample_device();
        device.ssh_username = "1joao".into();
        assert!(device.validate().is_err());
    }

    #[test]
    fn rejects_username_with_shell_metacharacters() {
        let mut device = sample_device();
        device.ssh_username = "joao;whoami".into();
        assert!(device.validate().is_err());
    }

    #[test]
    fn rejects_empty_name() {
        let mut device = sample_device();
        device.name = "   ".into();
        assert!(device.validate().is_err());
    }

    fn sample_service() -> DeviceService {
        DeviceService {
            id: "ha".into(),
            name: "Home Assistant".into(),
            url: "http://raspberrypi5:8123".into(),
            icon: None,
            description: None,
            enabled: true,
        }
    }

    #[test]
    fn accepts_http_and_https_service_urls() {
        let mut device = sample_device();
        let mut https_service = sample_service();
        https_service.url = "https://raspberrypi5:8123".into();
        device.services = vec![sample_service(), https_service];
        assert!(device.validate().is_ok());
    }

    #[test]
    fn rejects_service_url_with_disallowed_scheme() {
        let mut device = sample_device();
        let mut service = sample_service();
        service.url = "javascript:alert(1)".into();
        device.services = vec![service];
        assert!(device.validate().is_err());
    }

    #[test]
    fn rejects_service_url_with_file_scheme() {
        let mut device = sample_device();
        let mut service = sample_service();
        service.url = "file:///etc/passwd".into();
        device.services = vec![service];
        assert!(device.validate().is_err());
    }

    #[test]
    fn rejects_malformed_service_url() {
        let mut device = sample_device();
        let mut service = sample_service();
        service.url = "not a url".into();
        device.services = vec![service];
        assert!(device.validate().is_err());
    }

    #[test]
    fn rejects_empty_service_name() {
        let mut device = sample_device();
        let mut service = sample_service();
        service.name = "  ".into();
        device.services = vec![service];
        assert!(device.validate().is_err());
    }

    #[test]
    fn device_type_serializes_kebab_case() {
        let json = serde_json::to_string(&DeviceType::RaspberryPi).unwrap();
        assert_eq!(json, "\"raspberry-pi\"");
        let json = serde_json::to_string(&DeviceType::MiniPc).unwrap();
        assert_eq!(json, "\"mini-pc\"");
    }
}
