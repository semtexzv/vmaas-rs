use crate::prelude::*;

use rusqlite::{Connection, params, OpenFlags, types::FromSql};
use std::path::PathBuf;
use serde_aux::prelude::*;

use std::fmt::Display;

mod util;

use util::*;

#[derive(Debug, Deserialize, Serialize, Clone, Hash, PartialOrd, PartialEq, Eq)]
pub struct Evr(
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub i64,
    pub String,
    pub String,
);

impl FromStr for Evr {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<_> = s.split(':').collect::<Vec<_>>();
        let release = parts.pop().unwrap();
        let version = parts.pop().unwrap();
        let epoch = parts.pop().unwrap();
        Ok(Evr(
            epoch.parse().unwrap(),
            version.into(),
            release.into(),
        ))
    }
}

impl Display for Evr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}:{}", self.0, self.1, self.2)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Hash, PartialOrd, PartialEq, Eq)]
pub struct NevraId(pub i64, pub i64, pub i64);

impl FromStr for NevraId {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<_> = s.split(':').collect::<Vec<_>>();
        let arch = parts.pop().unwrap();
        let evr = parts.pop().unwrap();
        let name = parts.pop().unwrap();
        Ok(NevraId(
            name.parse().unwrap(),
            evr.parse().unwrap(),
            arch.parse().unwrap(),
        ))
    }
}

impl Display for NevraId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}:{}", self.0, self.1, self.2)
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Hash)]
pub struct Package {
    pub name_id: i64,
    pub evr_id: i64,
    pub arch_id: i64,
    pub summary: Option<i64>,
    pub desc: Option<i64>,
    pub source_pkg_id: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Cve {
    pub name: String,
    pub redhat_url: Option<String>,
    pub secondary_url: Option<String>,

    pub cvss3_score: Option<f64>,
    pub cvss3_metrics: Option<String>,

    pub impact: String,
    pub published_date: Option<String>,
    pub modified_date: Option<String>,
    pub iava: Option<String>,
    pub description: Option<String>,
    pub cvss2_score: Option<f64>,
    pub cvss2_metrics: Option<String>,

    pub cve_source: String,

    pub cwes: Vec<String>,
    pub pkgids: Vec<i64>,
    pub errataids: Vec<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Hash)]
pub struct Repo {
    pub label: String,
    pub name: String,
    pub url: String,
    pub basearch: Option<String>,
    pub releasever: Option<String>,
    pub product: Option<String>,
    pub product_id: Option<i64>,
    // DateTime, fix parsing
    pub revision: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Hash)]
pub struct Errata {
    pub name: String,
    pub synopsis: String,
    pub summary: String,
    pub typ: String,
    pub severity: String,
    pub description: Option<String>,
    pub solution: String,
    pub issued: String,
    pub updated: String,
    pub url: String,
}


#[derive(Debug, Default)]
pub struct Cache {
    pub name_to_id: Map<String, i64>,
    pub id_to_name: Map<i64, String>,

    pub updates: Map<i64, Vec<i64>>,
    pub updates_index: Map<i64, Map<i64, usize>>,

    pub evr_to_id: Map<Evr, i64>,
    pub id_to_evr: Map<i64, Evr>,

    pub arch_to_id: Map<String, i64>,
    pub id_to_arch: Map<i64, String>,

    pub arch_compat: Map<i64, Vec<i64>>,

    pub pkg_details: Map<i64, Package>,
    pub nevra_to_pkgid: Map<NevraId, i64>,
    pub repo_detail: Map<i64, Repo>,
    pub repolabel_to_ids: Map<String, Vec<i64>>,
    pub productid_to_repoids: Map<i64, Vec<i64>>,
    pub pkgid_to_repoids: Map<i64, Vec<i64>>,
    pub errataid_to_name: Map<i64, String>,
    pub pkgid_to_errataids: Map<i64, Vec<i64>>,
    pub errataid_to_repoids: Map<i64, Vec<i64>>,
    pub cve_detail: Map<i64, Cve>,
    pub dbchange: Map<String, String>,
    pub errata_detail: Map<i64, Errata>,
    pub pkgerrata_to_module: Map<String, String>,
    pub modulename_to_id: Map<String, Vec<i64>>,
    pub src_pkg_id_to_pkg_ids: Map<i64, Vec<i64>>,
    pub strings: Map<i64, Option<String>>,
}


