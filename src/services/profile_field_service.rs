use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::profile_field::{
        CreateProfileFieldDto, ProfileField, ProfileFieldValue, SetProfileValuesDto,
        UpdateProfileFieldDto,
    },
};

pub struct ProfileFieldService;

impl ProfileFieldService {
    // ── Definitions (admin-curated) ──────────────────────────────────────────

    pub async fn list_fields(db: &PgPool) -> Result<Vec<ProfileField>> {
        let rows = sqlx::query_as::<_, ProfileField>(
            "SELECT * FROM forum.profile_fields ORDER BY position, created_at",
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    async fn get_field(id: Uuid, db: &PgPool) -> Result<ProfileField> {
        sqlx::query_as::<_, ProfileField>("SELECT * FROM forum.profile_fields WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
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

    pub async fn create_field(dto: CreateProfileFieldDto, db: &PgPool) -> Result<ProfileField> {
        let options = Self::resolve_options(&dto.field_type, dto.options)?;
        sqlx::query_as::<_, ProfileField>(
            "INSERT INTO forum.profile_fields (key, label, field_type, options, position, visibility, show_on_posts, required)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
        )
        .bind(&dto.key)
        .bind(dto.label.trim())
        .bind(&dto.field_type)
        .bind(&options)
        .bind(dto.position)
        .bind(&dto.visibility)
        .bind(dto.show_on_posts)
        .bind(dto.required)
        .fetch_one(db)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.is_unique_violation() => {
                ForumError::Conflict("un champ avec cette clé existe déjà".into())
            }
            other => {
                tracing::error!(error = %other, "Creating a profile field");
                ForumError::Database(other)
            }
        })
    }

    /// Replaces the whole definition, merging onto the existing row for any
    /// column the caller left out — see the DTO's doc comment for why this is
    /// a merge-then-overwrite rather than a SQL `COALESCE`.
    pub async fn update_field(id: Uuid, dto: UpdateProfileFieldDto, db: &PgPool) -> Result<ProfileField> {
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

        sqlx::query_as::<_, ProfileField>(
            "UPDATE forum.profile_fields SET
                key = $2, label = $3, field_type = $4, options = $5,
                position = $6, visibility = $7, show_on_posts = $8, required = $9
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(&key)
        .bind(label.trim())
        .bind(&field_type)
        .bind(&options)
        .bind(position)
        .bind(&visibility)
        .bind(show_on_posts)
        .bind(required)
        .fetch_optional(db)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.is_unique_violation() => {
                ForumError::Conflict("un champ avec cette clé existe déjà".into())
            }
            other => {
                tracing::error!(error = %other, %id, "Updating a profile field");
                ForumError::Database(other)
            }
        })?
        .ok_or_else(|| ForumError::NotFound(format!("Profile field {id}")))
    }

    pub async fn delete_field(id: Uuid, db: &PgPool) -> Result<()> {
        let r = sqlx::query("DELETE FROM forum.profile_fields WHERE id = $1")
            .bind(id)
            .execute(db)
            .await
            .inspect_err(|e| tracing::error!(error = %e, %id, "Deleting a profile field"))?;
        if r.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("Profile field {id}")));
        }
        Ok(())
    }

    // ── Values (per member) ───────────────────────────────────────────────────

    pub async fn get_values(user_id: Uuid, db: &PgPool) -> Result<Vec<ProfileFieldValue>> {
        let rows = sqlx::query_as::<_, ProfileFieldValue>(
            "SELECT * FROM forum.profile_field_values WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Validates one answer against its field's declared type. Called with an
    /// already-trimmed, non-empty `value` — an empty answer is handled by the
    /// caller before this ever runs (it clears the stored value instead).
    fn validate_value(field: &ProfileField, value: &str) -> Result<()> {
        match field.field_type.as_str() {
            "bool" if value != "true" && value != "false" => {
                return Err(ForumError::Validation(format!(
                    "« {} » doit être vrai ou faux",
                    field.label
                )));
            }
            "url" => {
                // Server-side mirror of the frontend's `safeUrl` allow-list
                // (PostBody.tsx), restricted further to http(s) only: a
                // custom profile field is not a Markdown link, just a bare
                // address, so there is no reason to also accept mailto/tel
                // here. A crude prefix + no-whitespace check is enough — this
                // is a display value, not something ever dereferenced
                // server-side.
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
            // "textarea" and any future type default to the DTO-level 4000
            // character ceiling already enforced before this is reached.
            _ => {}
        }
        Ok(())
    }

    /// Upserts the caller's own answers. A field omitted from `dto.values`
    /// keeps whatever it already had; a field submitted with an empty
    /// (trimmed) value has its stored answer removed instead of storing an
    /// empty string — that is the one way to clear an optional field, since
    /// `required` fields reject an empty value outright.
    pub async fn set_values(user_id: Uuid, dto: SetProfileValuesDto, db: &PgPool) -> Result<Vec<ProfileFieldValue>> {
        let fields = Self::list_fields(db).await?;
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
                sqlx::query("DELETE FROM forum.profile_field_values WHERE user_id = $1 AND field_id = $2")
                    .bind(user_id)
                    .bind(field.id)
                    .execute(&mut *tx)
                    .await
                    .inspect_err(|e| tracing::error!(error = %e, %user_id, field_id = %field.id, "Clearing a profile field value"))?;
                continue;
            }

            Self::validate_value(field, trimmed)?;

            sqlx::query(
                "INSERT INTO forum.profile_field_values (user_id, field_id, value)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (user_id, field_id) DO UPDATE SET value = EXCLUDED.value",
            )
            .bind(user_id)
            .bind(field.id)
            .bind(trimmed)
            .execute(&mut *tx)
            .await
            .inspect_err(|e| tracing::error!(error = %e, %user_id, field_id = %field.id, "Saving a profile field value"))?;
        }

        tx.commit().await?;
        Self::get_values(user_id, db).await
    }

    /// Values of `target_user_id` fit for showing on their profile to
    /// `viewer`: the target themselves and any admin always see everything;
    /// otherwise a `public` field always shows and a `registered` one shows
    /// only to an authenticated `viewer`. Every route in this module already
    /// requires auth (`viewer` is always `Some` in practice today), so this
    /// currently never hides a `registered` field — the `Option` exists so
    /// the distinction still means something the day a guest/public read
    /// path is added in front of this service.
    pub async fn values_for_display(
        target_user_id: Uuid,
        viewer: Option<&ForumUser>,
        db: &PgPool,
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
