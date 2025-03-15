use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubApiAsset {
    pub url: String,
    pub id: i64,
    pub node_id: String,
    pub name: String,
    pub label: String,
    pub uploader: GithubApiUploader,
    pub content_type: String,
    pub state: String,
    pub size: i64,
    pub download_count: i64,
    pub created_at: String,
    pub updated_at: String,
    pub browser_download_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubApiUploader {
    pub login: String,
    pub id: i64,
    pub node_id: String,
    pub avatar_url: String,
    pub gravatar_id: String,
    pub url: String,
    pub html_url: String,
    pub followers_url: String,
    pub following_url: String,
    pub gists_url: String,
    pub starred_url: String,
    pub subscriptions_url: String,
    pub organizations_url: String,
    pub repos_url: String,
    pub events_url: String,
    pub received_events_url: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub user_view_type: String,
    pub site_admin: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubApiLatest {
    pub url: String,
    pub assets_url: String,
    pub upload_url: String,
    pub html_url: String,
    pub id: i64,
    pub author: GithubApiUploader,
    pub node_id: String,
    pub tag_name: String,
    pub target_commitish: String,
    pub name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: String,
    pub published_at: String,
    pub assets: Vec<GithubApiAsset>,
    pub tarball_url: String,
    pub zipball_url: String,
    pub body: String,
}
