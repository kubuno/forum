use chrono::NaiveDate;
use kubuno_db::dialect::Assign;
use kubuno_db::{new_id, params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::profile_field::{
        CreateProfileFieldDto, ProfileField, ProfileFieldValue, SetProfileValuesDto,
        UpdateProfileFieldDto,
    },
};

/// Maps a unique-key collision to a friendly conflict, everything else through.
fn map_field_error(e: sqlx::Error, ctx: &str) -> ForumError {
    match e {
        sqlx::Error::Database(ref d) if d.is_unique_violation() => {
            ForumError::Conflict("un champ avec cette clé existe déjà".into())
        }
        other => {
            tracing::error!(error = %other, "{ctx}");
            ForumError::Database(other)
        }
    }
}

pub struct ProfileFieldService;

impl ProfileFieldService {
    // ── Definitions (admin-curated) ──────────────────────────────────────────

    pub async fn list_fields(db: &DbPool) -> Result<Vec<ProfileField>> {
        let rows = db
            .fetch_all_as::<ProfileField>(
                "SELECT * FROM forum.profile_fields ORDER BY position, created_at",
                params![],
            )
            .await?;
        Ok(rows)
    }

    async fn get_field(id: Uuid, db: &DbPool) -> Result<ProfileField> {
        db.fetch_optional_as::<ProfileField>(
            "SELECT * FROM forum.profile_fields WHERE id = $1",
            params![id],
        )
        .await?
        .ok_or_else(|| ForumError::NotFound(format!("Profile field {id}")))
    }

    /// `options` only ever survives on a `dropdown` field: for any other
    /// `field_type` it is silently dropped, so a field that used to be a
    /// dropdown never keeps stale options once switched away from it.
    fn resolve_options(field_type: &str, options: Option<Vec<String>>) -> Result<Option<serde_json::Value>> {
        if field_type != "dropdown" {
            return Ok(None);
        }
        let options = options.unwrap_or_default();
        let cleaned: Vec<String> = options.iter().map(|o| o.trim().to_string()).filter(|o| !o.is_empty()).collect();
        if cleaned.is_empty() {
            return Err(ForumError::Validation(
                "un champ de type liste déroulante doit avoir au moins une option".into(),
            ));
        }
        Ok(Some(serde_json::json!(cleaned)))
    }