pub fn load_updates(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    load_rows(db, "updates", "name_id, package_id, package_order", "package_order", |r| {
        let arr = cache.updates.entry(r.get(0)?).or_default();
        arr.push(r.get(1)?);
        Ok(())
    })?;

    load_rows(db, "updates_index", "name_id, evr_id, package_order", "package_order", |r| {
        let per_name = cache.updates_index.entry(r.get(0)?).or_default();
        *per_name.entry(r.get(1)?).or_default() = r.get::<_, isize>(2)? as usize;
        Ok(())
    })?;

    Ok(())
}

pub fn load_evr(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    let mut res = vec![];

    let e = load_rows(db, "evr", "id, epoch, version, release", "id", |r| {
        res.push((r.get(0)?, Evr(r.get(1)?, r.get(2)?, r.get(3)?)));
        Ok(())
    })?;

    cache.id_to_evr = res.into_iter().collect();
    cache.evr_to_id = invert(&cache.id_to_evr);
    Ok(())
}

pub fn load_arch(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    cache.id_to_arch = load_map(db, "arch", "id", "arch", "id")?;
    cache.arch_to_id = invert(&cache.id_to_arch);

    let c1 = load_vec(db, "arch_compat", "from_arch_id", "from_arch_id")?;
    let c2 = load_vec(db, "arch_compat", "to_arch_id", "from_arch_id")?;

    for (c1, c2) in c1.into_iter().zip(c2.into_iter()) {
        cache.arch_compat.entry(c1).or_default().push(c2);
    }

    Ok(())
}

pub fn load_pkg(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    load_rows(db, "package_detail", "id, name_id, evr_id, arch_id, summary_id, description_id, source_package_id", "id", |r| {
        let id: i64 = r.get(0)?;
        let pkg = Package {
            name_id: r.get(1)?,
            evr_id: r.get(2)?,
            arch_id: r.get(3)?,

            summary: r.get(4)?,
            desc: r.get(5)?,

            source_pkg_id: r.get(6)?,
            ..Default::default()
        };
        cache.pkg_details.insert(id, pkg);
        Ok(())
    })?;

    for (id, p) in cache.pkg_details.iter() {
        cache.nevra_to_pkgid.insert(NevraId(p.name_id, p.evr_id, p.arch_id), *id);

        if let Some(src) = p.source_pkg_id {
            cache.src_pkg_id_to_pkg_ids.entry(src).or_default().push(*id);
        }
    }

    Ok(())
}

pub fn load_repos(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    load_rows(db, "repo_detail", "id, label, name, url, basearch, releasever, product, product_id, revision", "id", |r| {
        let repo = Repo {
            label: r.get(1)?,
            name: r.get(2)?,
            url: r.get(3)?,
            basearch: r.get(4)?,
            releasever: r.get(5)?,
            product: r.get(6)?,
            product_id: r.get(7)?,
            revision: r.get(8)?,
        };
        cache.repo_detail.insert(r.get(0)?, repo);
        Ok(())
    })?;

    for (id, r) in cache.repo_detail.iter() {
        cache.repolabel_to_ids.entry(r.label.clone()).or_default().push(*id);
        if let Some(pid) = r.product_id {
            cache.productid_to_repoids.entry(pid).or_default().push(*id);
        }
    }

    Ok(())
}

