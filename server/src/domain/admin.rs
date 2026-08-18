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

/// 展板组件化布局载荷。布局结构（schema_version / components[].settings / size / position）
/// 由前端定义并校验，后端仅做持久化 —— 新增组件类型无需改动后端。
#[derive(Deserialize)]
pub struct ShowcaseLayoutPayload {
    pub layout: serde_json::Value,
}
