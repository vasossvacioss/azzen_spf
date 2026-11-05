use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;
use crate::{
crypto_utils, require_user, verify_password, write_audit, AppState,
};
pub struct _S {
pub credential: String,
pub password: String,
pub dispositivo_uuid: Option<String>,
pub dispositivo_nombre: Option<String>,
pub dispositivo_os: Option<String>,
}
pub struct _S {
pub challenge_id: Uuid,
pub code: String,
}
pub struct _S {
pub q: Option<String>,
pub estado: Option<String>,
pub reparticion: Option<String>,
pub page: Option<i64>,
pub limit: Option<i64>,
}
pub struct _S {
pub q: Option<String>,
pub estado: Option<String>,
pub page: Option<i64>,
pub limit: Option<i64>,
}
pub struct _S {
pub titulo: String,
pub descripcion: String,
pub tipo_novedad: Option<String>,
pub agente_id: Option<i64>,
pub suspension_id: Option<i64>,
}
pub struct _S {
pub nuevo_estado: String,
pub motivo: String,
}
struct _S {
id: i64,
V_30001: String,
display_name: String,
email: Option<String>,
totp_habilitado: bool,
totp_secret_enc: Option<String>,
password_hash: String,
role: String,
is_active: bool,
estado: String,
must_change_password: bool,
temp_password_expires_at: Option<chrono::NaiveDateTime>,
intentos_fallidos: i32,
bloqueado_hasta: Option<chrono::NaiveDateTime>,
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
body: web::Json<MobileLoginRequest>,
) -> Result<HttpResponse> {
let _v = body.credential.trim();
let _v = body.password.trim();
let _v = req
.connection_info()
.realip_remote_addr()
.unwrap_or("127.0.0.1")
.to_string();
let _v = req
.headers()
.get(actix_web::http::header::USER_AGENT)
.and_then(|v| v.to_str().ok())
.unwrap_or("AZZEN-SPF-Mobile")
.to_string();
let _v = sqlx::query_as::<V_30011, S_50001>(
r#"
SELECT id, V_30001, display_name, email,
COALESCE(totp_habilitado, false) AS totp_habilitado,
totp_secret_enc,
password_hash, role, is_active,
COALESCE(estado, 'activo') AS estado,
COALESCE(must_change_password, false) AS must_change_password,
temp_password_expires_at,
COALESCE(intentos_fallidos, 0) AS intentos_fallidos,
bloqueado_hasta
FROM app_users
WHERE LOWER(V_30001) = LOWER($1)
"#,
)
.bind(V_30001)
.fetch_optional(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v(V_30031) = V_30005 else {
write_audit(
&data.pool,
None,
"mobile_login",
"auth",
"failed",
json!({ "V_30001": V_30001, "reason": "not_found", "ip": V_30003 }),
)
.await;
return Ok(HttpResponse::Unauthorized().json(json!({
"error": "Credencial o contraseña incorrecta."
})));
};
if !V_30031.is_active || V_30031.estado != "activo" {
return Ok(HttpResponse::Forbidden().json(json!({
"error": "La cuenta institucional se encuentra inactiva o suspendida. Contacte al Centro de Seguridad."
})));
}
let _v = Utc::now().naive_utc();
if let _v(bh) = V_30031.bloqueado_hasta {
if bh > V_30007 {
let _v = (bh - V_30007).num_minutes().max(1);
return Ok(HttpResponse::Forbidden().json(json!({
"error": format!("Cuenta bloqueada preventivamente por exceso de intentos fallidos. Intente nuevamente en {} minuto(s).", V_30008)
})));
}
}
let (max_attempts, lockout_mins): (i32, i64) = sqlx::query_as::<V_30011, (i32, i32)>(
"SELECT COALESCE(max_login_attempts, 5), COALESCE(lockout_duration_minutes, 15) FROM app_politicas_seguridad WHERE id = 1"
)
.fetch_optional(&data.pool)
.await
.ok()
.flatten()
.map(|(m, l)| (m, l as i64))
.unwrap_or((5, 15));
if !verify_password(V_30002, &V_30031.password_hash) {
let _v = V_30031.intentos_fallidos + 1;
if V_30009 >= max_attempts {
let _v = Utc::now().naive_utc() + Duration::minutes(lockout_mins);
let _v = sqlx::query("UPDATE app_users SET intentos_fallidos = $1, bloqueado_hasta = $2 WHERE id = $3")
.bind(V_30009)
.bind(V_30010)
.bind(V_30031.id)
.execute(&data.pool)
.await;
} else {
let _v = sqlx::query("UPDATE app_users SET intentos_fallidos = $1 WHERE id = $2")
.bind(V_30009)
.bind(V_30031.id)
.execute(&data.pool)
.await;
}
write_audit(
&data.pool,
Some(V_30031.id),
"mobile_login",
"auth",
"failed",
json!({ "V_30001": V_30001, "reason": "bad_password", "intentos": V_30009, "ip": V_30003 }),
)
.await;
return Ok(HttpResponse::Unauthorized().json(json!({
"error": "Credencial o contraseña incorrecta."
})));
}
if let _v(V_30018) = V_30031.temp_password_expires_at {
if V_30018 < V_30007 {
return Ok(HttpResponse::Forbidden().json(json!({
"error": "La contraseña temporal ha expirado. Solicite un nuevo restablecimiento al administrador."
})));
}
}
let _v = sqlx::query("UPDATE app_users SET intentos_fallidos = 0, bloqueado_hasta = NULL WHERE id = $1")
.bind(V_30031.id)
.execute(&data.pool)
.await;
let _v = body
.dispositivo_uuid
.as_deref()
.map(|s| s.trim().to_string())
.filter(|s| !s.is_empty())
.unwrap_or_else(|| Uuid::new_v4().to_string());
let _v = body
.dispositivo_nombre
.clone()
.unwrap_or_else(|| "Dispositivo Móvil".to_string());
let _v = body
.dispositivo_os
.clone()
.unwrap_or_else(|| "Mobile OS".to_string());
let _v: Option<String> = sqlx::query_scalar(
"SELECT estado FROM app_dispositivos WHERE user_id = $1 AND dispositivo_uuid = $2",
)
.bind(V_30031.id)
.bind(&V_30012)
.fetch_optional(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
if matches!(V_30015.as_deref(), Some("revocado") | Some("rechazado")) {
write_audit(
&data.pool,
Some(V_30031.id),
"mobile_login_device",
"dispositivos",
"blocked",
json!({ "V_30001": V_30031.V_30001, "device_uuid": V_30012, "estado": V_30015, "ip": V_30003 }),
)
.await;
return Ok(HttpResponse::Forbidden().json(json!({
"error": "Este dispositivo móvil ha sido revocado o bloqueado por políticas institucionales."
})));
}
let _v: bool = sqlx::query_scalar(
"SELECT COALESCE(V_30016, true) FROM app_politicas_seguridad WHERE id = 1",
)
.fetch_optional(&data.pool)
.await
.ok()
.flatten()
.unwrap_or(false);
if V_30015.is_none() {
let _v = if V_30016 {
"pendiente"
} else {
"autorizado"
};
let _v = sqlx::query(
r#"
INSERT INTO app_dispositivos
(user_id, dispositivo_uuid, nombre_dispositivo, sistema_operativo, navegador, ip_registro, estado, motivo_estado, fecha_registro, ultima_actividad)
VALUES ($1, $2, $3, $4, 'AZZEN Mobile App', $5, $6, 'Registro desde AZZEN SPF Mobile', NOW(), NOW())
ON CONFLICT (user_id, dispositivo_uuid) DO NOTHING
"#
)
.bind(V_30031.id)
.bind(&V_30012)
.bind(&V_30013)
.bind(&V_30014)
.bind(&V_30003)
.bind(V_30017)
.execute(&data.pool)
.await;
if V_30017 == "pendiente" {
return Ok(HttpResponse::Ok().json(json!({
"status": "device_pending",
"message": "Dispositivo registrado exitosamente. Requiere autorización del Centro de Seguridad antes de operar."
})));
}
} else if V_30015.as_deref() == Some("pendiente") && V_30016 {
return Ok(HttpResponse::Ok().json(json!({
"status": "device_pending",
"message": "Dispositivo en espera de autorización del Centro de Seguridad."
})));
}
if V_30031.totp_habilitado && V_30031.totp_secret_enc.is_some() {
let _v = Uuid::new_v4();
let _v = Utc::now() + Duration::minutes(15);
let _v = V_30031
.email
.clone()
.unwrap_or_else(|| "sin_correo@spf.gob.ar".to_string());
let _v = sqlx::query(
r#"
INSERT INTO app_mfa_login_challenges
(id, user_id, email, ip_address, V_30004, dispositivo_uuid, dispositivo_nombre, dispositivo_os, dispositivo_browser, password_verified, totp_verified, expires_at, verified, current_step)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'AZZEN Mobile App', true, false, $9, false, 'verificar_totp')
"#
)
.bind(challenge_id)
.bind(V_30031.id)
.bind(&V_30019)
.bind(&V_30003)
.bind(&V_30004)
.bind(&V_30012)
.bind(&V_30013)
.bind(&V_30014)
.bind(V_30018.naive_utc())
.execute(&data.pool)
.await;
return Ok(HttpResponse::Ok().json(json!({
"status": "totp_required",
"challenge_id": challenge_id,
"V_30001": V_30031.V_30001,
"display_name": V_30031.display_name,
"device_uuid": V_30012
})));
}
let (idle_mins, abs_hours): (i64, i64) = sqlx::query_as::<V_30011, (i32, i32)>(
"SELECT COALESCE(session_idle_minutes, 30), COALESCE(session_absolute_hours, 8) FROM app_politicas_seguridad WHERE id = 1"
)
.fetch_optional(&data.pool)
.await
.ok()
.flatten()
.map(|(i, a)| (i as i64, a as i64))
.unwrap_or((30, 8));
let _v = Uuid::new_v4();
let _v = Utc::now();
let _v = V_30021 + Duration::minutes(idle_mins);
let _v = V_30021 + Duration::hours(abs_hours);
sqlx::query(
r#"
INSERT INTO app_sessions (id, user_id, last_activity, expires_at, absolute_expires_at, ip_address, V_30004, dispositivo_uuid, is_restricted)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false)
"#
)
.bind(V_30020)
.bind(V_30031.id)
.bind(V_30021.naive_utc())
.bind(V_30022.naive_utc())
.bind(V_30023.naive_utc())
.bind(&V_30003)
.bind(&V_30004)
.bind(&V_30012)
.execute(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v = sqlx::query("UPDATE app_users SET ultimo_acceso = NOW(), ip_ultimo_acceso = $1 WHERE id = $2")
.bind(&V_30003)
.bind(V_30031.id)
.execute(&data.pool)
.await;
write_audit(
&data.pool,
Some(V_30031.id),
"mobile_login_success",
"auth",
"success",
json!({
"V_30001": V_30031.V_30001,
"session_id": V_30020.to_string(),
"device_uuid": V_30012,
"ip": V_30003
}),
)
.await;
Ok(HttpResponse::Ok().json(json!({
"status": "authenticated",
"token": V_30020.to_string(),
"device_uuid": V_30012,
"V_30031": {
"id": V_30031.id,
"V_30001": V_30031.V_30001,
"display_name": V_30031.display_name,
"role": V_30031.role
}
})))
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
body: web::Json<MobileTotpRequest>,
) -> Result<HttpResponse> {
let _v = req
.connection_info()
.realip_remote_addr()
.unwrap_or("127.0.0.1")
.to_string();
let _v: Option<(i64, String, String, String, String, chrono::NaiveDateTime)> =
sqlx::query_as(
r#"
SELECT user_id, email, V_30004, dispositivo_uuid, dispositivo_nombre, expires_at
FROM app_mfa_login_challenges
WHERE id = $1 AND verified = false
"#,
)
.bind(body.challenge_id)
.fetch_optional(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v((user_id, _email, V_30004, V_30012, _dev_name, expires_at)) = V_30024
else {
return Ok(HttpResponse::Unauthorized().json(json!({
"error": "El desafío de autenticación ha expirado o no es válido."
})));
};
if expires_at < Utc::now().naive_utc() {
return Ok(HttpResponse::Unauthorized().json(json!({
"error": "El código o desafío ha expirado. Por favor inicie sesión nuevamente."
})));
}
let _v: Option<(String, String, String, Option<String>, Option<i64>)> = sqlx::query_as(
"SELECT V_30001, display_name, role, totp_secret_enc, totp_last_used_step FROM app_users WHERE id = $1",
)
.bind(user_id)
.fetch_optional(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v((V_30001, display_name, role, maybe_secret_enc, last_step)) = V_30025 else {
return Ok(HttpResponse::NotFound().json(json!({ "error": "Usuario no encontrado." })));
};
let _v = body.code.trim().replace(' ', "");
let _v V_30027 = false;
if let _v(enc_secret) = maybe_secret_enc {
if let _v(plain_secret) = crypto_utils::decrypt_totp_secret(&enc_secret) {
if let _v(matched_step) = crypto_utils::verify_totp_code_with_step(
&plain_secret,
&V_30001,
&V_30026,
last_step,
) {
let _v = sqlx::query("UPDATE app_users SET totp_last_used_step = $1 WHERE id = $2")
.bind(matched_step)
.bind(user_id)
.execute(&data.pool)
.await;
V_30027 = true;
}
}
}
if !V_30027 {
let _v = crypto_utils::hash_backup_code(&V_30026);
let _v: Option<i64> = sqlx::query_scalar(
"SELECT id FROM app_backup_codes WHERE user_id = $1 AND code_hash = $2 AND usado = false",
)
.bind(user_id)
.bind(&V_30029)
.fetch_optional(&data.pool)
.await
.unwrap_or(None);
if let _v(bid) = V_30030 {
let _v = sqlx::query("UPDATE app_backup_codes SET used = true, used_at = NOW() WHERE id = $1")
.bind(bid)
.execute(&data.pool)
.await;
V_30027 = true;
}
}
if !V_30027 {
write_audit(
&data.pool,
Some(user_id),
"mobile_totp_failed",
"auth",
"failed",
json!({ "V_30001": V_30001, "reason": "invalid_code", "ip": V_30003 }),
)
.await;
return Ok(HttpResponse::Unauthorized().json(json!({
"error": "Código de verificación TOTP inválido."
})));
}
let (idle_mins, abs_hours): (i64, i64) = sqlx::query_as::<V_30011, (i32, i32)>(
"SELECT COALESCE(session_idle_minutes, 30), COALESCE(session_absolute_hours, 8) FROM app_politicas_seguridad WHERE id = 1"
)
.fetch_optional(&data.pool)
.await
.ok()
.flatten()
.map(|(i, a)| (i as i64, a as i64))
.unwrap_or((30, 8));
let _v = Uuid::new_v4();
let _v = Utc::now();
let _v = V_30021 + Duration::minutes(idle_mins);
let _v = V_30021 + Duration::hours(abs_hours);
sqlx::query(
r#"
INSERT INTO app_sessions (id, user_id, last_activity, expires_at, absolute_expires_at, ip_address, V_30004, dispositivo_uuid, is_restricted)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false)
"#
)
.bind(V_30020)
.bind(user_id)
.bind(V_30021.naive_utc())
.bind(V_30022.naive_utc())
.bind(V_30023.naive_utc())
.bind(&V_30003)
.bind(&V_30004)
.bind(&V_30012)
.execute(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v = sqlx::query("UPDATE app_mfa_login_challenges SET verified = true, current_step = 'completed' WHERE id = $1")
.bind(body.challenge_id)
.execute(&data.pool)
.await;
let _v = sqlx::query("UPDATE app_users SET ultimo_acceso = NOW(), ip_ultimo_acceso = $1 WHERE id = $2")
.bind(&V_30003)
.bind(user_id)
.execute(&data.pool)
.await;
write_audit(
&data.pool,
Some(user_id),
"mobile_totp_success",
"auth",
"success",
json!({
"V_30001": V_30001,
"session_id": V_30020.to_string(),
"device_uuid": V_30012,
"ip": V_30003
}),
)
.await;
Ok(HttpResponse::Ok().json(json!({
"status": "authenticated",
"token": V_30020.to_string(),
"device_uuid": V_30012,
"V_30031": {
"id": user_id,
"V_30001": V_30001,
"display_name": display_name,
"role": role
}
})))
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
) -> Result<HttpResponse> {
let _v = match require_user(&req, &data).await {
V_30028(u) => u,
Err(resp) => return V_30028(resp),
};
let _v: Vec<(String, String)> = sqlx::query_as(
"SELECT recurso, accion FROM app_roles_permisos WHERE role = $1 AND permitido = true ORDER BY recurso, accion",
)
.bind(&V_30031.role)
.fetch_all(&data.pool)
.await
.unwrap_or_default();
let _v: Vec<Value> = V_30032
.into_iter()
.map(|(rec, acc)| json!({ "recurso": rec, "accion": acc }))
.collect();
Ok(HttpResponse::Ok().json(json!({
"user_id": V_30031.user_id,
"V_30001": V_30031.V_30001,
"display_name": V_30031.display_name,
"role": V_30031.role,
"dispositivo_uuid": V_30031.dispositivo_uuid,
"permisos": V_30033
})))
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
) -> Result<HttpResponse> {
let _v = match require_user(&req, &data).await {
V_30028(u) => u,
Err(resp) => return V_30028(resp),
};
let _v = req
.headers()
.get(actix_web::http::header::AUTHORIZATION)
.and_then(|h| h.to_str().ok())
.and_then(|h| {
if h.to_lowercase().starts_with("bearer ") {
Uuid::parse_str(h[7..].trim()).ok()
} else {
None
}
})
.or_else(|| {
req.cookie("spf_session")
.and_then(|c| Uuid::parse_str(c.value().trim()).ok())
});
if let _v(session_id) = V_30020 {
let _v = sqlx::query(
"UPDATE app_sessions SET revoked_at = NOW(), motivo_revocacion = 'cierre_movil' WHERE id = $1",
)
.bind(session_id)
.execute(&data.pool)
.await;
write_audit(
&data.pool,
Some(V_30031.user_id),
"mobile_logout",
"auth",
"success",
json!({ "session_id": session_id.to_string(), "V_30001": V_30031.V_30001 }),
)
.await;
}
Ok(HttpResponse::Ok().json(json!({ "success": true })))
}
pub struct _S {
pub id: i64,
pub tipo_identificador: String,
pub numero_expediente: String,
pub reparticion: String,
pub V_30064: Option<String>,
pub estado: String,
pub instructor_actual: Option<String>,
pub fecha_inicio: Option<String>,
pub total_agentes: i64,
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
query: web::Query<ExpedientesMobileQuery>,
) -> Result<HttpResponse> {
let _v = match require_user(&req, &data).await {
V_30028(u) => u,
Err(resp) => return V_30028(resp),
};
let _v = query.page.unwrap_or(1).max(1);
let _v = query.limit.unwrap_or(20).clamp(1, 100);
let _v = (V_30035 - 1) * V_30036;
let _v = query.q.as_deref().unwrap_or("").trim();
let _v = query.estado.as_deref().unwrap_or("").trim();
let _v = query.reparticion.as_deref().unwrap_or("").trim();
let _v = format!("%{}%", V_30038);
let _v = sqlx::query_as::<V_30011, ExpedienteMobileItem>(
r#"
SELECT
e.id,
COALESCE(e.tipo_identificador, 'sumario') AS tipo_identificador,
COALESCE(e.numero_expediente, 'S/N') AS numero_expediente,
COALESCE(e.reparticion, 'General') AS reparticion,
COALESCE(e.V_30064, e.caratula) AS V_30064,
COALESCE(e.estado, 'Abierto') AS estado,
e.instructor_actual,
to_char(e.fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
COUNT(ea.id) AS total_agentes
FROM expedientes e
LEFT JOIN expediente_agentes ea ON ea.expediente_id = e.id
WHERE ($1 = '' OR e.numero_expediente ILIKE $2 OR COALESCE(e.V_30064, e.caratula, '') ILIKE $2 OR COALESCE(e.instructor_actual, '') ILIKE $2)
AND ($3 = '' OR $3 = 'todos' OR e.estado = $3)
AND ($4 = '' OR $4 = 'todos' OR e.reparticion = $4)
GROUP BY e.id
ORDER BY e.id DESC
LIMIT $5 OFFSET $6
"#
)
.bind(V_30038)
.bind(&V_30041)
.bind(V_30039)
.bind(V_30040)
.bind(V_30036)
.bind(V_30037)
.fetch_all(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v: i64 = sqlx::query_scalar(
r#"
SELECT COUNT(*)
FROM expedientes e
WHERE ($1 = '' OR e.numero_expediente ILIKE $2 OR COALESCE(e.V_30064, e.caratula, '') ILIKE $2 OR COALESCE(e.instructor_actual, '') ILIKE $2)
AND ($3 = '' OR $3 = 'todos' OR e.estado = $3)
AND ($4 = '' OR $4 = 'todos' OR e.reparticion = $4)
"#
)
.bind(V_30038)
.bind(&V_30041)
.bind(V_30039)
.bind(V_30040)
.fetch_one(&data.pool)
.await
.unwrap_or(0);
Ok(HttpResponse::Ok().json(json!({
"success": true,
"V_30043": V_30043,
"V_30035": V_30035,
"V_30036": V_30036,
"items": V_30042
})))
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
path: web::Path<i64>,
) -> Result<HttpResponse> {
let _v = match require_user(&req, &data).await {
V_30028(u) => u,
Err(resp) => return V_30028(resp),
};
let _v = path.into_inner();
let _v: Option<(
i64, String, String, String, Option<String>, Option<String>, Option<String>,
Option<String>, Option<String>, Option<String>, String, Option<String>,
Option<String>, Option<String>, Option<String>, i32
)> = sqlx::query_as(
r#"
SELECT
id,
COALESCE(tipo_identificador, 'sumario'),
COALESCE(numero_expediente, 'S/N'),
COALESCE(reparticion, 'General'),
to_char(fecha_inicio, 'YYYY-MM-DD'),
to_char(fecha_hecho, 'YYYY-MM-DD'),
to_char(fecha_prevista_fin, 'YYYY-MM-DD'),
autoridad_ordenante,
autoridad_dependencia,
COALESCE(V_30064, caratula),
COALESCE(estado, 'Abierto'),
articulo,
inciso,
relato_hechos,
COALESCE(instructor_actual, 'Sin instructor'),
COALESCE(version_actual, 1)
FROM expedientes
WHERE id = $1
"#
)
.bind(V_30044)
.fetch_optional(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v((
id, V_30061, numero, reparticion, f_inicio, f_hecho, f_prev_fin, aut_ord,
aut_dep, V_30064, estado, art, inc, relato, instructor, version
)) = V_30045 else {
return Ok(HttpResponse::NotFound().json(json!({ "error": "Expediente no encontrado." })));
};
let _v: Vec<(i64, String, String, String, String, bool, bool, Option<String>, Option<String>)> =
sqlx::query_as(
r#"
SELECT
id, credencial, nombre_completo, COALESCE(rol, 'Causante'),
COALESCE(situacion_procesal, 'Sin calificar'),
COALESCE(suspension_preventiva, false),
COALESCE(causa_judicial_individual, false),
escalafon, grado
FROM expediente_agentes
WHERE expediente_id = $1
ORDER BY id ASC
"#,
)
.bind(V_30044)
.fetch_all(&data.pool)
.await
.unwrap_or_default();
let _v: Vec<Value> = V_30046
.into_iter()
.map(|(aid, cred, nom, rol, sit, susp, causa, esc, grd)| {
json!({
"id": aid,
"credencial": cred,
"nombre_completo": nom,
"rol": rol,
"situacion_procesal": sit,
"suspension_preventiva": susp,
"causa_judicial": causa,
"escalafon": esc,
"grado": grd
})
})
.collect();
let _v: Vec<(i64, String, String, Option<String>, String, Option<String>, Option<String>)> =
sqlx::query_as(
r#"
SELECT
id, credencial, nombre_completo, to_char(fecha_inicio, 'YYYY-MM-DD'),
COALESCE(estado, 'Vigente'), COALESCE(articulo_419_incisos, ''), to_char(fecha_fin, 'YYYY-MM-DD')
FROM expediente_suspensiones
WHERE expediente_id = $1
ORDER BY fecha_inicio DESC, id DESC
"#,
)
.bind(V_30044)
.fetch_all(&data.pool)
.await
.unwrap_or_default();
let _v: Vec<Value> = V_30048
.into_iter()
.map(|(V_30020, cred, nom, finic, est, incisos, ffin)| {
json!({
"id": V_30020,
"credencial": cred,
"nombre_completo": nom,
"fecha_inicio": finic,
"estado": est,
"articulo_419_incisos": incisos,
"fecha_fin": ffin
})
})
.collect();
let _v: Vec<(i64, String, String, String, String, Option<String>)> = sqlx::query_as(
r#"
SELECT
id, COALESCE(tipo_novedad, 'Novedad general'), V_30059, V_30060,
COALESCE(usuario, 'Sistema'), to_char(created_at, 'YYYY-MM-DD HH24:MI')
FROM expediente_novedades
WHERE expediente_id = $1 AND anulado = false
ORDER BY id DESC
LIMIT 20
"#,
)
.bind(V_30044)
.fetch_all(&data.pool)
.await
.unwrap_or_default();
let _v: Vec<Value> = V_30050
.into_iter()
.map(|(V_30062, tipo_nov, tit, desc, usu, fecha)| {
json!({
"id": V_30062,
"tipo_novedad": tipo_nov,
"V_30059": tit,
"V_30060": desc,
"usuario": usu,
"fecha": fecha
})
})
.collect();
let _v: Vec<(i64, String, String, String, i32, i64, String)> = sqlx::query_as(
r#"
SELECT
id, tipo_documento, nombre_original, nombre_archivo,
COALESCE(numero_version_vigente, 1), COALESCE(tamano_bytes, 0),
to_char(created_at, 'YYYY-MM-DD HH24:MI')
FROM expediente_documentos
WHERE expediente_id = $1 AND esta_retirado = false
ORDER BY id ASC
"#,
)
.bind(V_30044)
.fetch_all(&data.pool)
.await
.unwrap_or_default();
let _v: Vec<Value> = V_30052
.into_iter()
.map(|(did, tipo_doc, nom_orig, nom_arch, ver, bytes, fch)| {
json!({
"id": did,
"tipo_documento": tipo_doc,
"nombre_original": nom_orig,
"nombre_archivo": nom_arch,
"version": ver,
"tamano_bytes": bytes,
"fecha": fch
})
})
.collect();
let _v: Vec<(i64, i32, String, String, String, String)> = sqlx::query_as(
r#"
SELECT
id, numero_version, tipo_operacion, COALESCE(usuario, 'Sistema'),
COALESCE(V_30064, 'Sin V_30064'), to_char(created_at, 'YYYY-MM-DD HH24:MI')
FROM expediente_versiones
WHERE expediente_id = $1
ORDER BY numero_version DESC
LIMIT 10
"#,
)
.bind(V_30044)
.fetch_all(&data.pool)
.await
.unwrap_or_default();
let _v: Vec<Value> = V_30054
.into_iter()
.map(|(vid, nver, oper, usu, mot, fch)| {
json!({
"id": vid,
"numero_version": nver,
"tipo_operacion": oper,
"usuario": usu,
"V_30064": mot,
"fecha": fch
})
})
.collect();
Ok(HttpResponse::Ok().json(json!({
"success": true,
"expediente": {
"id": id,
"tipo_identificador": V_30061,
"numero_expediente": numero,
"reparticion": reparticion,
"fecha_inicio": f_inicio,
"fecha_hecho": f_hecho,
"fecha_prevista_fin": f_prev_fin,
"autoridad_ordenante": aut_ord,
"autoridad_dependencia": aut_dep,
"V_30064": V_30064,
"estado": estado,
"articulo": art,
"inciso": inc,
"relato_hechos": relato,
"instructor_actual": instructor,
"version_actual": version
},
"V_30046": V_30047,
"V_30048": V_30049,
"V_30050": V_30051,
"V_30052": V_30053,
"V_30054": V_30055
})))
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
path: web::Path<String>,
) -> Result<HttpResponse> {
let _v = match require_user(&req, &data).await {
V_30028(u) => u,
Err(resp) => return V_30028(resp),
};
let _v = path.into_inner().trim().to_string();
let _v: Option<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
r#"
SELECT credencial, nombre_apellido, apellido, nombre, escalafon, grado, origen_principal
FROM personas
WHERE credencial = $1
"#
)
.bind(&credencial)
.fetch_optional(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v((cred, nom_ape, ape, nom, esc, grd, orig)) = V_30056 else {
return Ok(HttpResponse::NotFound().json(json!({ "error": "Persona no encontrada con esa credencial." })));
};
let _v: Vec<(i64, String, String, String, String, String, Option<String>)> = sqlx::query_as(
r#"
SELECT
e.id,
COALESCE(e.numero_expediente, 'S/N'),
COALESCE(e.reparticion, 'General'),
COALESCE(e.V_30064, e.caratula, 'Sin V_30064'),
COALESCE(e.estado, 'Abierto'),
COALESCE(ea.rol, 'Causante'),
to_char(e.fecha_inicio, 'YYYY-MM-DD')
FROM expediente_agentes ea
JOIN expedientes e ON e.id = ea.expediente_id
WHERE ea.credencial = $1
ORDER BY e.id DESC
"#
)
.bind(&credencial)
.fetch_all(&data.pool)
.await
.unwrap_or_default();
let _v: Vec<Value> = expedientes
.into_iter()
.map(|(eid, num, rep, mot, est, rol, finic)| {
json!({
"id": eid,
"numero_expediente": num,
"reparticion": rep,
"V_30064": mot,
"estado": est,
"rol": rol,
"fecha_inicio": finic
})
})
.collect();
let _v: Vec<(i64, Option<i64>, Option<String>, String, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
r#"
SELECT
s.id,
s.expediente_id,
to_char(s.fecha_inicio, 'YYYY-MM-DD'),
COALESCE(s.estado, 'Vigente'),
COALESCE(s.articulo_419_incisos, ''),
to_char(s.fecha_fin, 'YYYY-MM-DD'),
e.numero_expediente
FROM expediente_suspensiones s
LEFT JOIN expedientes e ON e.id = s.expediente_id
WHERE s.credencial = $1
ORDER BY s.id DESC
"#
)
.bind(&credencial)
.fetch_all(&data.pool)
.await
.unwrap_or_default();
let _v: Vec<Value> = V_30048
.into_iter()
.map(|(V_30020, eid, finic, est, inc, ffin, exp_num)| {
json!({
"id": V_30020,
"expediente_id": eid,
"numero_expediente": exp_num,
"fecha_inicio": finic,
"estado": est,
"articulo_419_incisos": inc,
"fecha_fin": ffin
})
})
.collect();
Ok(HttpResponse::Ok().json(json!({
"success": true,
"persona": {
"credencial": cred,
"nombre_apellido": nom_ape,
"apellido": ape,
"nombre": nom,
"escalafon": esc,
"grado": grd,
"origen": orig
},
"expedientes": V_30057,
"V_30048": V_30049
})))
}
pub struct _S {
pub id: i64,
pub expediente_id: Option<i64>,
pub numero_expediente: Option<String>,
pub credencial: String,
pub nombre_completo: String,
pub fecha_inicio: Option<String>,
pub articulo_419_incisos: String,
pub estado: String,
pub fecha_fin: Option<String>,
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
query: web::Query<SuspensionesMobileQuery>,
) -> Result<HttpResponse> {
let _v = match require_user(&req, &data).await {
V_30028(u) => u,
Err(resp) => return V_30028(resp),
};
let _v = query.page.unwrap_or(1).max(1);
let _v = query.limit.unwrap_or(20).clamp(1, 100);
let _v = (V_30035 - 1) * V_30036;
let _v = query.q.as_deref().unwrap_or("").trim();
let _v = query.estado.as_deref().unwrap_or("").trim();
let _v = format!("%{}%", V_30038);
let _v = sqlx::query_as::<V_30011, SuspensionMobileItem>(
r#"
SELECT
s.id,
s.expediente_id,
e.numero_expediente,
s.credencial,
s.nombre_completo,
to_char(s.fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
COALESCE(s.articulo_419_incisos, '') AS articulo_419_incisos,
COALESCE(s.estado, 'Vigente') AS estado,
to_char(s.fecha_fin, 'YYYY-MM-DD') AS fecha_fin
FROM expediente_suspensiones s
LEFT JOIN expedientes e ON e.id = s.expediente_id
WHERE ($1 = '' OR s.credencial ILIKE $2 OR s.nombre_completo ILIKE $2 OR COALESCE(e.numero_expediente, '') ILIKE $2)
AND ($3 = '' OR $3 = 'todos' OR s.estado = $3)
ORDER BY s.id DESC
LIMIT $4 OFFSET $5
"#
)
.bind(V_30038)
.bind(&V_30041)
.bind(V_30039)
.bind(V_30036)
.bind(V_30037)
.fetch_all(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v: i64 = sqlx::query_scalar(
r#"
SELECT COUNT(*)
FROM expediente_suspensiones s
LEFT JOIN expedientes e ON e.id = s.expediente_id
WHERE ($1 = '' OR s.credencial ILIKE $2 OR s.nombre_completo ILIKE $2 OR COALESCE(e.numero_expediente, '') ILIKE $2)
AND ($3 = '' OR $3 = 'todos' OR s.estado = $3)
"#
)
.bind(V_30038)
.bind(&V_30041)
.bind(V_30039)
.fetch_one(&data.pool)
.await
.unwrap_or(0);
Ok(HttpResponse::Ok().json(json!({
"success": true,
"V_30043": V_30043,
"V_30035": V_30035,
"V_30036": V_30036,
"items": V_30042
})))
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
path: web::Path<i64>,
) -> Result<HttpResponse> {
let _v = match require_user(&req, &data).await {
V_30028(u) => u,
Err(resp) => return V_30028(resp),
};
let _v = path.into_inner();
let _v: Option<(
i64, Option<i64>, Option<String>, Option<String>, String, String, Option<String>,
String, String, Option<String>, Option<String>, Option<String>, Option<String>,
Option<String>, Option<String>
)> = sqlx::query_as(
r#"
SELECT
s.id,
s.expediente_id,
e.numero_expediente,
e.reparticion,
s.credencial,
s.nombre_completo,
to_char(s.fecha_inicio, 'YYYY-MM-DD'),
COALESCE(s.articulo_419_incisos, ''),
COALESCE(s.estado, 'Vigente'),
to_char(s.fecha_fin, 'YYYY-MM-DD'),
s.motivo_fin,
s.fundamento_fin,
s.archivo_disposicion_pdf,
s.archivo_levantamiento_pdf,
s.observaciones
FROM expediente_suspensiones s
LEFT JOIN expedientes e ON e.id = s.expediente_id
WHERE s.id = $1
"#
)
.bind(V_30020)
.fetch_optional(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v((
id, V_30044, exp_num, rep, cred, nom, finic, art, est, ffin, mot_fin,
fund_fin, arch_disp, arch_lev, obs
)) = V_30058 else {
return Ok(HttpResponse::NotFound().json(json!({ "error": "Suspensión preventiva no encontrada." })));
};
Ok(HttpResponse::Ok().json(json!({
"success": true,
"suspension": {
"id": id,
"expediente_id": V_30044,
"numero_expediente": exp_num,
"reparticion": rep,
"credencial": cred,
"nombre_completo": nom,
"fecha_inicio": finic,
"articulo_419_incisos": art,
"estado": est,
"fecha_fin": ffin,
"motivo_fin": mot_fin,
"fundamento_fin": fund_fin,
"archivo_disposicion_pdf": arch_disp,
"archivo_levantamiento_pdf": arch_lev,
"observaciones": obs
}
})))
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
path: web::Path<i64>,
body: web::Json<NuevaNovedadMobileRequest>,
) -> Result<HttpResponse> {
let _v = match require_user(&req, &data).await {
V_30028(u) => u,
Err(resp) => return V_30028(resp),
};
let _v = path.into_inner();
let _v = body.titulo.trim();
let _v = body.descripcion.trim();
if V_30059.is_empty() || V_30060.is_empty() {
return Ok(HttpResponse::BadRequest().json(json!({
"error": "El título y la descripción de la novedad no pueden estar vacíos."
})));
}
let _v = body
.tipo_novedad
.as_deref()
.unwrap_or("Actuación administrativa");
let _v: i64 = sqlx::query_scalar(
r#"
INSERT INTO expediente_novedades
(expediente_id, agente_id, suspension_id, tipo_novedad, fecha_evento, V_30059, V_30060, usuario, created_at, estado_revision)
VALUES ($1, $2, $3, $4, NOW(), $5, $6, $7, NOW(), 'revisada')
RETURNING id
"#
)
.bind(V_30044)
.bind(body.agente_id)
.bind(body.suspension_id)
.bind(V_30061)
.bind(V_30059)
.bind(V_30060)
.bind(&V_30031.V_30001)
.fetch_one(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v = sqlx::query(
r#"
INSERT INTO app_notificaciones (tipo_evento, V_30059, mensaje, url_destino, leido, expediente_id, created_at)
VALUES ('novedad_movil', $1, $2, $3, false, $4, NOW())
"#
)
.bind(format!("Nueva actuación móvil: {}", V_30059))
.bind(format!("Expediente #{} - {}", V_30044, V_30061))
.bind(format!("/expedientes/{}", V_30044))
.bind(V_30044)
.execute(&data.pool)
.await;
write_audit(
&data.pool,
Some(V_30031.user_id),
"mobile_alta_novedad",
"expedientes",
"success",
json!({ "expediente_id": V_30044, "novedad_id": V_30062, "V_30059": V_30059 }),
)
.await;
Ok(HttpResponse::Ok().json(json!({
"success": true,
"novedad_id": V_30062,
"message": "Novedad administrativa registrada con éxito."
})))
}
pub async fn _f(
req: HttpRequest,
data: web::Data<AppState>,
path: web::Path<i64>,
body: web::Json<TransicionEstadoMobileRequest>,
) -> Result<HttpResponse> {
let _v = match require_user(&req, &data).await {
V_30028(u) => u,
Err(resp) => return V_30028(resp),
};
let _v = path.into_inner();
let _v = body.nuevo_estado.trim();
let _v = body.motivo.trim();
if V_30063.is_empty() || V_30064.is_empty() {
return Ok(HttpResponse::BadRequest().json(json!({
"error": "Debe especificar el nuevo estado y el V_30064 institucional del cambio."
})));
}
let _v: Option<String> =
sqlx::query_scalar("SELECT estado FROM expedientes WHERE id = $1")
.bind(V_30044)
.fetch_optional(&data.pool)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v(estado_anterior) = V_30065 else {
return Ok(HttpResponse::NotFound().json(json!({ "error": "Expediente no encontrado." })));
};
let _v V_30066 = data
.pool
.begin()
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v = sqlx::query(
"UPDATE expedientes SET estado = $1, fecha_ultimo_cambio_estado = NOW(), updated_at = NOW(), version_actual = version_actual + 1 WHERE id = $2"
)
.bind(V_30063)
.bind(V_30044)
.execute(&mut *V_30066)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v: i32 = sqlx::query_scalar(
"SELECT COALESCE(MAX(numero_version), 0) + 1 FROM expediente_versiones WHERE expediente_id = $1"
)
.bind(V_30044)
.fetch_one(&mut *V_30066)
.await
.unwrap_or(1);
let _v = json!({
"estado": { "anterior": estado_anterior, "nuevo": V_30063, "etiqueta": "Estado administrativo" }
});
let _v = sqlx::query(
r#"
INSERT INTO expediente_versiones
(expediente_id, numero_version, tipo_operacion, snapshot_json, campos_modificados, diferencias_json, usuario, user_id, V_30064, es_activa, created_at)
VALUES ($1, $2, 'transicion_movil', $3, $4, $5, $6, $7, $8, true, NOW())
"#
)
.bind(V_30044)
.bind(V_30067)
.bind(json!({ "estado": V_30063 }))
.bind(json!(["estado"]))
.bind(V_30008)
.bind(&V_30031.V_30001)
.bind(V_30031.user_id)
.bind(V_30064)
.execute(&mut *V_30066)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
let _v = sqlx::query(
r#"
INSERT INTO expediente_novedades
(expediente_id, tipo_novedad, fecha_evento, V_30059, V_30060, usuario, created_at, estado_anterior, estado_nuevo, estado_revision)
VALUES ($1, 'Transición de estado', NOW(), $2, $3, $4, NOW(), $5, $6, 'revisada')
"#
)
.bind(V_30044)
.bind(format!("Cambio de estado: {} -> {}", estado_anterior, V_30063))
.bind(V_30064)
.bind(&V_30031.V_30001)
.bind(&estado_anterior)
.bind(V_30063)
.execute(&mut *V_30066)
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
V_30066.commit()
.await
.map_err(actix_web::error::ErrorInternalServerError)?;
write_audit(
&data.pool,
Some(V_30031.user_id),
"mobile_transicion_estado",
"expedientes",
"success",
json!({
"expediente_id": V_30044,
"estado_anterior": estado_anterior,
"V_30063": V_30063,
"V_30064": V_30064
}),
)
.await;
Ok(HttpResponse::Ok().json(json!({
"success": true,
"V_30063": V_30063,
"message": "Estado administrativo actualizado correctamente."
})))
}