pub fn load_cve(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    load_rows(db, "cve_detail", "id, name, redhat_url, secondary_url, cvss3_score, cvss3_metrics, impact, published_date, modified_date, iava, description, cvss2_score, cvss2_metrics, source", "id", |r| {
        let repo = Cve {
            name: r.get(1)?,
            redhat_url: r.get(2)?,
            secondary_url: r.get(3)?,

            cvss3_score: r.get(4)?,
            cvss3_metrics: r.get(5)?,

            impact: r.get(6)?,
            published_date: r.get(7)?,
            modified_date: r.get(8)?,

            iava: r.get(9)?,
            description: r.get(10)?,
            cvss2_score: r.get(11)?,
            cvss2_metrics: r.get(12)?,
            cve_source: r.get(13)?,

            cwes: vec![],
            errataids: vec![],
            pkgids: vec![],
        };
        cache.cve_detail.insert(r.get(0)?, repo);
        Ok(())
    })?;

    for (cve_id, v) in load_multimap(db, "cve_cwe", "cve_id", "cwe", "cve_id")?.into_iter() {
        if let Some(cve) = cache.cve_detail.get_mut(&cve_id) {
            cve.cwes = v;
        }
    }

    for (cve_id, erratas) in load_multimap(db, "errata_cve", "cve_id", "errata_id", "cve_id")?.into_iter() {
        if let Some(cve) = cache.cve_detail.get_mut(&cve_id) {
            cve.errataids = erratas;
        }
    }

    for (cve_id, pkgids) in load_multimap(db, "cve_pkg", "cve_id", "pkg_id", "cve_id")?.into_iter() {
        if let Some(cve) = cache.cve_detail.get_mut(&cve_id) {
            cve.pkgids = pkgids;
        }
    }

    Ok(())
}

pub fn load_dbchange(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    Ok(())
}

pub fn load_errata(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    load_rows(db, "errata_detail", "id, name, synopsis, summary, type, severity, description, solution, issued, updated, url", "id", |r| {
        let erratum = Errata {
            name: r.get(1)?,
            synopsis: r.get(2)?,
            summary: r.get(3)?,
            typ: r.get(4)?,
            severity: r.get(5)?,
            description: r.get(6)?,
            solution: r.get(7)?,
            issued: r.get(8)?,
            updated: r.get(9)?,
            url: r.get(10)?,
        };
        cache.errata_detail.insert(r.get(0)?, erratum);
        Ok(())
    })?;

    Ok(())
}


pub fn load_modules(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    cache.modulename_to_id = load_multimap(db, "module_stream", "module", "stream_id", "module")?;
    /*
    pub pkgerrata_to_module: Map<String, String>,
    */

    Ok(())
}


pub fn load_names(db: &mut Connection, cache: &mut Cache) -> Result<()> {
    cache.id_to_name = load_map(db, "packagename", "id", "packagename", "id")?;
    cache.name_to_id = invert(&cache.id_to_name);

    Ok(())
}


#[no_mangle]
pub extern "C" fn load(name: &str) -> Result<Cache> {
    let file = PathBuf::from(name);
    let mut db = Connection::open_with_flags(&file, OpenFlags::SQLITE_OPEN_READ_ONLY).expect("Opening failed");

    let mut cache = Cache::default();

    load_names(&mut db, &mut cache)?;
    load_updates(&mut db, &mut cache)?;
    load_evr(&mut db, &mut cache)?;
    load_arch(&mut db, &mut cache)?;
    load_pkg(&mut db, &mut cache)?;
    load_repos(&mut db, &mut cache)?;

    cache.pkgid_to_repoids = load_multimap(&mut db, "pkg_repo", "pkg_id", "repo_id", "pkg_id")?;
    cache.errataid_to_name = load_map(&mut db, "errata_detail", "id", "name", "id")?;
    cache.pkgid_to_errataids = load_multimap(&mut db, "pkg_errata", "pkg_id", "errata_id", "pkg_id")?;
    cache.errataid_to_repoids = load_multimap(&mut db, "errata_repo", "errata_id", "repo_id", "errata_id")?;

    load_cve(&mut db, &mut cache)?;
    load_dbchange(&mut db, &mut cache)?;
    load_errata(&mut db, &mut cache)?;
    load_modules(&mut db, &mut cache)?;

    cache.strings = load_map::<i64, _>(&mut db, "string", "id", "string", "id")?;

    println!("Loaded all");
    Ok(cache)
}
