pub use serde::{Serialize, Deserialize, de::DeserializeOwned, Serializer, Deserializer};
pub use std::error::Error;
pub use std::str::FromStr;

pub use fnv::{FnvHashMap as Map, FnvHashSet as Set};

//pub use std::collections::{BTreeMap as Map, BTreeSet as Set};
pub use log::{trace, debug, info, warn, error, log};
pub use std::iter::FromIterator;

pub type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

pub use regex::Regex;
pub use lazy_static::lazy_static;
pub use crate::cache::Evr;

pub use std::{io, fmt};
pub use std::hash::Hash;
pub use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct PagingInfo {
    page: usize,
    page_size: usize,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Nevra {
    pub name: String,
    pub epoch: Option<String>,
    pub version: String,
    pub release: String,
    pub arch: String,
}

impl Nevra {
    pub fn from_name_evr_arch(
        name: impl Into<String>,
        evr: impl Into<Evr>,
        arch: impl Into<String>,
    ) -> Self {
        let evr = evr.into();
        let mut epoch = None;
        if evr.0 != 0 {
            epoch = Some(evr.0.to_string());
        }

        Nevra {
            name: name.into(),
            epoch: epoch,
            version: evr.1.into(),
            release: evr.2.into(),
            arch: arch.into(),
        }
    }
}

impl Nevra {
    pub fn evr(&self) -> Evr {
        let mut epoch = 0;
        if let Some(e) = self.epoch.as_ref() {
            epoch = FromStr::from_str(&e).unwrap();
        }
        return Evr(epoch, self.version.clone(), self.release.clone());
    }
}

impl ToString for Nevra {
    fn to_string(&self) -> String {
        let epoch = if let Some(ref epoch) = self.epoch {
            format!("{}:", epoch)
        } else {
            String::new()
        };

        format!(
            "{}-{}{}-{}.{}",
            self.name, epoch, self.version, self.release, self.arch
        )
    }
}

impl Serialize for Nevra {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}


pub const PKG_NAME: &str = "([^:(/=<> ]+)";
pub const PKG_EPOCH: &str = "([0-9]+:)?";
pub const PKG_VERSION: &str = "([^-:(/=<> ]+)";
pub const PKG_RELEASE: &str = PKG_VERSION;
pub const PKG_ARCH: &str = "([^-:.(/=<> ]+)";

lazy_static! {
    static ref NEVRA_RE: Regex =
    Regex::new(&format!(r#"^{}-{}{}-{}\.{}$"#, PKG_NAME,PKG_EPOCH, PKG_VERSION, PKG_RELEASE, PKG_ARCH)).unwrap();

}

impl FromStr for Nevra {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: Rewrite using nom parser, gonna be faster and prettier
        if let Some(caps) = NEVRA_RE.captures(s) {
            return Ok(Nevra {
                name: caps.get(1).map(|x| x.as_str().to_owned()).unwrap().into(),
                epoch: caps.get(2).map(|x| x.as_str()).map(|s : &str| {
                    s.trim_matches(':').to_string()
                }),
                version: caps.get(3).map(|x| x.as_str().to_owned()).unwrap().into(),
                release: caps.get(4).map(|x| x.as_str().to_owned()).unwrap().into(),
                arch: caps.get(5).map(|x| x.as_str().to_owned()).unwrap().into(),
            });
        }
        Err(())
    }
}
