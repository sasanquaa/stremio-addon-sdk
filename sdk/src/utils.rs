use semver::Version;
use stremio_core::types::addon::Manifest;

pub fn default_manifest() -> Manifest {
    Manifest {
        id: "".to_string(),
        version: Version {
            major: 0,
            minor: 1,
            patch: 0,
            pre: Default::default(),
            build: Default::default(),
        },
        name: "".to_string(),
        contact_email: None,
        description: None,
        logo: None,
        background: None,
        types: vec![],
        resources: vec![],
        id_prefixes: None,
        catalogs: vec![],
        addon_catalogs: vec![],
        behavior_hints: Default::default(),
    }
}
