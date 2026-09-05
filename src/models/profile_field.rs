use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use validator::{Validate, ValidationError};

/// The allowed field kinds ("custom profile fields", kept
/// intentionally small — no numeric/checkbox-group/etc.).
const FIELD_TYPES: &[&str] = &["text", "textarea", "bool", "url", "date", "dropdown"];
const VISIBILITIES: &[&str] = &["public", "registered"];

/// An admin-curated custom profile field definition. Answers are ALWAYS
/// rendered as plain, escaped text at display time (see
/// `services::profile_field_service::values_for_display`) — never Markdown or
/// HTML — so a member's own input can never inject anything into another
/// member's view of their profile.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProfileField {
    pub id:            Uuid,
    pub key:           String,
    pub label:         String,
    pub field_type:    String,
    /// Only meaningful for `field_type == "dropdown"`: a JSON array of the
    /// allowed option strings.
    pub options:       Option<Value>,
    pub position:      i32,
    pub visibility:    String,
    pub show_on_posts: bool,
    pub required:      bool,
    pub created_at:    DateTime<Utc>,
}

/// One member's answer to one field. `value` is always plain text, whatever
/// the field's declared type (see the migration's comment on the column).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProfileFieldValue {
    pub user_id:  Uuid,
    pub field_id: Uuid,
    pub value:    String,
}

fn validate_field_key(key: &str) -> Result<(), ValidationError> {
    let ok = !key.is_empty()
        && key.len() <= 40
        && key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if ok {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_field_key"))
    }
}

fn validate_field_type(field_type: &str) -> Result<(), ValidationError> {
    if FIELD_TYPES.contains(&field_type) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_field_type"))
    }
}

fn validate_visibility(visibility: &str) -> Result<(), ValidationError> {
    if VISIBILITIES.contains(&visibility) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_visibility"))
    }
}

/// Bounds the options list itself (not what a member later picks from it):
/// at most 50 choices, each 1..200 characters once trimmed. The per-value
/// membership check (a submitted answer must be one of these) happens in the
/// service, against the field's *stored* options, not this DTO. The field
/// this validates is `Option<Vec<String>>` — `None` (option omitted) is
/// always fine, the service decides afterwards whether the *merged*
/// `field_type` actually required one.
fn validate_field_options(options: &[String]) -> Result<(), ValidationError> {
    if options.len() > 50 {
        return Err(ValidationError::new("too_many_options"));
    }
    for opt in options {
        let trimmed = opt.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 200 {
            return Err(ValidationError::new("invalid_option"));
        }
    }
    Ok(())
}

fn default_visibility() -> String {
    "public".to_string()
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProfileFieldDto {
    #[validate(custom(function = "validate_field_key"))]
    pub key:      String,
    #[validate(length(min = 1, max = 80))]
    pub label:    String,
    #[validate(custom(function = "validate_field_type"))]
    pub field_type: String,
    /// Required (and non-empty) when `field_type == "dropdown"`, ignored
    /// (and cleared) otherwise — enforced in the service, since it depends on
    /// another field of this same DTO.
    #[serde(default)]
    #[validate(custom(function = "validate_field_options"))]
    pub options:  Option<Vec<String>>,
    #[serde(default)]
    pub position: i32,
    #[serde(default = "default_visibility")]
    #[validate(custom(function = "validate_visibility"))]
    pub visibility: String,
    #[serde(default)]
    pub show_on_posts: bool,
    #[serde(default)]
    pub required: bool,
}

/// A full replacement of an existing field's definition (the admin UI edits
/// the whole record in place, like `ForumEditWindow` does for forums) — every
/// field is optional here only so a caller may omit one it did not change;
/// the service merges onto the existing row rather than doing a SQL
/// `COALESCE`, so it can re-derive `options` from the *final* `field_type`.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileFieldDto {
    #[validate(custom(function = "validate_field_key"))]
    pub key:      Option<String>,
    #[validate(length(min = 1, max = 80))]
    pub label:    Option<String>,
    #[validate(custom(function = "validate_field_type"))]
    pub field_type: Option<String>,
    #[serde(default)]
    #[validate(custom(function = "validate_field_options"))]
    pub options:  Option<Vec<String>>,
    pub position: Option<i32>,
    #[validate(custom(function = "validate_visibility"))]
    pub visibility: Option<String>,
    pub show_on_posts: Option<bool>,
    pub required: Option<bool>,
}

/// One (field, answer) pair submitted by a member for their own profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileFieldValueInput {
    pub field_id: Uuid,
    pub value:    String,
}

fn validate_values(values: &[ProfileFieldValueInput]) -> Result<(), ValidationError> {
    if values.len() > 200 {
        return Err(ValidationError::new("too_many_values"));
    }
    for v in values {
        if v.value.chars().count() > 4000 {
            return Err(ValidationError::new("value_too_long"));
        }
    }
    Ok(())
}

/// Body of `PUT /me/profile-fields`: the caller's own answers, one per field
/// they are setting (an omitted `field_id` keeps its previously stored
/// value untouched — see `ProfileFieldService::set_values`).
#[derive(Debug, Deserialize, Validate)]
pub struct SetProfileValuesDto {
    #[validate(custom(function = "validate_values"))]
    pub values: Vec<ProfileFieldValueInput>,
}