    pub async fn create_field(dto: CreateProfileFieldDto, db: &DbPool) -> Result<ProfileField> {
        let options = Self::resolve_options(&dto.field_type, dto.options)?;
        let id = new_id();
        db.execute(
            "INSERT INTO forum.profile_fields \
                (id, field_key, label, field_type, options, position, visibility, show_on_posts, required) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            params![
                id,
                dto.key,
                dto.label.trim(),
                dto.field_type,
                options,
                dto.position,
                dto.visibility,
                dto.show_on_posts,
                dto.required
            ],
        )
        .await
        .map_err(|e| map_field_error(e, "Creating a profile field"))?;
        Self::get_field(id, db).await
    }

    /// Replaces the whole definition, merging onto the existing row for any
    /// column the caller left out — see the DTO's doc comment for why this is
    /// a merge-then-overwrite rather than a SQL `COALESCE`.
    pub async fn update_field(id: Uuid, dto: UpdateProfileFieldDto, db: &DbPool) -> Result<ProfileField> {
        let existing = Self::get_field(id, db).await?;
        let key = dto.key.unwrap_or(existing.key);
        let label = dto.label.unwrap_or(existing.label);
        let field_type = dto.field_type.unwrap_or(existing.field_type);
        let visibility = dto.visibility.unwrap_or(existing.visibility);
        let position = dto.position.unwrap_or(existing.position);
        let show_on_posts = dto.show_on_posts.unwrap_or(existing.show_on_posts);
        let required = dto.required.unwrap_or(existing.required);
        // Only re-derive `options` from the caller's payload when the caller
        // actually sent one; otherwise fall back to whatever the field
        // already had (still re-validated/cleared against the final type).
        let options_input = dto.options.or_else(|| {
            existing
                .options
                .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
        });
        let options = Self::resolve_options(&field_type, options_input)?;

        let affected = db
            .execute(
                "UPDATE forum.profile_fields SET \
                    field_key = $1, label = $2, field_type = $3, options = $4, \
                    position = $5, visibility = $6, show_on_posts = $7, required = $8 \
                 WHERE id = $9",
                params![
                    key,
                    label.trim(),
                    field_type,
                    options,
                    position,
                    visibility,
                    show_on_posts,
                    required,
                    id
                ],
            )
            .await
            .map_err(|e| map_field_error(e, "Updating a profile field"))?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Profile field {id}")));
        }
        Self::get_field(id, db).await
    }

    pub async fn delete_field(id: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute("DELETE FROM forum.profile_fields WHERE id = $1", params![id])
            .await
            .inspect_err(|e| tracing::error!(error = %e, %id, "Deleting a profile field"))?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Profile field {id}")));
        }
        Ok(())
    }

    // ── Values (per member) ───────────────────────────────────────────────────

    pub async fn get_values(user_id: Uuid, db: &DbPool) -> Result<Vec<ProfileFieldValue>> {
        let rows = db
            .fetch_all_as::<ProfileFieldValue>(
                "SELECT * FROM forum.profile_field_values WHERE user_id = $1",
                params![user_id],
            )
            .await?;
        Ok(rows)
    }

    /// Validates one answer against its field's declared type. Called with an
    /// already-trimmed, non-empty `value`.
    fn validate_value(field: &ProfileField, value: &str) -> Result<()> {
        match field.field_type.as_str() {
            "bool" if value != "true" && value != "false" => {
                return Err(ForumError::Validation(format!(
                    "« {} » doit être vrai ou faux",
                    field.label
                )));
            }
            "url" => {
                let lower_ok = value.starts_with("http://") || value.starts_with("https://");
                if !lower_ok || value.chars().any(|c| c.is_whitespace()) || value.chars().count() > 2000 {
                    return Err(ForumError::Validation(format!(
                        "« {} » doit être un lien http:// ou https:// valide",
                        field.label
                    )));
                }
            }
            "date" if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() => {
                return Err(ForumError::Validation(format!(
                    "« {} » doit être une date au format AAAA-MM-JJ",
                    field.label
                )));
            }
            "dropdown" => {
                let options: Vec<String> = field
                    .options
                    .clone()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                if !options.iter().any(|o| o == value) {
                    return Err(ForumError::Validation(format!(
                        "« {} » : valeur non autorisée",
                        field.label
                    )));
                }
            }
            "text" if value.chars().count() > 200 => {
                return Err(ForumError::Validation(format!(
                    "« {} » : texte trop long (200 caractères maximum)",
                    field.label
                )));
            }
            _ => {}
        }
        Ok(())
    }

    /// Upserts the caller's own answers. A field omitted from `dto.values`
    /// keeps whatever it already had; a field submitted with an empty
    /// (trimmed) value has its stored answer removed instead.
    pub async fn set_values(user_id: Uuid, dto: SetProfileValuesDto, db: &DbPool) -> Result<Vec<ProfileFieldValue>> {
        let fields = Self::list_fields(db).await?;
        let b = db.backend();
        let upsert_sql = format!(
            "INSERT INTO forum.profile_field_values (user_id, field_id, value) VALUES ($1, $2, $3){}",
            b.upsert("profile_field_values", &["user_id", "field_id"], &[Assign::Incoming("value")])
        );

        let mut tx = db.begin().await?;

        for input in &dto.values {
            let Some(field) = fields.iter().find(|f| f.id == input.field_id) else {
                return Err(ForumError::NotFound(format!("Profile field {}", input.field_id)));
            };
            let trimmed = input.value.trim();

            if trimmed.is_empty() {
                if field.required {
                    return Err(ForumError::Validation(format!("« {} » est obligatoire", field.label)));
                }
                tx.execute(
                    "DELETE FROM forum.profile_field_values WHERE user_id = $1 AND field_id = $2",
                    params![user_id, field.id],
                )
                .await
                .inspect_err(|e| tracing::error!(error = %e, %user_id, field_id = %field.id, "Clearing a profile field value"))?;
                continue;
            }

            Self::validate_value(field, trimmed)?;

            tx.execute(&upsert_sql, params![user_id, field.id, trimmed])
                .await
                .inspect_err(|e| tracing::error!(error = %e, %user_id, field_id = %field.id, "Saving a profile field value"))?;
        }

        tx.commit().await?;
        Self::get_values(user_id, db).await
    }

    /// Values of `target_user_id` fit for showing on their profile to `viewer`.
    pub async fn values_for_display(
        target_user_id: Uuid,
        viewer: Option<&ForumUser>,
        db: &DbPool,
    ) -> Result<Vec<(ProfileField, String)>> {
        let fields = Self::list_fields(db).await?;
        let values = Self::get_values(target_user_id, db).await?;
        let is_owner_or_admin = viewer.is_some_and(|v| v.id == target_user_id || v.is_admin());
        let is_authenticated = viewer.is_some();

        let mut out = Vec::new();
        for field in fields {
            let visible = is_owner_or_admin
                || field.visibility == "public"
                || (field.visibility == "registered" && is_authenticated);
            if !visible {
                continue;
            }
            if let Some(v) = values.iter().find(|v| v.field_id == field.id) {
                out.push((field, v.value.clone()));
            }
        }
        Ok(out)
    }
}
