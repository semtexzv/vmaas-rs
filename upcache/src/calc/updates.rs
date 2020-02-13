use crate::prelude::*;

use crate::cache::{Cache, NevraId};
use std::collections::BTreeSet;

pub struct Updates;

#[derive(Debug, Deserialize, Clone)]
pub struct ModuleSpec {
    pub module_name: String,
    pub module_stream: String,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct Request {
    pub package_list: Vec<String>,

    pub repository_list: Option<Vec<String>>,
    pub modules_list: Option<Vec<ModuleSpec>>,
    pub releasever: Option<String>,
    pub basearch: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PkgUpdate {
    package: Nevra,
    erratum: String,

    repository: Option<String>,
    basearch: Option<String>,
    // TODO: Make this an option string
    releasever: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdatesPkgDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    available_updates: Vec<PkgUpdate>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Response {
    update_list: Map<String, UpdatesPkgDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    releasever: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    basearch: Option<String>,
}

macro_rules! try_skip {
    ($res:expr) => {
        match $res {
            Some(val) => val,
            _ => {
                continue;
            }
        }
    };
    ($res:expr, $msg:expr $(,$args:expr)*) => {
        match $res {
            Some(val) => val,
            _ => {
                println!($msg $(,$args)*);
                continue;
            }
        }
    };
}

impl Updates {
    fn build_nevra(cache: &Cache, update_pkg_id: i64) -> Nevra {
        let det = &cache.pkg_details[&update_pkg_id];
        let name = &cache.id_to_name[&det.name_id];
        let evr = &cache.id_to_evr[&det.evr_id];
        let arh = &cache.id_to_arch[&det.arch_id];
        return Nevra::from_name_evr_arch(name.clone(), evr.clone(), arh.clone());
    }

    fn related_products(cache: &Cache, original_repo_ids: &Set<i64>) -> Set<Option<i64>> {
        let mut product_ids = Set::default();
        for original_pkg_repo_id in original_repo_ids.iter() {
            product_ids.insert(cache.repo_detail[original_pkg_repo_id].product_id);
        }
        return product_ids;
    }

    fn valid_releasevers(cache: &Cache, original_repo_ids: &Set<i64>) -> Set<Option<String>> {
        let mut valid_releasevers = Set::default();
        for original_pkg_repo_id in original_repo_ids.iter() {
            valid_releasevers.insert(cache.repo_detail[original_pkg_repo_id].releasever.to_owned());
        }
        return valid_releasevers;
    }

    fn get_repositories(
        cache: &Cache,
        product_ids: &Set<Option<i64>>,
        update_pkg_id: i64,
        errata_ids: &[i64],
        available_repo_ids: &Set<i64>,
        valid_releasevers: &Set<Option<String>>,
    ) -> Set<i64> {
        let mut errata_repo_ids = Set::default();

        for errata_id in errata_ids {
            errata_repo_ids.extend(&cache.errataid_to_repoids[errata_id]);
        }

        let repo_ids = cache.pkgid_to_repoids.get(&update_pkg_id)
            .map(Set::from_iter)
            .unwrap_or(Set::default());

        let mut repo_ids = repo_ids
            .intersection(&errata_repo_ids).map(|s| **s).collect::<Set<i64>>()
            .intersection(available_repo_ids).map(|s| *s).collect::<Set<i64>>();


        repo_ids.retain(|repo_id| {
            let detail = &cache.repo_detail[&repo_id];
            valid_releasevers.contains(&detail.releasever)
                && product_ids.contains(&detail.product_id)
        });


        return repo_ids;
    }

    fn process_updates(
        cache: &Cache,
        packages_to_process: &Map<&str, Nevra>,
        available_repo_ids: &Set<i64>,
        response: &mut Response,
    ) -> Result<()> {
        for (pkg, nevra) in packages_to_process.iter() {
            println!("Processing {:?}", pkg);
            let name_id = try_skip!(cache.name_to_id.get(&nevra.name), "Name not found");
            let updates = try_skip!(cache.updates.get(&name_id), "Updates not found");
            let updates_index = try_skip!(cache.updates_index.get(&name_id), "updates index not found");

            let evr_id = try_skip!(cache.evr_to_id.get(&nevra.evr()), "Evr not found {:?}", nevra);

            let arch_id = cache.arch_to_id.get(&nevra.arch).ok_or(format!("arch_id not found : {:?}", nevra.arch))?;
            let arch_compat = try_skip!(cache.arch_compat.get(arch_id), "Arch compat not found");
            let current_evr_idx = try_skip!(updates_index.get(&evr_id), "EVR index not found");

            let current_nevra_pkg_id = updates[*current_evr_idx];
            println!("Package FOUND {:?} = with NEVRA:{:?} Found", current_nevra_pkg_id, nevra);

            let resp_pkg_detail = response.update_list.entry((*pkg).into()).or_default();

            let last_version_pkg_id = updates.last();
            if last_version_pkg_id == Some(&current_nevra_pkg_id) {
                println!("Package is last, no updates");
                continue;
            }

            let mut original_package_repo_ids = Set::default();

            if let Some(repoids) = cache.pkgid_to_repoids.get(&current_nevra_pkg_id) {
                original_package_repo_ids.extend(repoids.iter());
            }

            let product_ids = Self::related_products(cache, &original_package_repo_ids);
            let valid_releasevers = Self::valid_releasevers(cache, &original_package_repo_ids);

            println!("Valid prods : {:?}, valid vers : {:?}", product_ids, valid_releasevers);
            let update_pkg_ids = &updates[(*current_evr_idx + 1)..];

            for update_pkg_id in update_pkg_ids {
                println!("Update pkg id : {:?}", update_pkg_id);
                let errata_ids = try_skip!(cache.pkgid_to_errataids.get(update_pkg_id));
                let updated_nevra_arch_id = cache.pkg_details[update_pkg_id].arch_id;
                //println!("Update pkg arch : {:?}, orig arch id : {:?}", updated_nevra_arch_id, arch_id);

                if updated_nevra_arch_id != *arch_id && !arch_compat.contains(&updated_nevra_arch_id)
                {
                    //println!("Filteroing out id : {:?}, wrong arch", update_pkg_id);
                    continue;
                }
                let nevra = Self::build_nevra(cache, *update_pkg_id);
                for errata_id in errata_ids {
                    let mut repo_ids = Self::get_repositories(
                        cache,
                        &product_ids,
                        *update_pkg_id,
                        &[*errata_id],
                        &available_repo_ids,
                        &valid_releasevers,
                    );
                    println!("Repoids avail : {:?}", repo_ids);

                    for repo_id in repo_ids {
                        let repo_det = &cache.repo_detail[&repo_id];
                        resp_pkg_detail.available_updates.push(PkgUpdate {
                            package: nevra.clone(),
                            erratum: cache.errataid_to_name[errata_id].clone(),
                            repository: Some(repo_det.label.clone()),
                            basearch: repo_det.basearch.clone(),
                            releasever: repo_det.releasever.clone().unwrap_or_default(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn process_repositories(
        cache: &Cache,
        data: &Request,
        response: &mut Response,
    ) -> Set<i64> {
        // Either use provided repository list or all repositories in resolution
        let mut available_repo_ids = Vec::new();
        if let Some(ref repos) = data.repository_list {
            for repo in repos {
                if let Some(ids) = cache.repolabel_to_ids.get(repo) {
                    available_repo_ids.extend_from_slice(&ids)
                }
            }
            response.repository_list = Some(repos.clone());
        } else {
            available_repo_ids = cache.repo_detail.keys().map(|v| *v).collect::<Vec<_>>();
        }

        // If we have releasever, then we filter out repositories which have nothing to do with it
        if let Some(ref releasever) = data.releasever {
            available_repo_ids.retain(|oid| {
                (cache.repo_detail[oid].releasever.as_ref() == Some(&releasever)
                    || (cache.repo_detail[oid].releasever.is_none()
                    && cache.repo_detail[oid].url.contains(releasever.as_str())))
            });
            response.releasever = Some(releasever.clone())
        }

        if let Some(ref basearch) = data.basearch {
            available_repo_ids.retain(|oid| {
                (cache.repo_detail[oid].basearch.as_ref() == Some(&basearch)
                    || (cache.repo_detail[oid].basearch.is_none()
                    && cache.repo_detail[oid].url.contains(basearch.as_str())))
            });
            response.basearch = Some(basearch.clone())
        }

        return Set::from_iter(available_repo_ids);
    }

    pub fn process_input_packages<'a>(
        cache: &'a Cache,
        data: &'a Request,
        response: &mut Response,
    ) -> Map<&'a str, Nevra> {
        let mut filtered_pkgs_to_process = Map::default();

        for pkg in &data.package_list {
            if let Ok(nevra) = Nevra::from_str(pkg.as_str()) {
                if let Some(id) = cache.name_to_id.get(&nevra.name) {
                    if let Some(up) = cache.updates_index.get(id) {
                        filtered_pkgs_to_process.insert(pkg.as_str(), nevra);
                    }
                }
            } else {
                println!("Not a valid nevra {:?}", pkg)
            }
        }

        filtered_pkgs_to_process
    }

    pub fn calc_updates(cache: &Cache, data: Request) -> Result<Response> {
        let mut response = Response::default();
        let available_repo_ids = Self::process_repositories(cache, &data, &mut response);

        if let Some(ref modules_list) = data.modules_list {
            for m in modules_list {}
        }
        let mut packages_to_process = Self::process_input_packages(cache, &data, &mut response);

        for (pkg, nevra) in packages_to_process.iter() {
            response.update_list.insert(pkg.to_string(), UpdatesPkgDetail::default());
        }

        println!("Calc updates - {:?}", available_repo_ids);
        Self::process_updates(
            cache,
            &packages_to_process,
            &available_repo_ids,
            &mut response,
        )?;
        Ok(response)
    }
}