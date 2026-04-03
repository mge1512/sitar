// render_json.rs — JSON renderer (render-json BEHAVIOR)
#![allow(dead_code)]

use crate::types::SitarManifest;

/// Serialise the SitarManifest to a JSON file.
pub fn render_json(manifest: &SitarManifest, outfile: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(manifest)?;
    std::fs::write(outfile, &json)?;
    Ok(json.len())
}

/// Serialise to a String (for testing)
pub fn render_json_string(manifest: &SitarManifest) -> Result<String, Box<dyn std::error::Error>> {
    Ok(serde_json::to_string_pretty(manifest)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_render_json_meta() {
        let mut m = SitarManifest::default();
        m.meta.format_version = 1;
        m.meta.sitar_version  = "0.9.0".to_string();
        m.meta.hostname       = "myhost".to_string();
        m.meta.uname          = "Linux myhost 5.15.0".to_string();
        m.meta.collected_at   = "2026-04-03T00:00:00Z".to_string();

        let json = render_json_string(&m).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(v["meta"]["format_version"].as_u64(), Some(1));
        assert_eq!(v["meta"]["sitar_version"].as_str(), Some("0.9.0"));
        assert_eq!(v["meta"]["hostname"].as_str(), Some("myhost"));
    }

    #[test]
    fn test_render_json_scope_wrapper_structure() {
        let mut m = SitarManifest::default();
        m.cpu.elements.push(CpuRecord {
            processor: "0".to_string(),
            vendor_id: "GenuineIntel".to_string(),
            ..Default::default()
        });

        let json = render_json_string(&m).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify _attributes and _elements keys
        assert!(v["cpu"]["_attributes"].is_object());
        assert!(v["cpu"]["_elements"].is_array());
        assert_eq!(v["cpu"]["_elements"][0]["vendor_id"].as_str(), Some("GenuineIntel"));
    }

    #[test]
    fn test_render_json_packages_attributes() {
        let mut m = SitarManifest::default();
        m.packages.attributes.insert(
            "package_system".to_string(),
            serde_json::Value::String("rpm".to_string()),
        );
        m.packages.elements.push(PackageRecord {
            name: "bash".to_string(),
            version: "5.1".to_string(),
            ..Default::default()
        });

        let json = render_json_string(&m).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(v["packages"]["_attributes"]["package_system"].as_str(), Some("rpm"));
        assert_eq!(v["packages"]["_elements"][0]["name"].as_str(), Some("bash"));
    }

    #[test]
    fn test_render_json_format_version_always_1() {
        let mut m = SitarManifest::default();
        m.meta.format_version = 1;
        let json = render_json_string(&m).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["meta"]["format_version"].as_u64(), Some(1));
    }
}
