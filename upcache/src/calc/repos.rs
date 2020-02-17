use crate::prelude::*;
use crate::Cache;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub repository_list: Vec<String>,
    pub modified_since: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct ResRepo {
    pub label: String,
    pub name: String,
    pub url: String,
    pub basearch: String,
    pub releasever: String,
    pub product: Option<i32>,
    pub revision: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct Response {
    pub repository_list: Map<String, ResRepo>,
}

fn repos_by_regex(cache: &Cache, repo_regex: &str) -> Result<Vec<String>> {
    let re = regex::Regex::from_str(repo_regex)?;
    let mut res = vec![];
    for (l, ids) in cache.repolabel_to_ids.iter() {
        if re.is_match(&l) {
            res.push(l.clone());
        }
    }
    Ok(res)
}

pub fn get_repos(cache: &Cache, req: Request) -> Result<Response> {
    let mut res = Response::default();

    let mut repos = req.repository_list.clone();
    if repos.len() == 0 {
        return Ok(res);
    }
    if repos.len() == 1 {
        repos = repos_by_regex(cache, &repos[0])?;
    };
    Ok(res)
}