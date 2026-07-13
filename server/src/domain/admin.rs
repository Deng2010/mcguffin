use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateGroupPayload {
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdateGroupPayload {
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetUserGroupsPayload {
    pub group_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetUserPermissionsPayload {
    pub permissions: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetAclPayload {
    #[serde(default)]
    pub visible_to: Vec<String>,
    #[serde(default)]
    pub editable_by: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetProblemAclPayload {
    #[serde(default)]
    pub editable_by: Vec<String>,
}

#[derive(Deserialize)]
pub struct ShowcaseConfigPayload {
    #[serde(default)]
    pub problem_ids: Vec<String>,
    #[serde(default)]
    pub contest_ids: Vec<String>,
}
