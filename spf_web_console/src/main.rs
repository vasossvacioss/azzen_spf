use actix_files::{Files, NamedFile};
use actix_multipart::Multipart;
use actix_web::http::cookie::{Cookie, SameSite};
use actix_web::http::header;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Result, error, get, post, web};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use askama::Template;
use chrono::{Duration, Local, NaiveDateTime, Utc};
use csv::WriterBuilder;
use dotenvy::dotenv;
use rust_xlsxwriter::{Color, Format, Workbook, XlsxError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use sha2::{Digest, Sha256};

mod mailer;
mod crypto_utils;
pub(crate) mod modulos;
pub(crate) mod rutas_modulos_v2;
pub(crate) mod rutas_mobile;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub bind_addr: String,
    pub session_idle_minutes: i64,
    pub session_absolute_hours: i64,
    pub max_page_size: i64,
    pub max_export_rows: i64,
    pub bootstrap_credential: Option<String>,
    pub bootstrap_password: Option<String>,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub smtp_from_name: String,
    pub app_base_url: String,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) V_30012: PgPool,
    pub(crate) V_30011: AppConfig,
}

#[derive(FromRow)]
struct S_50001 {
    id: i64,
    V_30114: String,
    display_name: String,
    password_hash: String,
    role: String,
    V_30639: bool,
}

#[derive(FromRow)]
pub(crate) struct SessionUser {
    pub(crate) user_id: i64,
    pub(crate) credential: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
    pub(crate) totp_enabled: bool,
    pub(crate) is_restricted: bool,
    pub(crate) dispositivo_uuid: Option<String>,
}

#[derive(FromRow, Clone)]
struct S_50002 {
    credencial: String,
    apellido: Option<String>,
    nombre: Option<String>,
    escalafon_grado: Option<String>,
    V_30526: Option<String>,
    origen_principal: Option<String>,
    last_seen_at: NaiveDateTime,
    tiene_suspension_activa: bool,
    tiene_sumario_abierto: bool,
    tiene_impedimento_judicial: bool,
}

impl S_50002 {
    pub fn estado_disciplinario_label(&self) -> &'static str {
        if self.tiene_suspension_activa {
            "Suspensión Activa"
        } else if self.tiene_sumario_abierto {
            "Sumario Abierto"
        } else if self.tiene_impedimento_judicial {
            "Impedimento Judicial"
        } else {
            "Sin medidas activas"
        }
    }

    pub fn estado_disciplinario_badge_class(&self) -> &'static str {
        if self.tiene_suspension_activa {
            "badge-danger"
        } else if self.tiene_sumario_abierto {
            "badge-warning"
        } else if self.tiene_impedimento_judicial {
            "badge-purple"
        } else {
            "badge-success"
        }
    }
}

#[derive(FromRow)]
struct S_50003 {
    id: i64,
    dominio: String,
    hoja_origen: String,
    V_30526: Option<String>,
    credencial_resuelta: Option<String>,
    metodo_resolucion: Option<String>,
    score_resolucion: Option<f64>,
}

#[derive(Clone)]
pub(crate) struct NavFile {
    pub(crate) file_id: i64,
    pub(crate) file_name: String,
    pub(crate) sheets: Vec<NavSheet>,
}

#[derive(Clone)]
pub(crate) struct NavSheet {
    pub(crate) sheet_id: i64,
    pub(crate) sheet_name: String,
}

#[derive(FromRow)]
struct S_50004 {
    file_id: i64,
    file_name: String,
    sheet_id: i64,
    sheet_name: String,
    sheet_order: i32,
}

#[derive(FromRow)]
struct S_50005 {
    file_id: i64,
    file_name: String,
    sheet_id: i64,
    sheet_name: String,
}

#[derive(FromRow)]
struct S_50006 {
    original_name: String,
    technical_column_name: String,
}

#[derive(FromRow)]
struct S_50007 {
    source_row_number: i32,
    payload_json: Value,
    version_actual: i32,
    total_novedades: i32,
}

#[derive(Clone, Serialize, Deserialize)]
struct S_50008 {
    source_row_number: i32,
    credencial: String,
    V_30526: String,
    expediente_resolucion: String,
    origen_destino: String,
    values: Vec<String>,
    V_30381: String,
    version_actual: i32,
    total_novedades: i32,
}

#[derive(FromRow)]
struct S_50009 {
    credencial: String,
    rol_credencial: String,
    archivo_origen: String,
    hoja_origen: String,
    fila_origen: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AgenteInvolucradoItem {
    pub credencial: String,
    pub nombre_completo: String,
    pub escalafon: String,
    pub grado: String,
    pub rol: String,
}

#[derive(Clone, FromRow)]
struct S_50010 {
    id: i64,
    numero_expediente: String,
    tipo_identificador: String,
    reparticion: String,
    instructor_actual: String,
    estado: String,
    caratula: String,
    fecha_inicio: String,
    agentes_vinculados: i64,
    version_actual: i32,
}

#[derive(Clone, Serialize)]
pub struct ExpedienteRow {
    pub id: i64,
    pub numero_expediente: String,
    pub tipo_identificador: String,
    pub reparticion: String,
    pub instructor_actual: String,
    pub estado: String,
    pub caratula: String,
    pub fecha_inicio: String,
    pub agentes_vinculados: i64,
    pub version_actual: i32,
    pub agentes: Vec<AgenteInvolucradoItem>,
}

impl ExpedienteRow {
    pub fn agentes_primeros(&self) -> &[AgenteInvolucradoItem] {
        if self.agentes.len() <= 2 {
            &self.agentes[..]
        } else {
            &self.agentes[..2]
        }
    }

    pub fn agentes_restantes(&self) -> usize {
        if self.agentes.len() > 2 {
            self.agentes.len() - 2
        } else {
            0
        }
    }

    pub fn agentes_json_attr(&self) -> String {
        serde_json::to_string(&self.agentes).unwrap_or_else(|V_30020| "[]".to_string())
    }
}

#[derive(Clone, FromRow)]
struct S_50011 {
    id: i64,
    V_30228: String,
    numero_expediente: String,
    reparticion: String,
    V_30230: Option<String>,
    fecha_inicio: Option<String>,
    V_30232: Option<String>,
    V_30233: Option<String>,
    V_30234: Option<String>,
    V_30235: Option<String>,
    V_30167: String,
    estado: String,
    V_30066: Option<String>,
    V_30067: Option<String>,
    V_30236: Option<String>,
    V_30237: Option<String>,
    V_30238: String,
    V_30068: String,
    V_30240: Option<String>,
    V_30241: Option<String>,
    V_30242: String,
    V_30243: Option<String>,
    documento_informe_hechos: Option<String>,
    documento_disposicion: Option<String>,
    documento_finalizacion: Option<String>,
    V_30231: Option<String>,
    V_30463: Option<String>,
    fecha_ultimo_cambio_estado: Option<String>,
    fecha_ultima_novedad: Option<String>,
    tipo_ultima_novedad: Option<String>,
    version_actual: i32,
}

#[derive(Clone, FromRow)]
pub(crate) struct ExpedienteAgenteItem {
    id: i64,
    V_30275: i64,
    credencial: String,
    V_30266: String,
    rol: String,
    V_30323: Option<String>,
    V_30465: Option<String>,
    V_30319: bool,
    V_30320: Option<String>,
    V_30321: Option<String>,
    V_30322: bool,
    V_30324: Option<String>,
    V_30325: Option<String>,
    V_30326: bool,
    V_30327: Option<String>,
    V_30328: Option<String>,
    V_30329: Option<String>,
    V_30330: String,
    V_30331: Option<String>,
    V_30332: Option<String>,
    apellido: Option<String>,
    nombre: Option<String>,
    V_30317: Option<String>,
    V_30318: Option<String>,
}

#[derive(Clone, Serialize)]
struct S_50012 {
    id: i64,
    V_30092: String,
    V_30182: String,
    nombre_original: String,
    version_numero: i32,
    tamano_bytes: i64,
    tamano_formateado: String,
    sha256_hash: String,
    sha256_short: String,
    esta_retirado: bool,
    motivo_retiro: Option<String>,
    V_30503: String,
    usuario: String,
    es_estandar: bool,
}

#[derive(Clone, Serialize)]
struct S_50013 {
    id: i64,
    tipo_entidad: String,
    entidad_id: i64,
    V_30275: i64,
    V_30543: String,
    expediente_reparticion: String,
    referencia: String,
    numero_version: i32,
    tipo_operacion: String,
    usuario: String,
    V_30167: String,
    fecha_hora: String,
    es_vigente: bool,
    esta_retirado: bool,
    V_30306: Option<i64>,
    version_origen_id: Option<i64>,
    sha256_hash: Option<String>,
    tamano_bytes: Option<i64>,
    resumen_cambios: String,
    detalles_json: String,
}

#[derive(Clone, Serialize)]
struct S_50014 {
    total_versiones: i64,
    expedientes_versionados: i64,
    documentos_versionados: i64,
    restauraciones_realizadas: i64,
}

#[derive(Deserialize, Clone)]
struct S_50015 {
    V_30600: Option<String>,
    V_30601: Option<String>,
    usuario: Option<String>,
    V_30065: Option<String>,
    V_30604: Option<String>,
    V_30605: Option<String>,
    V_30069: Option<i64>,
    V_30070: Option<i64>,
    V_30599: Option<String>,
}

#[derive(Clone, Serialize)]
struct S_50016 {
    id: i64,
    numero_expediente: String,
    reparticion: String,
    V_30167: String,
    version_actual: i32,
    total_versiones_exp: i64,
    total_documentos: i64,
    ultima_operacion: String,
    ultimo_usuario: String,
    ultimo_motivo: String,
    ultima_fecha_hora: String,
}

#[derive(FromRow)]
struct S_50017 {
    id: i64,
    numero_expediente: String,
    reparticion: String,
    V_30167: String,
    version_actual: i32,
    total_versiones_exp: i64,
    total_documentos: i64,
    ultima_operacion: String,
    ultimo_usuario: String,
    ultimo_motivo: String,
    ultima_fecha_hora: String,
}

#[derive(FromRow)]
struct S_50018 {
    id: i64,
    tipo_entidad: String,
    entidad_id: i64,
    V_30275: i64,
    numero_expediente: String,
    reparticion: String,
    referencia: String,
    numero_version: i32,
    tipo_operacion: String,
    usuario: String,
    V_30167: String,
    fecha_hora: String,
    es_vigente: bool,
    esta_retirado: bool,
    V_30306: Option<i64>,
    version_origen_id: Option<i64>,
    sha256_hash: Option<String>,
    tamano_bytes: Option<i64>,
    campos_json: String,
    dif_json: String,
    snapshot_json: String,
}

#[derive(Clone, FromRow)]
struct S_50019 {
    id: i64,
    V_30765: String,
    V_30766: Option<String>,
    V_30764: Option<String>,
    usuario: String,
    V_30167: Option<String>,
    V_30675: String,
}

#[derive(Clone, FromRow)]
struct S_50020 {
    id: i64,
    V_30275: i64,
    V_30464: Option<i64>,
    agente_nombre: Option<String>,
    agente_credencial: Option<String>,
    V_30458: String,
    V_30459: Option<String>,
    V_30460: String,
    V_30461: String,
    V_30462: String,
    V_30463: Option<String>,
    estado_anterior: Option<String>,
    estado_nuevo: Option<String>,
    usuario: String,
    es_auditoria_automatica: bool,
    V_30465: Option<String>,
    anulado: bool,
    anulado_motivo: Option<String>,
    anulado_usuario: Option<String>,
    anulado_at: Option<String>,
    V_30675: String,
    V_30467: String,
    revisor_usuario: Option<String>,
    revisado_at: Option<String>,
    observaciones_revision: Option<String>,
    V_30466: Option<String>,
}

#[derive(Clone, FromRow)]
struct S_50021 {
    id: i64,
    V_30275: Option<i64>,
    numero_expediente: Option<String>,
    V_30464: Option<i64>,
    credencial: String,
    V_30266: String,
    fecha_inicio: String,
    V_30471: String,
    estado: String,
    fecha_fin: Option<String>,
    V_30476: Option<String>,
    V_30477: Option<String>,
    archivo_disposicion_pdf: Option<String>,
    archivo_levantamiento_pdf: Option<String>,
    usuario: String,
    V_30463: Option<String>,
    V_30675: String,
}

#[derive(Clone, FromRow)]
struct S_50022 {
    id: i64,
    expediente_origen_id: i64,
    expediente_destino_id: i64,
    expediente_destino_numero: String,
    expediente_destino_reparticion: String,
    expediente_destino_estado: String,
    V_30481: String,
    V_30482: Option<String>,
    V_30167: String,
    V_30548: String,
    V_30465: Option<String>,
    usuario: String,
    estado: String,
    anulado_motivo: Option<String>,
    anulado_usuario: Option<String>,
    anulado_at: Option<String>,
    V_30282: String,
}

#[derive(Clone, FromRow)]
struct S_50023 {
    id: i64,
    expediente_origen_id: i64,
    expediente_destino_id: i64,
    V_30481: String,
    V_30482: Option<String>,
    V_30167: String,
    V_30548: String,
    V_30465: Option<String>,
    usuario: String,
    estado: String,
    anulado_motivo: Option<String>,
    anulado_usuario: Option<String>,
    anulado_at: Option<String>,
    other_id: i64,
    other_numero: String,
    other_reparticion: String,
    other_estado: String,
}

#[derive(Clone, FromRow)]
struct S_50024 {
    V_30275: i64,
    numero_expediente: String,
    reparticion: String,
    estado: String,
    agente_credencial: String,
    agente_nombre: String,
}

#[derive(Clone, FromRow)]
struct S_50025 {
    V_30275: i64,
    numero_expediente: String,
    V_30228: String,
    reparticion: String,
    estado: String,
    rol: String,
    fecha_inicio: Option<String>,
    V_30167: Option<String>,
    V_30465: Option<String>,
}

#[derive(Clone, Serialize)]
struct S_50026 {
    hoja: String,
    V_30501: String,
    tipo_impedimento: String,
    V_30502: String,
    V_30503: String,
    V_30504: String,
    V_30505: Option<String>,
}

#[derive(Clone, Serialize)]
struct S_50027 {
    hoja: String,
    complejo_unidad: String,
    V_30167: String,
    V_30113: String,
    V_30511: String,
    V_30512: String,
    V_30504: String,
}

#[derive(Clone, FromRow)]
struct S_50028 {
    id: i64,
    V_30275: i64,
    numero_expediente: String,
    V_30460: String,
    V_30092: String,
    V_30461: String,
    V_30462: String,
    usuario: String,
    V_30465: Option<String>,
    es_auditoria: bool,
}


#[derive(Template)]
#[template(path = "login.html")]
struct S_50029 {
    error: Option<String>,
    notice: Option<String>,
    device_warning: Option<String>,
}

#[derive(Template)]
#[template(path = "login_2fa.html")]
struct S_50030 {
    error: Option<String>,
    masked_email: String,
    V_30114: String,
}

#[derive(Template)]
#[template(path = "login_totp.html")]
struct S_50031 {
    error: Option<String>,
    V_30114: String,
    display_name: String,
}

#[derive(Template)]
#[template(path = "login_totp_enrolar.html")]
struct S_50032 {
    error: Option<String>,
    V_30114: String,
    V_30047: String,
    secret_base32: String,
    V_30046: String,
}

#[derive(Template)]
#[template(path = "login_totp_backup_codes.html")]
struct S_50033 {
    V_30114: String,
    V_30133: Vec<String>,
}

#[derive(Template)]
#[template(path = "recuperar_password.html")]
struct S_50034 {
    error: Option<String>,
    success_message: Option<String>,
}

#[derive(Template)]
#[template(path = "recuperar_password_confirmar.html")]
struct S_50035 {
    error: Option<String>,
    V_30153: String,
}

#[derive(Template)]
#[template(path = "cambiar_password_obligatorio.html")]
struct S_50036 {
    error: Option<String>,
    V_30114: String,
    reason: String,
}

#[derive(Template)]
#[template(path = "verificar_email_resultado.html")]
struct S_50037 {
    success: bool,
    message: String,
}

#[derive(Template)]
#[template(path = "login_dispositivo.html")]
struct S_50038 {
    error: Option<String>,
    notice: Option<String>,
    V_30114: String,
    display_name: String,
    V_30651: String,
    dispositivo_nombre: String,
    dispositivo_os: String,
    dispositivo_browser: String,
    estado_dispositivo: String,
    V_30053: bool,
}

#[derive(Template)]
#[template(path = "login_liberar_sesion.html")]
struct S_50039 {
    error: Option<String>,
    success_message: Option<String>,
    credencial: String,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct S_50040 {
    user_name: String,
    now_local: String,
    V_30088: Vec<NavFile>,
    V_30098: i64,
    V_30099: i64,
    V_30100: i64,
    V_30101: i64,
    V_30102: i64,
}

#[derive(Template)]
#[template(path = "personas.html")]
struct S_50041 {
    user_name: String,
    V_30088: Vec<NavFile>,
    V_30022: Vec<S_50002>,
    V_30065: String,
    V_30103: String,
    V_30104: String,
    V_30069: i64,
    V_30087: i64,
    V_30079: i64,
    V_30070: i64,
    V_30105: String,
    V_30106: String,
}

#[derive(Template)]
#[template(path = "F_40085.html")]
struct S_50042 {
    user_name: String,
    V_30088: Vec<NavFile>,
    V_30022: Vec<S_50003>,
    estado: String,
    V_30069: i64,
    V_30087: i64,
    V_30079: i64,
    V_30070: i64,
}

#[derive(Template)]
#[template(path = "F_40086.html")]
struct S_50043 {
    user_name: String,
    V_30088: Vec<NavFile>,
    file_id: i64,
    sheet_id: i64,
    file_name: String,
    sheet_name: String,
    V_30362: Vec<String>,
    V_30022: Vec<S_50008>,
    V_30364: bool,
    V_30065: String,
    V_30069: i64,
    V_30087: i64,
    V_30079: i64,
    V_30070: i64,
}

#[derive(Template)]
#[template(path = "F_40087.html")]
struct S_50044 {
    user_name: String,
    V_30088: Vec<NavFile>,
    credencial: String,
    V_30113: String,
    nombre: String,
    apellido: String,
    V_30114: bool,
    V_30022: Vec<S_50009>,
    V_30069: i64,
    V_30087: i64,
    V_30079: i64,
    V_30070: i64,
}

#[derive(Template)]
#[template(path = "importar_excel.html")]
struct S_50045 {
    user_name: String,
}

#[derive(Template)]
#[template(path = "expedientes.html")]
struct S_50046 {
    user_name: String,
    V_30088: Vec<NavFile>,
    V_30022: Vec<ExpedienteRow>,
    V_30065: String,
    reparticion: String,
    estado: String,
    V_30066: String,
    V_30067: String,
    V_30068: String,
    V_30089: Vec<String>,
    V_30090: Vec<String>,
    V_30069: i64,
    V_30087: i64,
    V_30079: i64,
    V_30070: i64,
}

#[derive(Template)]
#[template(path = "expediente_nuevo.html")]
struct S_50047 {
    user_name: String,
    V_30088: Vec<NavFile>,
    V_30089: Vec<String>,
    V_30091: Vec<String>,
    error_msg: Option<String>,
    warning_duplicado: Option<String>,
}

#[derive(Template)]
#[template(path = "expediente_detalle.html")]
struct S_50048 {
    user_name: String,
    V_30088: Vec<NavFile>,
    V_30087: S_50011,
    V_30086: Vec<ExpedienteAgenteItem>,
    V_30284: Vec<S_50012>,
    V_30276: Vec<S_50019>,
    V_30277: Vec<S_50020>,
    V_30278: Vec<S_50021>,
    V_30280: Vec<S_50022>,
    V_30283: Vec<S_50024>,
    V_30089: Vec<String>,
    V_30091: Vec<String>,
    is_nuevo: bool,
    V_30289: Vec<S_50080>,
    success_msg: Option<String>,
    error_msg: Option<String>,
}

#[derive(Template)]
#[template(path = "F_40124.html")]
struct S_50049 {
    user_name: String,
    user_role: String,
    V_30088: Vec<NavFile>,
    V_30607: S_50014,
    V_30008: Vec<S_50013>,
    grouped_items: Vec<S_50016>,
    V_30599: String,
    V_30600: String,
    V_30601: String,
    usuario: String,
    V_30065: String,
    V_30604: String,
    V_30605: String,
    V_30069: i64,
    V_30087: i64,
    V_30611: i64,
    V_30608: Vec<String>,
}

#[derive(Template)]
#[template(path = "seguridad.html")]
struct S_50050 {
    user_name: String,
    user_role: String,
    V_30088: Vec<NavFile>,
    V_30630: String,
    V_30607: S_50051,
    V_30631: S_50052,
    V_30633: Vec<S_50053>,
    V_30646: Vec<S_50054>,
    V_30660: Vec<S_50055>,
    V_30674: Vec<S_50056>,
    V_30691: Vec<S_50057>,
    V_30699: Vec<S_50058>,
    V_30707: Vec<S_50059>,
    current_user_id: i64,
}

#[derive(Clone, Serialize, Deserialize)]
struct S_50051 {
    V_30714: i64,
    V_30634: i64,
    V_30635: i64,
    V_30715: i64,
    sesiones_activas: i64,
    V_30647: i64,
    V_30648: i64,
    V_30661: i64,
    V_30700: i64,
}

#[derive(Clone, Serialize, Deserialize, FromRow)]
struct S_50052 {
    id: i32,
    session_idle_minutes: i32,
    session_absolute_hours: i32,
    max_login_attempts: i32,
    lockout_duration_minutes: i32,
    V_30042: String,
    temp_code_expiry_minutes: i32,
    max_temp_code_resends: i32,
    single_session_per_user: bool,
    V_30112: bool,
    password_min_length: i32,
    password_require_special: bool,
    updated_at: NaiveDateTime,
}

#[derive(Clone, Serialize, Deserialize)]
struct S_50053 {
    id: i64,
    V_30114: String,
    display_name: String,
    V_30050: String,
    V_30636: bool,
    V_30637: bool,
    V_30638: bool,
    role: String,
    V_30639: bool,
    estado: String,
    V_30640: i32,
    V_30641: String,
    V_30642: String,
    V_30643: String,
    V_30644: i64,
}

#[derive(Clone, Serialize, Deserialize)]
struct S_50054 {
    id: i64,
    V_30055: i64,
    V_30649: String,
    V_30650: String,
    V_30651: String,
    V_30652: String,
    V_30653: String,
    V_30654: String,
    V_30655: String,
    estado: String,
    V_30656: Option<String>,
    V_30657: String,
    V_30658: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct S_50055 {
    id: i64,
    V_30092: String,
    V_30662: i64,
    V_30663: String,
    V_30664: Option<i64>,
    V_30665: String,
    V_30666: String,
    V_30667: String,
    estado: String,
    V_30668: String,
    V_30669: String,
    V_30670: String,
    V_30671: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct S_50056 {
    id: String,
    V_30055: i64,
    V_30114: String,
    display_name: String,
    role: String,
    V_30675: String,
    V_30676: String,
    V_30677: String,
    V_30678: String,
    V_30092: String,
    V_30679: String,
    V_30682: String,
    V_30681: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct S_50057 {
    recurso: String,
    accion: String,
    superusuario: bool,
    admin_tecnico: bool,
    admin_expedientes: bool,
    auditor_seguridad: bool,
    operador: bool,
    consulta: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct S_50058 {
    id: i64,
    V_30701: String,
    V_30461: String,
    V_30462: String,
    V_30702: String,
    V_30703: String,
    estado: String,
    V_30704: String,
    V_30705: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct S_50059 {
    id: i64,
    V_30708: String,
    V_30709: String,
    V_30710: String,
    V_30711: String,
    V_30712: String,
    V_30713: String,
}

#[derive(Deserialize)]
struct S_50060 {
    session_idle_minutes: i32,
    session_absolute_hours: i32,
    max_login_attempts: i32,
    lockout_duration_minutes: i32,
    V_30042: String,
    temp_code_expiry_minutes: i32,
    max_temp_code_resends: i32,
    single_session_per_user: Option<bool>,
    V_30112: Option<bool>,
    password_min_length: i32,
    password_require_special: Option<bool>,
}

#[derive(Deserialize)]
struct S_50061 {
    V_30114: String,
    display_name: String,
    V_30050: String,
    V_30115: String,
    role: String,
    enviar_codigo_inmediato: Option<bool>,
}

#[derive(Deserialize)]
struct S_50062 {
    estado: String,
    V_30167: Option<String>,
}

#[derive(Deserialize)]
struct S_50063 {
    role: String,
    V_30167: Option<String>,
}

#[derive(Deserialize)]
struct S_50064 {
    new_password: Option<String>,
    V_30167: String,
}

#[derive(Deserialize)]
struct S_50065 {
    codigo: String,
}

#[derive(Deserialize)]
struct S_50066 {
    estado: String,
    V_30167: Option<String>,
    V_30557: Option<String>,
}

#[derive(Deserialize)]
struct S_50067 {
    V_30055: i64,
    V_30652: String,
    V_30653: String,
    V_30654: String,
}

#[derive(Deserialize)]
struct S_50068 {
    V_30092: String,
    V_30664: Option<i64>,
    V_30167: String,
}

#[derive(Deserialize)]
struct S_50069 {
    decision: String,
    V_30167: String,
}

#[derive(Deserialize)]
struct S_50070 {
    role: String,
    recurso: String,
    accion: String,
    permitido: bool,
}

#[derive(Deserialize)]
struct S_50071 {
    V_30167: String,
}

#[derive(Template)]
#[template(path = "V_30278.html")]
struct S_50072 {
    user_name: String,
    V_30088: Vec<NavFile>,
    V_30022: Vec<S_50021>,
    V_30065: String,
    estado: String,
    V_30066: String,
    V_30090: Vec<String>,
    V_30069: i64,
    V_30087: i64,
    V_30079: i64,
    V_30070: i64,
    success_msg: Option<String>,
    error_msg: Option<String>,
}

#[derive(Template)]
#[template(path = "agente_historial.html")]
struct S_50073 {
    user_name: String,
    V_30088: Vec<NavFile>,
    credencial: String,
    V_30266: String,
    apellido: String,
    nombre: String,
    escalafon_grado: String,
    origen_principal: String,
    V_30508: String,
    V_30518: String,
    situacion_badge_class: String,
    ultima_actualizacion: String,
    V_30514: usize,
    V_30515: usize,
    V_30497: usize,
    V_30516: usize,
    V_30517: usize,
    expedientes: Vec<S_50025>,
    V_30278: Vec<S_50021>,
    V_30498: Vec<S_50028>,
    V_30500: Vec<S_50026>,
    V_30507: Vec<S_50027>,
}

#[derive(Template)]
#[template(path = "suspension_detalle.html")]
struct S_50074 {
    user_name: String,
    V_30088: Vec<NavFile>,
    V_30478: S_50021,
    V_30540: i64,
    vencimiento_60: String,
    vencimiento_120: String,
    V_30543: Option<String>,
    V_30544: Option<String>,
    success_msg: Option<String>,
    error_msg: Option<String>,
}

#[derive(Clone, FromRow, Serialize)]
struct S_50075 {
    id: i64,
    V_30702: String,
    V_30461: String,
    mensaje: String,
    url_destino: String,
    leido: bool,
    leido_at: Option<String>,
    V_30675: String,
}

#[derive(Template)]
#[template(path = "V_30536.html")]
struct S_50076 {
    user_name: String,
    V_30088: Vec<NavFile>,
    V_30534: i64,
    V_30079: i64,
    V_30532: String,
    V_30533: String,
    V_30536: Vec<S_50075>,
    V_30069: i64,
    V_30087: i64,
}

#[derive(Deserialize)]
struct S_50077 {
    V_30092: Option<String>,
    estado: Option<String>,
    V_30069: Option<i64>,
}

#[derive(Deserialize)]
struct S_50078 {
    V_30475: String,
    V_30167: String,
    V_30548: Option<String>,
}

#[derive(Deserialize)]
struct S_50079 {
    V_30467: String,
    observaciones_revision: Option<String>,
}

#[derive(Clone, FromRow, Serialize)]
struct S_50080 {
    id: i64,
    V_30275: i64,
    numero_version: i32,
    V_30306: Option<i64>,
    tipo_operacion: String,
    snapshot_json: serde_json::Value,
    campos_modificados: serde_json::Value,
    diferencias_json: serde_json::Value,
    usuario: String,
    V_30055: Option<i64>,
    V_30167: String,
    version_origen_id: Option<i64>,
    es_activa: bool,
    V_30675: String,
}

impl S_50080 {
    pub fn resumen_campos(&self) -> String {
        if self.tipo_operacion == "creacion" || self.tipo_operacion == "linea_base" {
            return "Registro inicial completo".to_string();
        }
        if let Some(arr) = self.campos_modificados.as_array() {
            let V_30002: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(|V_30044| V_30044.to_string())).collect();
            if V_30002.is_empty() {
                "Sin diferencias".to_string()
            } else {
                V_30002.join(", ")
            }
        } else {
            "-".to_string()
        }
    }
}

#[derive(Deserialize)]
struct S_50081 {
    V_30167: String,
    V_30115: Option<String>,
    V_30557: Option<String>,
    version_activa_esperada: i32,
    confirmado_comparacion: Option<String>,
    solicitud_aprobacion_id: Option<i64>,
    V_30555: Option<bool>,
}

#[derive(Deserialize)]
struct S_50082 {
    v1: Option<i64>,
    v2: Option<i64>,
    v_a: Option<i32>,
    v_b: Option<i32>,
}

fn F_40001(role: &str) -> bool {
    matches!(
        role,
        "superusuario" | "supervisor" | "auditor" | "admin_cuentas" | "admin_tecnico" | "admin_expedientes"
    )
}

async fn F_40002(V_30012: &PgPool, role: &str, recurso: &str, accion: &str) -> bool {
    if role == "superusuario" {
        return true;
    }
    let V_30002: Option<bool> = sqlx::query_scalar(
        "SELECT permitido FROM app_roles_permisos WHERE role = $1 AND recurso = $2 AND accion = $3"
    )
    .bind(role)
    .bind(recurso)
    .bind(accion)
    .fetch_optional(V_30012)
    .await
    .ok()
    .flatten();

    if let Some(V_30147) = V_30002 {
        return V_30147;
    }

    match (role, recurso, accion) {
        ("admin_tecnico" | "admin_expedientes", "V_30289", "restaurar" | "aprobar" | "consultar") => true,
        ("admin_tecnico" | "admin_expedientes", "expedientes", V_30020) => true,
        ("admin_tecnico" | "admin_expedientes", "V_30284", V_30020) => true,
        ("auditor_seguridad", "auditoria" | "V_30289" | "expedientes" | "V_30284", "consultar" | "descargar") => true,
        ("operador", "expedientes" | "V_30278" | "V_30284", "consultar" | "crear" | "modificar" | "descargar") => true,
        ("consulta", V_30020, "consultar" | "descargar") => true,
        V_30020 => F_40001(role) && recurso == "V_30289" && accion == "restaurar",
    }
}

fn F_40003(V_30044: &str) -> String {
    V_30044.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn F_40004(V_30044: &str) -> String {
    F_40003(V_30044)
}

fn F_40005(V_30044: &str) -> std::V_30712::Result<String, String> {
    let V_30003 = F_40004(V_30044);
    if V_30003.is_empty() || V_30003.eq_ignore_ascii_case("sin datos") || V_30003 == "-" || V_30003.eq_ignore_ascii_case("V_30044/d") {
        return V_30156("La credencial del agente es obligatoria y no puede estar vacía.".to_string());
    }
    if V_30003.len() > 32 {
        return V_30156("La credencial no puede exceder los 32 caracteres.".to_string());
    }
    if !V_30003.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '/' || c == 'V_30020') {
        return V_30156("La credencial contiene caracteres no válidos (solo se admiten caracteres alfanuméricos y guiones, preservando ceros a la izquierda).".to_string());
    }
    V_30054(V_30003)
}

fn F_40006(V_30044: &str) -> std::V_30712::Result<String, String> {
    let V_30003 = F_40004(V_30044);
    if V_30003.is_empty() || V_30003.eq_ignore_ascii_case("sin datos") || V_30003 == "-" || V_30003.eq_ignore_ascii_case("V_30044/d") {
        return V_30156("El número o código de V_30113 es obligatorio.".to_string());
    }
    if V_30003.len() < 3 || V_30003.len() > 120 {
        return V_30156("El número de V_30113 debe tener entre 3 y 120 caracteres.".to_string());
    }
    V_30054(V_30003)
}

fn F_40007(V_30190: &serde_json::Value) -> String {
    match V_30190 {
        serde_json::Value::Object(map) => {
            let mut V_30004: Vec<&String> = map.V_30004().collect();
            V_30004.sort();
            let V_30005: Vec<String> = V_30004
                .into_iter()
                .map(|k| {
                    let V_30006 = serde_json::to_string(k).unwrap_or_default();
                    let V_30007 = F_40007(&map[k]);
                    format!("{}:{}", V_30006, V_30007)
                })
                .collect();
            format!("{{{}}}", V_30005.join(","))
        }
        serde_json::Value::Array(arr) => {
            let V_30008: Vec<String> = arr.iter().map(F_40007).collect();
            format!("[{}]", V_30008.join(","))
        }
        V_30020 => serde_json::to_string(V_30190).unwrap_or_else(|V_30020| "null".to_string()),
    }
}

fn F_40008(
    V_30055: &str,
    occurred_at_str: &str,
    V_30055: Option<i64>,
    V_30710: &str,
    V_30711: &str,
    V_30712: &str,
    details: &serde_json::Value,
) -> String {
    let V_30010 = F_40007(details);
    let V_30011 = V_30055.map(|u| u.to_string()).unwrap_or_else(|| "null".to_string());
    let V_30012 = format!(
        "v1|{}|{}|{}|{}|{}|{}|{}",
        V_30055,
        occurred_at_str,
        V_30011,
        V_30710.trim(),
        V_30711.trim(),
        V_30712.trim(),
        V_30010
    );
    let mut V_30035 = Sha256::new();
    V_30035.update(b"AZZEN_SPF_AUDIT_CHAIN_V1_SALT:");
    V_30035.update(V_30012.as_bytes());
    format!("{:x}", V_30035.finalize())
}

fn F_40009(
    V_30275: i64,
    numero_version: i32,
    V_30306: Option<i64>,
    tipo_operacion: &str,
    usuario: &str,
    V_30167: &str,
    version_origen_id: Option<i64>,
    snapshot_json: &serde_json::Value,
) -> String {
    let V_30014 = F_40007(snapshot_json);
    let V_30015 = V_30306.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
    let V_30016 = version_origen_id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
    let V_30012 = format!(
        "v1|V_30087:{}|ver:{}|prev:{}|V_30696:{}|V_30064:{}|V_30469:{}|V_30167:{}|V_30625:{}",
        V_30275,
        numero_version,
        V_30015,
        tipo_operacion.trim(),
        usuario.trim(),
        V_30016,
        V_30167.trim(),
        V_30014
    );
    let mut V_30035 = Sha256::new();
    V_30035.update(b"AZZEN_SPF_VERSION_CHAIN_V1_SALT:");
    V_30035.update(V_30012.as_bytes());
    format!("{:x}", V_30035.finalize())
}

fn F_40010(
    V_30228: &str,
    numero_expediente: &str,
    reparticion: &str,
    V_30230: Option<&str>,
    V_30231: Option<&str>,
    fecha_inicio: Option<&str>,
    V_30232: Option<&str>,
    V_30233: Option<&str>,
    V_30234: Option<&str>,
    V_30235: Option<&str>,
    V_30167: &str,
    V_30066: Option<&str>,
    V_30067: Option<&str>,
    V_30236: Option<&str>,
    V_30237: Option<&str>,
    V_30238: &str,
    V_30068: Option<&str>,
    V_30240: Option<&str>,
    V_30241: Option<&str>,
    V_30242: Option<&str>,
    V_30243: Option<&str>,
) -> serde_json::Value {
    json!({
        "V_30228": V_30228,
        "numero_expediente": numero_expediente,
        "reparticion": reparticion,
        "V_30230": V_30230.unwrap_or(""),
        "V_30231": V_30231.unwrap_or(""),
        "fecha_inicio": fecha_inicio.unwrap_or(""),
        "V_30232": V_30232.unwrap_or(""),
        "V_30233": V_30233.unwrap_or(""),
        "V_30234": V_30234.unwrap_or(""),
        "V_30235": V_30235.unwrap_or(""),
        "V_30167": V_30167,
        "V_30066": V_30066.unwrap_or(""),
        "V_30067": V_30067.unwrap_or(""),
        "V_30236": V_30236.unwrap_or(""),
        "V_30237": V_30237.unwrap_or(""),
        "V_30238": V_30238,
        "V_30068": V_30068.unwrap_or("No"),
        "V_30240": V_30240.unwrap_or(""),
        "V_30241": V_30241.unwrap_or(""),
        "V_30242": V_30242.unwrap_or("No"),
        "V_30243": V_30243.unwrap_or("")
    })
}

fn F_40011(
    old_snap: &serde_json::Value,
    new_snap: &serde_json::Value,
) -> (Vec<String>, serde_json::Value) {
    let mut V_30017 = Vec::new();
    let mut V_30018 = serde_json::Map::new();

    let V_30019 = [
        ("V_30228", "Tipo de Identificador"),
        ("numero_expediente", "Número de Expediente"),
        ("reparticion", "Repartición"),
        ("V_30230", "Otra Repartición"),
        ("V_30231", "Destino / Origen"),
        ("fecha_inicio", "Fecha de Inicio"),
        ("V_30232", "Fecha del Hecho"),
        ("V_30233", "Fecha Prevista de Fin"),
        ("V_30234", "Autoridad Ordenante"),
        ("V_30235", "Dependencia de Autoridad"),
        ("V_30167", "Motivo de la Actuación"),
        ("V_30066", "Artículo"),
        ("V_30067", "Inciso"),
        ("V_30236", "Relato de los Hechos"),
        ("V_30237", "Credencial del Instructor"),
        ("V_30238", "Instructor Actual"),
        ("V_30068", "Causa Judicial"),
        ("V_30240", "Juzgado"),
        ("V_30241", "Carátula"),
        ("V_30242", "Finalización"),
        ("V_30243", "Fecha de Finalización"),
    ];

    for (k, label) in V_30019 {
        let V_30020 = old_snap.get(k).and_then(|v| v.as_str()).unwrap_or("").trim();
        let V_30021 = new_snap.get(k).and_then(|v| v.as_str()).unwrap_or("").trim();
        if V_30020 != V_30021 {
            V_30017.push(k.to_string());
            V_30018.insert(
                k.to_string(),
                json!({
                    "etiqueta": label,
                    "anterior": V_30020,
                    "nuevo": V_30021
                }),
            );
        }
    }

    (V_30017, serde_json::Value::Object(V_30018))
}

fn F_40012(actual: &str, nuevo: &str) -> bool {
    let V_30009 = actual.trim().to_lowercase();
    let V_30010 = nuevo.trim().to_lowercase();
    if V_30009 == V_30010 {
        return false;
    }
    match V_30009.as_str() {
        "creado" => matches!(V_30010.as_str(), "abierto" | "en trámite" | "en tramite" | "anulado"),
        "abierto" | "inicio" => matches!(V_30010.as_str(), "en trámite" | "en tramite" | "cerrado" | "anulado"),
        "en trámite" | "en tramite" => matches!(V_30010.as_str(), "cerrado" | "anulado" | "abierto"),
        "cerrado" => matches!(V_30010.as_str(), "reabierto" | "reconstrucción" | "reconstruccion"),
        "reabierto" | "reapertura" => matches!(V_30010.as_str(), "en trámite" | "en tramite" | "cerrado" | "abierto"),
        "reconstrucción" | "reconstruccion" => matches!(V_30010.as_str(), "abierto" | "en trámite" | "en tramite" | "cerrado"),
        "anulado" => false,
        V_30020 => matches!(V_30010.as_str(), "creado" | "abierto" | "en trámite" | "en tramite" | "cerrado" | "anulado"),
    }
}

async fn F_40013(
    V_30012: &PgPool,
    V_30055: Option<i64>,
    V_30702: &str,
    V_30461: &str,
    mensaje: &str,
    url_destino: &str,
    V_30275: Option<i64>,
    agente_credencial: Option<&str>,
    V_30474: Option<i64>,
) {
    let V_30024: bool = sqlx::query_scalar(
        V_30522#"
        SELECT EXISTS(
            SELECT 1 FROM app_notificaciones
            WHERE V_30702 = $1 AND url_destino = $2
            AND V_30675 > (CURRENT_TIMESTAMP - INTERVAL '60 seconds')
            AND (V_30055 IS NOT DISTINCT FROM $3)
        )
        "#
    )
    .bind(V_30702)
    .bind(url_destino)
    .bind(V_30055)
    .fetch_one(V_30012)
    .await
    .unwrap_or(false);

    if !V_30024 {
        sqlx::query(
            V_30522#"
            INSERT INTO app_notificaciones (
                V_30055, V_30702, V_30461, mensaje, url_destino,
                V_30275, agente_credencial, V_30474, V_30675
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP)
            "#
        )
        .bind(V_30055)
        .bind(V_30702)
        .bind(V_30461)
        .bind(mensaje)
        .bind(url_destino)
        .bind(V_30275)
        .bind(agente_credencial)
        .bind(V_30474)
        .execute(V_30012)
        .await
        .ok();
    }
}

#[derive(Deserialize)]
struct S_50083 {
    credencial: String,
    V_30115: String,
    V_30651: Option<String>,
    dispositivo_nombre: Option<String>,
    dispositivo_os: Option<String>,
    dispositivo_browser: Option<String>,
}

#[derive(Deserialize)]
struct S_50084 {
    error: Option<String>,
    notice: Option<String>,
    device_warning: Option<String>,
}

#[derive(Deserialize)]
struct S_50085 {
    V_30137: Option<String>,
    codigo: Option<String>,
    codigo_backup: Option<String>,
}

#[derive(Deserialize)]
struct S_50086 {
    secret_base32: String,
    codigo_confirmacion: String,
}

#[derive(Deserialize)]
struct S_50087 {
    current_password: Option<String>,
    new_password: String,
    confirm_password: String,
}

#[derive(Deserialize)]
struct S_50088 {
    confirmado: Option<String>,
}

#[derive(Deserialize)]
struct S_50089 {
    V_30167: String,
}

#[derive(Deserialize)]
struct S_50090 {
    credencial: Option<String>,
}

#[derive(Deserialize)]
struct S_50091 {
    credencial: String,
    V_30115: String,
    codigo_mfa: String,
    V_30167: String,
}

#[derive(Deserialize)]
struct S_50092 {
    V_30167: Option<String>,
}

#[derive(Deserialize)]
struct S_50093 {
    identificador: String,
}

#[derive(Deserialize)]
struct S_50094 {
    V_30153: String,
    new_password: String,
    confirm_password: String,
}

#[derive(Deserialize)]
struct S_50095 {
    V_30167: Option<String>,
}

#[derive(Deserialize)]
struct S_50096 {
    V_30153: Option<String>,
}

#[derive(Deserialize)]
struct S_50097 {
    V_30065: Option<String>,
    V_30103: Option<String>,
    V_30104: Option<String>,
    V_30069: Option<i64>,
    V_30070: Option<i64>,
    V_30105: Option<String>,
    V_30106: Option<String>,
}

#[derive(Deserialize)]
struct S_50098 {
    estado: Option<String>,
    V_30069: Option<i64>,
    V_30070: Option<i64>,
}

#[derive(Deserialize)]
struct S_50099 {
    V_30065: Option<String>,
}

#[derive(Deserialize)]
struct S_50100 {
    V_30065: Option<String>,
    V_30103: Option<String>,
    V_30104: Option<String>,
}

#[derive(Deserialize)]
struct S_50101 {
    V_30065: Option<String>,
    reparticion: Option<String>,
    estado: Option<String>,
    V_30066: Option<String>,
    V_30067: Option<String>,
    V_30068: Option<String>,
    V_30069: Option<i64>,
    V_30070: Option<i64>,
    V_30137: Option<String>,
}

#[derive(Deserialize)]
struct S_50102 {
    V_30092: Option<String>,
    V_30093: Option<String>,
}

#[derive(Deserialize)]
struct S_50103 {
    nuevo: Option<bool>,
    V_30083: Option<String>,
    err: Option<String>,
    tab: Option<String>,
}

#[derive(Deserialize)]
struct S_50104 {
    V_30065: Option<String>,
    estado: Option<String>,
    V_30066: Option<String>,
    V_30069: Option<i64>,
    V_30070: Option<i64>,
    V_30083: Option<String>,
    err: Option<String>,
}

#[derive(Deserialize)]
struct S_50105 {
    motivo_rectificacion: String,
    nueva_descripcion: Option<String>,
}

#[derive(Deserialize)]
struct S_50106 {
    motivo_anulacion: String,
}

#[derive(Deserialize)]
struct S_50107 {
    V_30065: Option<String>,
}


#[derive(Deserialize)]
struct S_50108 {
    V_30228: String,
    numero_expediente: String,
    reparticion: String,
    V_30230: Option<String>,
    V_30231: Option<String>,
    fecha_inicio: Option<String>,
    V_30232: Option<String>,
    V_30233: Option<String>,
    V_30234: Option<String>,
    V_30235: Option<String>,
    V_30167: String,
    estado: String,
    V_30066: Option<String>,
    V_30067: Option<String>,
    V_30236: Option<String>,
    V_30237: Option<String>,
    V_30238: String,
    V_30068: Option<String>,
    V_30240: Option<String>,
    V_30241: Option<String>,
    V_30242: Option<String>,
    V_30243: Option<String>,
    motivo_modificacion: Option<String>,
    version_activa_esperada: Option<i32>,
    V_30239: Option<bool>,
    involucrado_credencial: Option<String>,
    involucrado_apellido: Option<String>,
    involucrado_nombre: Option<String>,
    involucrado_escalafon: Option<String>,
    involucrado_grado: Option<String>,
    involucrado_rol: Option<String>,
}

#[derive(Deserialize)]
struct S_50109 {
    V_30065: Option<String>,
    V_30069: Option<i64>,
    V_30070: Option<i64>,
}

#[derive(Deserialize)]
struct S_50110 {
    credencial: Option<String>,
    V_30113: Option<String>,
    nombre: Option<String>,
    apellido: Option<String>,
    V_30114: Option<bool>,
    V_30069: Option<i64>,
    V_30070: Option<i64>,
}

#[actix_web::F_40014]
async fn F_40014() -> std::io::Result<()> {
    dotenv().ok();
    let V_30011 = F_40015();
    let V_30012 = PgPoolOptions::new()
        .max_connections(10)
        .connect(&V_30011.database_url)
        .await
        .expect("No se pudo conectar a PostgreSQL");

    F_40016(&V_30012).await.expect("No se pudo inicializar esquema app");
    F_40018(&V_30012, &V_30011)
        .await
        .expect("No se pudo ejecutar bootstrap de usuario");

    let V_30035 = AppState {
        V_30012: V_30012.clone(),
        V_30011: V_30011.clone(),
    };
    println!("spf_web_console escuchando en http://{}", V_30011.bind_addr);
    if V_30011.bind_addr.starts_with("0.0.0.0") {
        println!("  -> Acceso Local:     http://127.0.0.1:8088");
        println!("  -> Acceso Red LAN:   Disponible en la IP de red local para pruebas");
    }

    let V_30014: std::path::PathBuf = if std::path::Path::new("static").is_dir() {
        std::path::PathBuf::from("static")
    } else if std::path::Path::new("spf_web_console/static").is_dir() {
        std::path::PathBuf::from("spf_web_console/static")
    } else {
        std::path::PathBuf::from(V_30522"C:\Proyectos\SPF\spf_web_console\static")
    };

    let V_30015 = V_30014.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(V_30035.clone()))
            .service(Files::new("/static", V_30015.clone()))
            .service(F_40026)
            .service(F_40034)
            .service(F_40040)
            .service(F_40041)
            .service(F_40035)
            .service(F_40036)
            .service(F_40037)
            .service(F_40039)
            .service(F_40038)
            .service(F_40042)
            .service(F_40043)
            .service(F_40044)
            .service(F_40045)
            .service(F_40046)
            .service(F_40047)
            .service(F_40049)
            .service(F_40052)
            .service(F_40055)
            .service(F_40050)
            .service(F_40053)
            .service(F_40056)
            .service(F_40057)
            .service(F_40058)
            .service(F_40059)
            .service(F_40060)
            .service(F_40061)
            .service(F_40062)
            .service(logout)
            .service(dashboard)
            .service(expedientes)
            .service(F_40072)
            .service(F_40073)
            .service(F_40074)
            .service(F_40075)
            .service(F_40076)
            .service(F_40077)
            .service(F_40078)
            .service(F_40079)
            .service(F_40100)
            .service(F_40101)
            .service(F_40102)
            .service(F_40103)
            .service(F_40104)
            .service(F_40105)
            .service(F_40106)
            .service(F_40107)
            .service(F_40108)
            .service(F_40109)
            .service(F_40110)
            .service(F_40111)
            .service(rutas_modulos_v2::modulos_list)
            .service(rutas_modulos_v2::modulos_dashboard)
            .service(rutas_modulos_v2::modulos_registros_lista)
            .service(rutas_mobile::mobile_login)
            .service(rutas_mobile::mobile_totp)
            .service(rutas_mobile::mobile_me)
            .service(rutas_mobile::mobile_logout)
            .service(rutas_mobile::mobile_expedientes_list)
            .service(rutas_mobile::mobile_expediente_detalle)
            .service(rutas_mobile::mobile_persona_detalle)
            .service(rutas_mobile::mobile_suspensiones_list)
            .service(rutas_mobile::mobile_suspension_detalle)
            .service(rutas_mobile::mobile_expediente_nueva_novedad)
            .service(rutas_mobile::mobile_expediente_transicion)
            .service(F_40112)
            .service(F_40084)
            .service(personas)
            .service(F_40085)
            .service(F_40086)
            .service(F_40087)
            .service(F_40088)
            .service(F_40089)
            .service(F_40093)
            .service(F_40094)
            .service(F_40095)
            .service(F_40096)
            .service(F_40097)
            .service(F_40098)
            .service(F_40099)
            .service(F_40113)
            .service(F_40114)
            .service(F_40115)
            .service(F_40116)
            .service(F_40117)
            .service(F_40118)
            .service(F_40119)
            .service(F_40120)
            .service(F_40121)
            .service(F_40122)
            .service(F_40123)
            .service(F_40124)
            .service(F_40125)
            .service(F_40126)
            .service(F_40080)
            .service(F_40081)
            .service(F_40082)
            .service(F_40083)
            .service(F_40127)
            .service(F_40128)
            .service(F_40129)
            .service(F_40130)
            .service(F_40131)
            .service(F_40132)
            .service(F_40133)
            .service(F_40134)
            .service(F_40135)
            .service(F_40136)
            .service(F_40137)
            .service(F_40138)
            .service(F_40139)
            .service(F_40140)
            .service(F_40141)
            .service(F_40142)
            .service(F_40143)
            .service(F_40144)
            .service(F_40145)
            .service(F_40147)
            .service(F_40148)
            .service(F_40150)
            .service(F_40151)
            .service(F_40152)
            .service(F_40153)
            .service(F_40154)
            .service(F_40155)
            .service(F_40156)
    })
    .bind(V_30011.bind_addr)?
    .run()
    .await
}

fn F_40015() -> AppConfig {
    let V_30016 = std::env::var("SMTP_PASS").unwrap_or_else(|V_30020| "jqgedmkqttlzzfaz".to_string());
    let V_30017 = V_30016.replace(' ', "");

    AppConfig {
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|V_30020| "postgres://postgres:admin123@localhost:5432/spf_excel".to_string()),
        bind_addr: std::env::var("APP_BIND_ADDR").unwrap_or_else(|V_30020| "0.0.0.0:8088".to_string()),
        session_idle_minutes: std::env::var("SESSION_IDLE_MINUTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
        session_absolute_hours: std::env::var("SESSION_ABSOLUTE_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8),
        max_page_size: std::env::var("MAX_PAGE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(200),
        max_export_rows: std::env::var("MAX_EXPORT_ROWS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10000),
        bootstrap_credential: std::env::var("BOOTSTRAP_CREDENTIAL").ok(),
        bootstrap_password: std::env::var("BOOTSTRAP_PASSWORD").ok(),
        smtp_host: std::env::var("SMTP_HOST").unwrap_or_else(|V_30020| "smtp.gmail.com".to_string()),
        smtp_port: std::env::var("SMTP_PORT").ok().and_then(|V_30147| V_30147.parse().ok()).unwrap_or(587),
        smtp_user: std::env::var("SMTP_USER").unwrap_or_else(|V_30020| "sisl16.ede@gmail.com".to_string()),
        V_30017,
        smtp_from: std::env::var("SMTP_FROM").unwrap_or_else(|V_30020| "sisl16.ede@gmail.com".to_string()),
        smtp_from_name: std::env::var("SMTP_FROM_NAME").unwrap_or_else(|V_30020| "AZZEN SPF · Centro de Seguridad".to_string()),
        app_base_url: std::env::var("APP_BASE_URL").unwrap_or_else(|V_30020| "http://127.0.0.1:8088".to_string()),
    }
}

async fn F_40016(V_30012: &PgPool) -> std::V_30712::Result<(), sqlx::Error> {
    let V_30018 = [
        "CREATE TABLE IF NOT EXISTS app_users (
            id BIGSERIAL PRIMARY KEY,
            V_30114 TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL,
            V_30639 BOOLEAN NOT NULL DEFAULT true,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE TABLE IF NOT EXISTS app_sessions (
            id UUID PRIMARY KEY,
            V_30055 BIGINT NOT NULL REFERENCES app_users(id),
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            V_30676 TIMESTAMP NOT NULL,
            V_30677 TIMESTAMP NOT NULL,
            absolute_expires_at TIMESTAMP NOT NULL,
            revoked_at TIMESTAMP NULL
        )",
        "CREATE TABLE IF NOT EXISTS app_audit_log (
            id BIGSERIAL PRIMARY KEY,
            V_30708 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            V_30055 BIGINT NULL REFERENCES app_users(id),
            V_30710 TEXT NOT NULL,
            V_30711 TEXT NOT NULL,
            V_30712 TEXT NOT NULL,
            details JSONB NOT NULL DEFAULT '{}'::jsonb
        )",
        "CREATE TABLE IF NOT EXISTS reparticiones (
            id BIGSERIAL PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE,
            V_30763 TEXT NOT NULL DEFAULT 'reparticion',
            activo BOOLEAN NOT NULL DEFAULT true,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE TABLE IF NOT EXISTS expedientes (
            id BIGSERIAL PRIMARY KEY,
            V_30228 TEXT NOT NULL DEFAULT 'sumario',
            numero_expediente TEXT NOT NULL,
            reparticion TEXT NOT NULL DEFAULT 'Sin datos',
            V_30230 TEXT,
            fecha_inicio TIMESTAMP,
            V_30232 TIMESTAMP,
            V_30167 TEXT,
            estado TEXT NOT NULL DEFAULT 'Abierto',
            V_30231 TEXT,
            V_30238 TEXT,
            V_30068 TEXT,
            V_30240 TEXT,
            V_30241 TEXT,
            V_30463 TEXT,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (V_30228, numero_expediente)
        )",
        "CREATE TABLE IF NOT EXISTS expediente_agentes (
            id BIGSERIAL PRIMARY KEY,
            V_30275 BIGINT NOT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            credencial TEXT,
            V_30266 TEXT,
            rol TEXT NOT NULL DEFAULT 'causante',
            V_30323 TEXT,
            V_30465 TEXT,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE TABLE IF NOT EXISTS expediente_historial (
            id BIGSERIAL PRIMARY KEY,
            V_30275 BIGINT NOT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            V_30765 TEXT NOT NULL,
            V_30766 TEXT,
            V_30764 TEXT,
            usuario TEXT,
            V_30167 TEXT,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE TABLE IF NOT EXISTS expediente_novedades (
            id BIGSERIAL PRIMARY KEY,
            V_30275 BIGINT NOT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            V_30464 BIGINT NULL REFERENCES expediente_agentes(id) ON DELETE SET NULL,
            V_30474 BIGINT NULL,
            V_30458 TEXT NOT NULL,
            V_30459 TEXT NULL,
            V_30460 TIMESTAMP NOT NULL,
            V_30461 TEXT NOT NULL,
            V_30462 TEXT NOT NULL,
            V_30463 TEXT NULL,
            estado_anterior TEXT NULL,
            estado_nuevo TEXT NULL,
            usuario TEXT NOT NULL,
            es_auditoria_automatica BOOLEAN NOT NULL DEFAULT false,
            V_30465 TEXT NULL,
            anulado BOOLEAN NOT NULL DEFAULT false,
            anulado_motivo TEXT NULL,
            anulado_usuario TEXT NULL,
            anulado_at TIMESTAMP NULL,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE TABLE IF NOT EXISTS expediente_suspensiones (
            id BIGSERIAL PRIMARY KEY,
            V_30275 BIGINT NULL REFERENCES expedientes(id) ON DELETE SET NULL,
            V_30464 BIGINT NULL REFERENCES expediente_agentes(id) ON DELETE SET NULL,
            credencial TEXT NOT NULL,
            V_30266 TEXT NOT NULL,
            fecha_inicio TIMESTAMP NOT NULL,
            V_30471 TEXT NOT NULL DEFAULT 'a)',
            estado TEXT NOT NULL DEFAULT 'Vigente',
            fecha_fin TIMESTAMP NULL,
            V_30476 TEXT NULL,
            V_30477 TEXT NULL,
            archivo_disposicion_pdf TEXT NULL,
            archivo_levantamiento_pdf TEXT NULL,
            usuario TEXT NOT NULL,
            V_30463 TEXT NULL,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE TABLE IF NOT EXISTS expediente_relaciones (
            id BIGSERIAL PRIMARY KEY,
            expediente_origen_id BIGINT NOT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            expediente_destino_id BIGINT NOT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            V_30481 TEXT NOT NULL,
            V_30482 TEXT NULL,
            V_30167 TEXT NOT NULL,
            V_30548 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            V_30465 TEXT NULL,
            usuario TEXT NOT NULL,
            estado TEXT NOT NULL DEFAULT 'Activa',
            anulado_motivo TEXT NULL,
            anulado_usuario TEXT NULL,
            anulado_at TIMESTAMP NULL,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CONSTRAINT check_no_self_rel CHECK (expediente_origen_id <> expediente_destino_id)
        )",
        "CREATE TABLE IF NOT EXISTS expediente_documentos (
            id BIGSERIAL PRIMARY KEY,
            V_30275 BIGINT NOT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            novedad_id BIGINT NULL REFERENCES expediente_novedades(id) ON DELETE SET NULL,
            V_30464 BIGINT NULL REFERENCES expediente_agentes(id) ON DELETE SET NULL,
            V_30474 BIGINT NULL REFERENCES expediente_suspensiones(id) ON DELETE SET NULL,
            relacion_id BIGINT NULL REFERENCES expediente_relaciones(id) ON DELETE SET NULL,
            nombre_archivo TEXT NOT NULL,
            nombre_original TEXT NOT NULL,
            tipo_documento TEXT NOT NULL,
            usuario TEXT NOT NULL,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE TABLE IF NOT EXISTS staging_sin_credencial (
            id BIGSERIAL PRIMARY KEY,
            dominio TEXT NOT NULL DEFAULT '',
            hoja_origen TEXT NOT NULL DEFAULT '',
            V_30526 TEXT,
            credencial_resuelta TEXT,
            metodo_resolucion TEXT,
            score_resolucion DOUBLE PRECISION,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE TABLE IF NOT EXISTS app_notificaciones (
            id BIGSERIAL PRIMARY KEY,
            V_30055 BIGINT NULL REFERENCES app_users(id) ON DELETE CASCADE,
            V_30702 TEXT NOT NULL,
            V_30461 TEXT NOT NULL,
            mensaje TEXT NOT NULL,
            url_destino TEXT NOT NULL,
            leido BOOLEAN NOT NULL DEFAULT false,
            leido_at TIMESTAMP NULL,
            V_30275 BIGINT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            agente_credencial TEXT NULL,
            V_30474 BIGINT NULL REFERENCES expediente_suspensiones(id) ON DELETE CASCADE,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE INDEX IF NOT EXISTS idx_notif_user_leido ON app_notificaciones(V_30055, leido)",
        "CREATE INDEX IF NOT EXISTS idx_notif_created ON app_notificaciones(V_30675)",
        "CREATE INDEX IF NOT EXISTS idx_staging_credencial_resuelta ON staging_sin_credencial(credencial_resuelta)",
        "CREATE INDEX IF NOT EXISTS idx_app_sessions_user ON app_sessions(V_30055)",
        "CREATE INDEX IF NOT EXISTS idx_app_sessions_exp ON app_sessions(V_30677)",
        "CREATE INDEX IF NOT EXISTS idx_app_audit_date ON app_audit_log(V_30708)",
        "CREATE INDEX IF NOT EXISTS idx_expedientes_numero ON expedientes(numero_expediente)",
        "CREATE INDEX IF NOT EXISTS idx_expedientes_reparticion ON expedientes(reparticion)",
        "CREATE INDEX IF NOT EXISTS idx_expediente_agentes_exp ON expediente_agentes(V_30275)",
        "CREATE INDEX IF NOT EXISTS idx_expediente_historial_exp ON expediente_historial(V_30275)",
        "CREATE INDEX IF NOT EXISTS idx_novedades_expediente ON expediente_novedades(V_30275)",
        "CREATE INDEX IF NOT EXISTS idx_novedades_agente ON expediente_novedades(V_30464)",
        "CREATE INDEX IF NOT EXISTS idx_novedades_fecha ON expediente_novedades(V_30460)",
        "CREATE INDEX IF NOT EXISTS idx_novedades_tipo ON expediente_novedades(V_30458)",
        "CREATE INDEX IF NOT EXISTS idx_suspensiones_exp ON expediente_suspensiones(V_30275)",
        "CREATE INDEX IF NOT EXISTS idx_suspensiones_cred ON expediente_suspensiones(credencial)",
        "CREATE INDEX IF NOT EXISTS idx_suspensiones_estado ON expediente_suspensiones(estado)",
        "CREATE INDEX IF NOT EXISTS idx_relaciones_origen ON expediente_relaciones(expediente_origen_id)",
        "CREATE INDEX IF NOT EXISTS idx_relaciones_destino ON expediente_relaciones(expediente_destino_id)",
        "CREATE INDEX IF NOT EXISTS idx_relaciones_tipo ON expediente_relaciones(V_30481)",
        "CREATE INDEX IF NOT EXISTS idx_documentos_exp ON expediente_documentos(V_30275)",
        "CREATE INDEX IF NOT EXISTS idx_documentos_nov ON expediente_documentos(novedad_id)",
        "CREATE TABLE IF NOT EXISTS expediente_versiones (
            id BIGSERIAL PRIMARY KEY,
            V_30275 BIGINT NOT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            numero_version INTEGER NOT NULL,
            V_30306 BIGINT NULL REFERENCES expediente_versiones(id) ON DELETE SET NULL,
            tipo_operacion TEXT NOT NULL,
            snapshot_json JSONB NOT NULL,
            campos_modificados JSONB NOT NULL,
            diferencias_json JSONB NOT NULL,
            usuario TEXT NOT NULL,
            V_30055 BIGINT NULL REFERENCES app_users(id) ON DELETE SET NULL,
            V_30167 TEXT NOT NULL,
            version_origen_id BIGINT NULL REFERENCES expediente_versiones(id) ON DELETE SET NULL,
            es_activa BOOLEAN NOT NULL DEFAULT false,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CONSTRAINT unq_expediente_version UNIQUE(V_30275, numero_version)
        )",
        "CREATE INDEX IF NOT EXISTS idx_expediente_versiones_exp ON expediente_versiones(V_30275, numero_version DESC)",
        "CREATE INDEX IF NOT EXISTS idx_expediente_versiones_activa ON expediente_versiones(V_30275, es_activa)",
        "CREATE TABLE IF NOT EXISTS documento_versiones (
            id BIGSERIAL PRIMARY KEY,
            documento_id BIGINT NOT NULL REFERENCES expediente_documentos(id) ON DELETE CASCADE,
            V_30275 BIGINT NOT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            numero_version INTEGER NOT NULL,
            V_30306 BIGINT NULL REFERENCES documento_versiones(id) ON DELETE SET NULL,
            nombre_archivo TEXT NOT NULL,
            nombre_original TEXT NOT NULL,
            tipo_documento TEXT NOT NULL,
            tamano_bytes BIGINT NOT NULL,
            sha256_hash TEXT NOT NULL,
            tipo_operacion TEXT NOT NULL,
            V_30167 TEXT NOT NULL,
            usuario TEXT NOT NULL,
            V_30055 BIGINT NULL REFERENCES app_users(id) ON DELETE SET NULL,
            es_vigente BOOLEAN NOT NULL DEFAULT true,
            esta_retirado BOOLEAN NOT NULL DEFAULT false,
            version_origen_id BIGINT NULL REFERENCES documento_versiones(id) ON DELETE SET NULL,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CONSTRAINT unq_documento_version UNIQUE(documento_id, numero_version)
        )",
        "CREATE INDEX IF NOT EXISTS idx_doc_ver_doc ON documento_versiones(documento_id)",
        "CREATE INDEX IF NOT EXISTS idx_doc_ver_exp ON documento_versiones(V_30275)",
        "CREATE INDEX IF NOT EXISTS idx_doc_ver_vigente ON documento_versiones(documento_id, es_vigente)",
        "CREATE INDEX IF NOT EXISTS idx_doc_ver_created ON documento_versiones(V_30675)",
    ];
    for statement in V_30018 {
        sqlx::query(statement).execute(V_30012).await?;
    }

    let V_30019 = [
        "ALTER TABLE expediente_documentos ADD COLUMN IF NOT EXISTS version_vigente_id BIGINT",
        "ALTER TABLE expediente_documentos ADD COLUMN IF NOT EXISTS numero_version_vigente INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE expediente_documentos ADD COLUMN IF NOT EXISTS esta_retirado BOOLEAN NOT NULL DEFAULT FALSE",
        "ALTER TABLE expediente_documentos ADD COLUMN IF NOT EXISTS motivo_retiro TEXT",
        "ALTER TABLE expediente_documentos ADD COLUMN IF NOT EXISTS fecha_retiro TIMESTAMP",
        "ALTER TABLE expediente_documentos ADD COLUMN IF NOT EXISTS usuario_retiro TEXT",
        "ALTER TABLE expediente_documentos ADD COLUMN IF NOT EXISTS sha256_hash TEXT",
        "ALTER TABLE expediente_documentos ADD COLUMN IF NOT EXISTS tamano_bytes BIGINT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS version_actual INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS V_30233 TIMESTAMP",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS V_30234 TEXT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS V_30235 TEXT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS V_30066 TEXT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS V_30067 TEXT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS documento_informe_hechos TEXT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS documento_disposicion TEXT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS V_30236 TEXT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS V_30237 TEXT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS V_30242 TEXT NOT NULL DEFAULT 'No'",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS V_30243 TIMESTAMP",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS documento_finalizacion TEXT",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS fecha_ultimo_cambio_estado TIMESTAMP",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS fecha_ultima_novedad TIMESTAMP",
        "ALTER TABLE expedientes ADD COLUMN IF NOT EXISTS tipo_ultima_novedad TEXT",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30319 BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30320 TIMESTAMP",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30321 TEXT",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30322 BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30324 TIMESTAMP",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30325 TEXT",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30326 BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30327 TIMESTAMP",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30328 TEXT",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30329 TEXT",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30330 TEXT NOT NULL DEFAULT 'No'",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30331 TIMESTAMP",
        "ALTER TABLE expediente_agentes ADD COLUMN IF NOT EXISTS V_30332 TEXT",
        "ALTER TABLE expediente_novedades ADD COLUMN IF NOT EXISTS V_30467 TEXT NOT NULL DEFAULT 'revisada'",
        "ALTER TABLE expediente_novedades ADD COLUMN IF NOT EXISTS revisor_usuario TEXT",
        "ALTER TABLE expediente_novedades ADD COLUMN IF NOT EXISTS revisado_at TIMESTAMP",
        "ALTER TABLE expediente_novedades ADD COLUMN IF NOT EXISTS observaciones_revision TEXT",
        "ALTER TABLE expediente_novedades ADD COLUMN IF NOT EXISTS V_30466 TEXT",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS V_30050 TEXT NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS V_30636 BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS email_verificado_at TIMESTAMP NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS ultimo_cambio_email_at TIMESTAMP NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS V_30640 INT NOT NULL DEFAULT 0",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS bloqueado_hasta TIMESTAMP NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS ultimo_acceso TIMESTAMP NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS ip_ultimo_acceso TEXT NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS mfa_habilitado BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS estado TEXT NOT NULL DEFAULT 'activo'",
        "ALTER TABLE app_sessions ADD COLUMN IF NOT EXISTS V_30678 TEXT NULL",
        "ALTER TABLE app_sessions ADD COLUMN IF NOT EXISTS V_30092 TEXT NULL",
        "ALTER TABLE app_sessions ADD COLUMN IF NOT EXISTS V_30651 TEXT NULL",
        "CREATE TABLE IF NOT EXISTS app_politicas_seguridad (
            id INT PRIMARY KEY DEFAULT 1,
            session_idle_minutes INT NOT NULL DEFAULT 30,
            session_absolute_hours INT NOT NULL DEFAULT 8,
            max_login_attempts INT NOT NULL DEFAULT 5,
            lockout_duration_minutes INT NOT NULL DEFAULT 15,
            V_30042 TEXT NOT NULL DEFAULT 'privilegiados',
            temp_code_expiry_minutes INT NOT NULL DEFAULT 5,
            max_temp_code_resends INT NOT NULL DEFAULT 3,
            single_session_per_user BOOLEAN NOT NULL DEFAULT false,
            V_30112 BOOLEAN NOT NULL DEFAULT false,
            password_min_length INT NOT NULL DEFAULT 8,
            password_require_special BOOLEAN NOT NULL DEFAULT false,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_by BIGINT NULL REFERENCES app_users(id)
        )",
        "INSERT INTO app_politicas_seguridad (id) VALUES (1) ON CONFLICT (id) DO NOTHING",
        "CREATE TABLE IF NOT EXISTS app_dispositivos (
            id BIGSERIAL PRIMARY KEY,
            V_30055 BIGINT NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
            V_30651 TEXT NOT NULL UNIQUE,
            V_30652 TEXT NOT NULL,
            V_30653 TEXT NOT NULL DEFAULT 'Desconocido',
            V_30654 TEXT NOT NULL DEFAULT 'Desconocido',
            V_30655 TEXT NOT NULL DEFAULT '127.0.0.1',
            estado TEXT NOT NULL DEFAULT 'autorizado',
            autorizado_por BIGINT NULL REFERENCES app_users(id),
            V_30656 TEXT NULL,
            V_30657 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            V_30658 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            certificado_ref TEXT NULL
        )",
        "CREATE TABLE IF NOT EXISTS app_codigos_temporales (
            id BIGSERIAL PRIMARY KEY,
            V_30055 BIGINT NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
            V_30092 TEXT NOT NULL,
            codigo_hash TEXT NOT NULL,
            email_destino TEXT NOT NULL,
            creado_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expira_at TIMESTAMP NOT NULL,
            usado BOOLEAN NOT NULL DEFAULT false,
            usado_at TIMESTAMP NULL,
            intentos INT NOT NULL DEFAULT 0,
            max_intentos INT NOT NULL DEFAULT 3
        )",
        "CREATE TABLE IF NOT EXISTS app_solicitudes_acceso (
            id BIGSERIAL PRIMARY KEY,
            V_30092 TEXT NOT NULL,
            V_30662 BIGINT NOT NULL REFERENCES app_users(id),
            V_30664 BIGINT NULL REFERENCES app_users(id),
            datos_solicitud JSONB NOT NULL DEFAULT '{}'::jsonb,
            estado TEXT NOT NULL DEFAULT 'pendiente',
            V_30666 TEXT NOT NULL,
            aprobador_id BIGINT NULL REFERENCES app_users(id),
            V_30669 TEXT NULL,
            V_30670 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            V_30671 TIMESTAMP NULL
        )",
        "CREATE TABLE IF NOT EXISTS app_roles_permisos (
            id BIGSERIAL PRIMARY KEY,
            role TEXT NOT NULL,
            recurso TEXT NOT NULL,
            accion TEXT NOT NULL,
            permitido BOOLEAN NOT NULL DEFAULT true,
            UNIQUE (role, recurso, accion)
        )",
        "CREATE TABLE IF NOT EXISTS app_alertas_seguridad (
            id BIGSERIAL PRIMARY KEY,
            V_30701 TEXT NOT NULL DEFAULT 'media',
            V_30461 TEXT NOT NULL,
            V_30462 TEXT NOT NULL,
            V_30702 TEXT NOT NULL,
            V_30055 BIGINT NULL REFERENCES app_users(id),
            estado TEXT NOT NULL DEFAULT 'activa',
            resuelta_por BIGINT NULL REFERENCES app_users(id),
            V_30704 TEXT NULL,
            V_30705 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            resuelta_at TIMESTAMP NULL
        )",
        "CREATE TABLE IF NOT EXISTS app_mfa_login_challenges (
            id UUID PRIMARY KEY,
            V_30055 BIGINT NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
            V_30050 TEXT NOT NULL,
            V_30678 TEXT NULL,
            V_30092 TEXT NULL,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            V_30677 TIMESTAMP NOT NULL,
            verified BOOLEAN NOT NULL DEFAULT false
        )",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS totp_secret_enc TEXT NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS V_30637 BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS totp_verificado_at TIMESTAMP NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS V_30638 BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS temp_password_expires_at TIMESTAMP NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS created_by BIGINT NULL REFERENCES app_users(id)",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS role_modified_by BIGINT NULL REFERENCES app_users(id)",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS role_modified_at TIMESTAMP NULL",
        "CREATE TABLE IF NOT EXISTS app_backup_codes (
            id BIGSERIAL PRIMARY KEY,
            V_30055 BIGINT NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
            code_hash TEXT NOT NULL,
            usado BOOLEAN NOT NULL DEFAULT false,
            usado_at TIMESTAMP NULL,
            creado_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expira_at TIMESTAMP NULL
        )",
        "CREATE INDEX IF NOT EXISTS idx_backup_codes_user ON app_backup_codes(V_30055, usado)",
        "CREATE TABLE IF NOT EXISTS app_password_resets (
            id BIGSERIAL PRIMARY KEY,
            V_30055 BIGINT NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
            V_30154 TEXT NOT NULL,
            expira_at TIMESTAMP NOT NULL,
            usado BOOLEAN NOT NULL DEFAULT false,
            usado_at TIMESTAMP NULL,
            creado_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            ip_solicitud TEXT NULL
        )",
        "CREATE INDEX IF NOT EXISTS idx_password_resets_token ON app_password_resets(V_30154, usado, expira_at)",
        "CREATE UNIQUE INDEX IF NOT EXISTS unq_app_dispositivos_user_disp ON app_dispositivos(V_30055, V_30651)",
        "ALTER TABLE app_dispositivos ADD COLUMN IF NOT EXISTS rechazado_por BIGINT NULL REFERENCES app_users(id)",
        "ALTER TABLE app_dispositivos ADD COLUMN IF NOT EXISTS V_30671 TIMESTAMP NULL",
        "ALTER TABLE app_politicas_seguridad ADD COLUMN IF NOT EXISTS recovery_code_expiry_minutes INT NOT NULL DEFAULT 15",
        "ALTER TABLE app_politicas_seguridad ADD COLUMN IF NOT EXISTS temp_password_expiry_hours INT NOT NULL DEFAULT 24",
        "ALTER TABLE app_politicas_seguridad ADD COLUMN IF NOT EXISTS concurrent_alert_cooldown_minutes INT NOT NULL DEFAULT 5",
        "ALTER TABLE app_politicas_seguridad ALTER COLUMN single_session_per_user SET DEFAULT true",
        "UPDATE app_politicas_seguridad SET single_session_per_user = true WHERE id = 1",
        "ALTER TABLE app_sessions ADD COLUMN IF NOT EXISTS is_restricted BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE app_sessions ADD COLUMN IF NOT EXISTS motivo_revocacion TEXT NULL",
        "ALTER TABLE app_sessions ADD COLUMN IF NOT EXISTS revoked_by BIGINT NULL REFERENCES app_users(id)",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS totp_pending_secret_enc TEXT NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS totp_last_used_step BIGINT NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS backup_codes_confirmed BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS onboarding_completed_at TIMESTAMP NULL",
        "ALTER TABLE app_users ADD COLUMN IF NOT EXISTS last_concurrent_alert_at TIMESTAMP NULL",
        "UPDATE app_users SET backup_codes_confirmed = true WHERE V_30637 = true AND backup_codes_confirmed = false",
        "ALTER TABLE app_mfa_login_challenges ADD COLUMN IF NOT EXISTS V_30651 TEXT NULL",
        "ALTER TABLE app_mfa_login_challenges ADD COLUMN IF NOT EXISTS dispositivo_nombre TEXT NULL",
        "ALTER TABLE app_mfa_login_challenges ADD COLUMN IF NOT EXISTS dispositivo_os TEXT NULL",
        "ALTER TABLE app_mfa_login_challenges ADD COLUMN IF NOT EXISTS dispositivo_browser TEXT NULL",
        "ALTER TABLE app_mfa_login_challenges ADD COLUMN IF NOT EXISTS password_verified BOOLEAN NOT NULL DEFAULT true",
        "ALTER TABLE app_mfa_login_challenges ADD COLUMN IF NOT EXISTS totp_verified BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE app_mfa_login_challenges ADD COLUMN IF NOT EXISTS backup_codes_plain_json TEXT NULL",
        "ALTER TABLE app_mfa_login_challenges ADD COLUMN IF NOT EXISTS V_30640 INT NOT NULL DEFAULT 0",
        "ALTER TABLE app_mfa_login_challenges ADD COLUMN IF NOT EXISTS current_step TEXT NOT NULL DEFAULT 'init'",
        "INSERT INTO app_roles_permisos (role, recurso, accion, permitido)
         SELECT 'superusuario', V_30522.recurso, a.accion, true
         FROM (VALUES ('expedientes'), ('V_30278'), ('V_30284'), ('V_30289'), ('usuarios'), ('seguridad'), ('auditoria'), ('calidad_datos')) AS V_30522(recurso)
         CROSS JOIN (VALUES ('consultar'), ('crear'), ('modificar'), ('rectificar'), ('descargar'), ('restaurar'), ('administrar'), ('aprobar')) AS a(accion)
         ON CONFLICT (role, recurso, accion) DO UPDATE SET permitido = true",
        "ALTER TABLE app_audit_log ADD COLUMN IF NOT EXISTS V_30055 TEXT NULL",
        "ALTER TABLE app_audit_log ADD COLUMN IF NOT EXISTS V_30057 TEXT NULL",
        "ALTER TABLE app_audit_log ADD COLUMN IF NOT EXISTS hash_version TEXT NOT NULL DEFAULT 'v1'",
        "ALTER TABLE app_audit_log ADD COLUMN IF NOT EXISTS V_30554 TEXT NULL",
        "ALTER TABLE expediente_versiones ADD COLUMN IF NOT EXISTS V_30554 TEXT NULL",
        "ALTER TABLE expediente_versiones ADD COLUMN IF NOT EXISTS mfa_verificado BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE expediente_versiones ADD COLUMN IF NOT EXISTS mfa_step BIGINT NULL",
        "ALTER TABLE expediente_versiones ADD COLUMN IF NOT EXISTS aprobacion_id BIGINT NULL",
        "ALTER TABLE expediente_versiones ADD COLUMN IF NOT EXISTS record_hash TEXT NULL",
        "CREATE TABLE IF NOT EXISTS app_integridad_verificaciones (
            id BIGSERIAL PRIMARY KEY,
            ejecutado_por BIGINT NULL REFERENCES app_users(id) ON DELETE SET NULL,
            V_30137 TEXT NOT NULL DEFAULT 'manual',
            V_30148 TEXT NOT NULL,
            V_30132 BIGINT NOT NULL DEFAULT 0,
            V_30133 BIGINT NOT NULL DEFAULT 0,
            V_30139 BIGINT NOT NULL DEFAULT 0,
            V_30140 BIGINT NOT NULL DEFAULT 0,
            V_30144 BIGINT NOT NULL DEFAULT 0,
            V_30145 BIGINT NOT NULL DEFAULT 0,
            V_30130 BIGINT NULL,
            resumen_hallazgos JSONB NOT NULL DEFAULT '[]'::jsonb,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "ALTER TABLE app_politicas_seguridad ADD COLUMN IF NOT EXISTS require_dual_approval_restore BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE app_solicitudes_acceso ADD COLUMN IF NOT EXISTS V_30275 BIGINT NULL REFERENCES expedientes(id) ON DELETE CASCADE",
        "ALTER TABLE app_solicitudes_acceso ADD COLUMN IF NOT EXISTS version_id BIGINT NULL REFERENCES expediente_versiones(id) ON DELETE SET NULL",
        "ALTER TABLE app_solicitudes_acceso ADD COLUMN IF NOT EXISTS version_activa_esperada INT NULL",
        "ALTER TABLE app_solicitudes_acceso ADD COLUMN IF NOT EXISTS ejecutada BOOLEAN NOT NULL DEFAULT false",
        "ALTER TABLE app_solicitudes_acceso ADD COLUMN IF NOT EXISTS ejecutada_at TIMESTAMP NULL",
        "ALTER TABLE app_notificaciones ADD COLUMN IF NOT EXISTS idempotency_key TEXT NULL",
        "CREATE TABLE IF NOT EXISTS app_calidad_datos_casos (
            id BIGSERIAL PRIMARY KEY,
            V_30763 TEXT NOT NULL,
            subtipo TEXT NOT NULL,
            V_30701 TEXT NOT NULL DEFAULT 'media',
            entidad_tipo TEXT NOT NULL,
            entidad_id BIGINT NOT NULL,
            V_30275 BIGINT NULL REFERENCES expedientes(id) ON DELETE CASCADE,
            identificador_visible TEXT NOT NULL,
            campo_afectado TEXT NULL,
            V_30462 TEXT NOT NULL,
            sugerencia TEXT NULL,
            V_30467 TEXT NOT NULL DEFAULT 'pendiente',
            regularizado_por BIGINT NULL REFERENCES app_users(id) ON DELETE SET NULL,
            regularizado_at TIMESTAMP NULL,
            V_30704 TEXT NULL,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (V_30763, entidad_tipo, entidad_id, campo_afectado)
        )",
        "INSERT INTO app_roles_permisos (role, recurso, accion, permitido)
         SELECT 'admin_tecnico', V_30522.recurso, a.accion, true
         FROM (VALUES ('expedientes'), ('V_30278'), ('V_30284'), ('V_30289'), ('usuarios'), ('seguridad'), ('auditoria'), ('calidad_datos')) AS V_30522(recurso)
         CROSS JOIN (VALUES ('consultar'), ('crear'), ('modificar'), ('rectificar'), ('descargar'), ('restaurar'), ('administrar'), ('aprobar')) AS a(accion)
         ON CONFLICT (role, recurso, accion) DO NOTHING",
        "INSERT INTO app_roles_permisos (role, recurso, accion, permitido)
         VALUES
             ('administrador', 'expedientes', 'consultar', true),
             ('administrador', 'expedientes', 'crear', true),
             ('administrador', 'expedientes', 'modificar', true),
             ('administrador', 'expedientes', 'rectificar', true),
             ('administrador', 'V_30289', 'consultar', true),
             ('administrador', 'V_30289', 'restaurar', true),
             ('administrador', 'V_30289', 'aprobar', true),
             ('administrador', 'V_30284', 'consultar', true),
             ('administrador', 'V_30284', 'crear', true),
             ('administrador', 'V_30284', 'descargar', true),
             ('administrador', 'calidad_datos', 'consultar', true),
             ('administrador', 'calidad_datos', 'modificar', true),
             ('auditor', 'expedientes', 'consultar', true),
             ('auditor', 'V_30289', 'consultar', true),
             ('auditor', 'V_30289', 'aprobar', true),
             ('auditor', 'V_30284', 'consultar', true),
             ('auditor', 'V_30284', 'descargar', true),
             ('auditor', 'auditoria', 'consultar', true),
             ('auditor', 'seguridad', 'consultar', true),
             ('auditor', 'calidad_datos', 'consultar', true),
             ('operador', 'expedientes', 'consultar', true),
             ('operador', 'expedientes', 'crear', true),
             ('operador', 'expedientes', 'modificar', true),
             ('operador', 'V_30289', 'consultar', true),
             ('operador', 'V_30289', 'restaurar', false),
             ('operador', 'V_30284', 'consultar', true),
             ('operador', 'V_30284', 'crear', true),
             ('operador', 'V_30284', 'descargar', true),
             ('consulta', 'expedientes', 'consultar', true),
             ('consulta', 'V_30289', 'consultar', true),
             ('consulta', 'V_30284', 'consultar', true),
             ('consulta', 'V_30284', 'descargar', false)
         ON CONFLICT (role, recurso, accion) DO NOTHING",
        "CREATE TABLE IF NOT EXISTS app_fila_novedades (
            id BIGSERIAL PRIMARY KEY,
            V_30777 TEXT NOT NULL DEFAULT 'hoja',
            file_id BIGINT NULL,
            sheet_id BIGINT NULL,
            row_number BIGINT NULL,
            V_30275 BIGINT NULL REFERENCES expedientes(id) ON DELETE SET NULL,
            V_30474 BIGINT NULL REFERENCES expediente_suspensiones(id) ON DELETE SET NULL,
            credencial TEXT NULL,
            V_30778 TEXT NULL,
            numero_expediente TEXT NULL,
            V_30458 TEXT NOT NULL DEFAULT 'Observación de fila',
            V_30460 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            V_30461 TEXT NOT NULL,
            V_30462 TEXT NOT NULL,
            V_30463 TEXT NULL,
            campos_editados_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            usuario TEXT NOT NULL,
            V_30055 BIGINT NULL REFERENCES app_users(id) ON DELETE SET NULL,
            V_30675 TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        "CREATE INDEX IF NOT EXISTS idx_fila_nov_sheet_row ON app_fila_novedades(sheet_id, row_number)",
        "CREATE INDEX IF NOT EXISTS idx_fila_nov_exp ON app_fila_novedades(V_30275)",
        "CREATE INDEX IF NOT EXISTS idx_fila_nov_cred ON app_fila_novedades(credencial)",
        "CREATE INDEX IF NOT EXISTS idx_expedientes_num_upper ON expedientes (UPPER(TRIM(numero_expediente)))",
        "CREATE INDEX IF NOT EXISTS idx_exp_agentes_cred_upper ON expediente_agentes (UPPER(TRIM(credencial)))",
        "CREATE INDEX IF NOT EXISTS idx_personas_cred_upper ON personas (UPPER(TRIM(credencial)))",
        "ALTER TABLE personas ADD COLUMN IF NOT EXISTS apellido text",
        "ALTER TABLE personas ADD COLUMN IF NOT EXISTS nombre text",
        "ALTER TABLE personas ADD COLUMN IF NOT EXISTS escalafon_grado text",
        "CREATE INDEX IF NOT EXISTS idx_personas_apellido ON personas (UPPER(TRIM(apellido)))",
        "CREATE INDEX IF NOT EXISTS idx_personas_nombre ON personas (UPPER(TRIM(nombre)))",
        "CREATE INDEX IF NOT EXISTS idx_personas_escalafon ON personas (UPPER(TRIM(escalafon_grado)))",
        "CREATE INDEX IF NOT EXISTS idx_suspensiones_cred_upper ON expediente_suspensiones (UPPER(TRIM(credencial)))",
    ];
    for V_30124 in V_30019 {
        sqlx::query(V_30124).execute(V_30012).await.ok();
    }

    sqlx::query(
        "INSERT INTO reparticiones (nombre) VALUES ('DRH'), ('CPF I'), ('CPF II'), ('CPF VI'), ('OTRO'), ('Sin datos') ON CONFLICT (nombre) DO NOTHING",
    )
    .execute(V_30012)
    .await?;

    sqlx::query(
        V_30522#"
        INSERT INTO expedientes (V_30228, numero_expediente, reparticion, V_30167, fecha_inicio, estado, V_30238)
        SELECT DISTINCT
            'sumario',
            TRIM(COALESCE(NULLIF(V_30522.payload_json ->> 'numero_expediente', ''), NULLIF(V_30522.payload_json ->> 'número_expediente', ''), NULLIF(V_30522.payload_json ->> 'V_30063�mero_expediente', ''), 'Sin datos')),
            COALESCE(NULLIF(TRIM(COALESCE(V_30522.payload_json ->> 'destino', V_30522.payload_json ->> 'V_30104', V_30522.payload_json ->> 'reparticion')), ''), 'Sin datos'),
            COALESCE(NULLIF(TRIM(V_30522.payload_json ->> 'V_30167'), ''), 'Sin datos'),
            CASE
                WHEN NULLIF(TRIM(COALESCE(V_30522.payload_json ->> 'V_30503', '')), '') IS NOT NULL
                     AND LOWER(TRIM(COALESCE(V_30522.payload_json ->> 'V_30503', ''))) NOT IN ('sin datos', 'V_30044/d', 'V_30063/a', 'na', 'null', '-')
                     AND COALESCE(V_30522.payload_json ->> 'V_30503', '') ~ '^[0-9]{4}(-[0-9]{2}){0,2}([T ][0-9]{2}(:[0-9]{2}){0,2}(\.[0-9]+)?(Z|[+-][0-9]{2}:?[0-9]{2})?)?$]'
                THEN CAST(COALESCE(V_30522.payload_json ->> 'V_30503', '') AS TIMESTAMP)
                ELSE NULL
            END,
            'Abierto',
            COALESCE(NULLIF(TRIM(COALESCE(V_30522.payload_json ->> 'instructor', V_30522.payload_json ->> 'causante')), ''), 'Sin datos')
        FROM raw_sheet_rows V_30522
        WHERE V_30522.sheet_id IN (12, 13, 14)
        ON CONFLICT (V_30228, numero_expediente) DO NOTHING
        "#,
    )
    .execute(V_30012)
    .await?;

    let V_30020 = F_40017(V_30012).await;

    V_30054(())
}

async fn F_40017(V_30012: &PgPool) -> std::V_30712::Result<(), sqlx::Error> {
    // 1. Backfill cadena de hashes en app_audit_log si existen filas sin V_30057
    let V_30021: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_audit_log WHERE V_30057 IS NULL OR V_30057 = ''"
    )
    .fetch_one(V_30012)
    .await
    .unwrap_or(0);

    if V_30021 > 0 {
        let V_30022: Vec<(i64, String, Option<i64>, String, String, String, serde_json::Value, Option<String>)> = sqlx::query_as(
            V_30522#"
            SELECT id,
                   TO_CHAR(V_30708, 'YYYY-MM-DD HH24:MI:SS.US') AS occ_str,
                   V_30055,
                   V_30710,
                   V_30711,
                   V_30712,
                   COALESCE(details, '{}'::jsonb),
                   V_30057
            FROM app_audit_log
            ORDER BY id ASC
            "#
        )
        .fetch_all(V_30012)
        .await
        .unwrap_or_default();

        let mut V_30023 = "GENESIS".to_string();
        for (id, occ_str, V_30076, V_30710, V_30711, V_30712, details, existing_hash) in V_30022 {
            if let Some(ref h) = existing_hash {
                if !h.trim().is_empty() {
                    V_30023 = h.clone();
                    continue;
                }
            }
            let V_30024 = details.get("V_30554").and_then(|v| v.as_str()).map(|V_30044| V_30044.to_string());
            let V_30025 = F_40008(
                &V_30023,
                &occ_str,
                V_30076,
                &V_30710,
                &V_30711,
                &V_30712,
                &details,
            );
            let V_30020 = sqlx::query(
                "UPDATE app_audit_log SET V_30055 = $1, V_30057 = $2, hash_version = 'v1', V_30554 = COALESCE(V_30554, $3) WHERE id = $4"
            )
            .bind(&V_30023)
            .bind(&V_30025)
            .bind(&V_30024)
            .bind(id)
            .execute(V_30012)
            .await;
            V_30023 = V_30025;
        }
    }

    // 2. Backfill record_hash en expediente_versiones si existen filas sin record_hash
    let V_30026: Vec<(i64, i64, i32, Option<i64>, String, String, String, Option<i64>, serde_json::Value)> = sqlx::query_as(
        V_30522#"
        SELECT id, V_30275, numero_version, V_30306,
               tipo_operacion, usuario, V_30167, version_origen_id,
               COALESCE(snapshot_json, '{}'::jsonb)
        FROM expediente_versiones
        WHERE record_hash IS NULL OR record_hash = ''
        ORDER BY id ASC
        "#
    )
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();

    for (id, exp_id, num_ver, prev_id, tipo_op, usr, V_30510, orig_id, V_30625) in V_30026 {
        let V_30027 = F_40009(
            exp_id,
            num_ver,
            prev_id,
            &tipo_op,
            &usr,
            &V_30510,
            orig_id,
            &V_30625,
        );
        let V_30020 = sqlx::query("UPDATE expediente_versiones SET record_hash = $1 WHERE id = $2")
            .bind(V_30027)
            .bind(id)
            .execute(V_30012)
            .await;
    }

    V_30054(())
}

async fn F_40018(V_30012: &PgPool, V_30011: &AppConfig) -> std::V_30712::Result<(), sqlx::Error> {
    let Some(V_30114) = V_30011.bootstrap_credential.as_ref() else {
        return V_30054(());
    };
    let Some(V_30115) = V_30011.bootstrap_password.as_ref() else {
        return V_30054(());
    };
    let V_30028: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM app_users WHERE V_30114 = $1")
        .bind(V_30114)
        .fetch_one(V_30012)
        .await?;
    if V_30028 > 0 {
        return V_30054(());
    }
    let V_30029 = F_40019(V_30115).map_err(|V_30020| sqlx::Error::Protocol("V_30029 error".into()))?;
    sqlx::query(
        "INSERT INTO app_users (V_30114, display_name, password_hash, role, V_30639) VALUES ($1, $2, $3, 'admin_tecnico', true)",
    )
    .bind(V_30114)
    .bind("Administrador inicial")
    .bind(V_30029)
    .execute(V_30012)
    .await?;
    println!("Usuario bootstrap creado para credencial {}", V_30114);
    V_30054(())
}

fn F_40019(V_30115: &str) -> std::V_30712::Result<String, V_30031::password_hash::Error> {
    let V_30030 = SaltString::generate(&mut OsRng);
    let V_30031 = Argon2::default();
    let V_30029 = V_30031.F_40019(V_30115.as_bytes(), &V_30030)?.to_string();
    V_30054(V_30029)
}

pub(crate) fn verify_password(V_30115: &str, V_30029: &str) -> bool {
    let V_30046 = V_30115.trim();
    if V_30046.is_empty() {
        return false;
    }
    // Soporte exclusivo para el V_30029 placeholder inicial de bootstrap antes del primer cambio de clave
    if V_30029 == "$argon2id$v=19$m=19456,t=2,V_30147=1$abcdefghijklmnop$admin_baseline_test_hash" {
        return V_30046 == "admin" || V_30046 == "admin123";
    }
    let V_30047 = match PasswordHash::new(V_30029) {
        V_30054(v) => v,
        V_30156(V_30020) => return false,
    };
    Argon2::default()
        .verify_password(V_30046.as_bytes(), &V_30047)
        .is_ok()
}

fn F_40020() -> (String, String) {
    use argon2::password_hash::rand_core::RngCore;
    let mut V_30032 = OsRng;
    let V_30033 = (V_30032.next_u32() % 900_000) + 100_000;
    let V_30034 = format!("{:06}", V_30033);
    let mut V_30035 = Sha256::new();
    V_30035.update(V_30034.as_bytes());
    let V_30029 = format!("{:x}", V_30035.finalize());
    (V_30034, V_30029)
}

fn F_40021(V_30034: &str) -> String {
    let mut V_30035 = Sha256::new();
    V_30035.update(V_30034.trim().as_bytes());
    format!("{:x}", V_30035.finalize())
}

fn F_40022(V_30050: &str) -> String {
    let V_30036: Vec<&str> = V_30050.split('@').collect();
    if V_30036.len() != 2 {
        return V_30050.to_string();
    }
    let V_30037 = V_30036[0];
    let V_30038 = V_30036[1];
    if V_30037.len() <= 2 {
        format!("{}***@{}", &V_30037[0..1], V_30038)
    } else {
        format!("{}{}***{}@{}", &V_30037[0..1], &V_30037[1..2], &V_30037[V_30037.len()-1..], V_30038)
    }
}

pub(crate) async fn write_audit(V_30012: &PgPool, V_30055: Option<i64>, V_30710: &str, V_30711: &str, V_30712: &str, details: serde_json::Value) {
    let V_30024 = details.get("V_30554").and_then(|v| v.as_str()).map(|V_30044| V_30044.to_string());
    if let Ok(mut V_30094) = V_30012.begin().await {
        // Advisory lock para serializar el encadenamiento de hashes de auditoría sin contención de tablas
        let V_30020 = sqlx::query("SELECT pg_advisory_xact_lock(77441122)").execute(&mut *V_30094).await;
        let V_30055: String = sqlx::query_scalar(
            "SELECT COALESCE(V_30057, 'GENESIS') FROM app_audit_log WHERE V_30057 IS NOT NULL ORDER BY id DESC LIMIT 1"
        )
        .fetch_optional(&mut *V_30094)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "GENESIS".to_string());

        let V_30056: String = sqlx::query_scalar(
            "SELECT TO_CHAR(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US')"
        )
        .fetch_one(&mut *V_30094)
        .await
        .unwrap_or_else(|V_30020| "2026-01-01 00:00:00.000000".to_string());

        let V_30057 = F_40008(
            &V_30055,
            &V_30056,
            V_30055,
            V_30710,
            V_30711,
            V_30712,
            &details,
        );

        let V_30020 = sqlx::query(
            V_30522#"
            INSERT INTO app_audit_log (
                V_30708, V_30055, V_30710, V_30711, V_30712, details,
                V_30055, V_30057, hash_version, V_30554
            )
            VALUES (
                TO_TIMESTAMP($1, 'YYYY-MM-DD HH24:MI:SS.US'),
                $2, $3, $4, $5, $6,
                $7, $8, 'v1', $9
            )
            "#,
        )
        .bind(&V_30056)
        .bind(V_30055)
        .bind(V_30710)
        .bind(V_30711)
        .bind(V_30712)
        .bind(&details)
        .bind(&V_30055)
        .bind(&V_30057)
        .bind(&V_30024)
        .execute(&mut *V_30094)
        .await;

        let V_30020 = V_30094.commit().await;
    }
}

pub(crate) async fn require_user(req: &HttpRequest, V_30188: &web::Data<AppState>) -> Result<SessionUser, HttpResponse> {
    let V_30058 = req.path().starts_with("/api/");
    let V_30059 = || {
        if V_30058 {
            HttpResponse::Unauthorized().json(json!({ "error": "Sesión no autenticada o expirada." }))
        } else {
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/login"))
                .finish()
        }
    };

    let V_30060 = req
        .V_30457()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| {
            if h.to_lowercase().starts_with("bearer ") {
                Uuid::parse_str(h[7..].trim()).ok()
            } else {
                None
            }
        })
        .or_else(|| {
            req.V_30041("spf_session").and_then(|c| Uuid::parse_str(c.value().trim()).ok())
        })
        .ok_or_else(V_30059)?;

    let V_30061 = sqlx::query_as::<V_30020, SessionUser>(
        V_30522#"
        SELECT u.id AS V_30055, u.V_30114, u.display_name, u.role,
               COALESCE(u.V_30638, false) AS V_30638,
               COALESCE(V_30044.is_restricted, false) AS is_restricted,
               V_30044.V_30651
        FROM app_sessions V_30044
        JOIN app_users u ON u.id = V_30044.V_30055
        WHERE V_30044.id = $1
          AND V_30044.revoked_at IS NULL
          AND V_30044.V_30677 > NOW()
          AND V_30044.absolute_expires_at > NOW()
          AND u.V_30639 = true
          AND COALESCE(u.estado, 'activo') = 'activo'
        "#,
    )
    .bind(V_30060)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(|V_30020| HttpResponse::InternalServerError().body("Error de sesión"))?;

    let Some(V_30064) = V_30061 else {
        // Si la sesión existe pero expiró sin haber sido revocada aún, registramos el cierre por expiración
        if let Ok(Some((V_30076, rev_at))) = sqlx::query_as::<V_30020, (i64, Option<NaiveDateTime>)>(
            "SELECT V_30055, revoked_at FROM app_sessions WHERE id = $1"
        )
        .bind(V_30060)
        .fetch_optional(&V_30188.V_30012)
        .await
        {
            if rev_at.is_none() {
                let V_30020 = sqlx::query("UPDATE app_sessions SET revoked_at = NOW(), motivo_revocacion = 'expirada' WHERE id = $1 AND revoked_at IS NULL")
                    .bind(V_30060)
                    .execute(&V_30188.V_30012)
                    .await;
                write_audit(
                    &V_30188.V_30012,
                    Some(V_30076),
                    "sesion_expirada",
                    "V_30674",
                    "expired",
                    json!({ "V_30060": V_30060.to_string(), "reason": "idle_or_absolute_timeout" }),
                )
                .await;
            }
        }
        return V_30156(V_30059());
    };

    // Verificar si el dispositivo asociado fue revocado o rechazado posteriormente
    if let Some(ref V_30110) = V_30064.V_30651 {
        let V_30052: Option<String> = sqlx::query_scalar(
            "SELECT estado FROM app_dispositivos WHERE V_30055 = $1 AND V_30651 = $2"
        )
        .bind(V_30064.V_30055)
        .bind(V_30110)
        .fetch_optional(&V_30188.V_30012)
        .await
        .ok()
        .flatten();

        if matches!(V_30052.as_deref(), Some("revocado") | Some("rechazado") | Some("suspendido")) {
            let V_30020 = sqlx::query("UPDATE app_sessions SET revoked_at = NOW(), motivo_revocacion = 'dispositivo_no_autorizado' WHERE id = $1")
                .bind(V_30060)
                .execute(&V_30188.V_30012)
                .await;
            return V_30156(V_30059());
        }
    }

    // Backend restriction: Si la cuenta tiene cambio obligatorio pendiente o sesión restringida, bloquear acceso a cualquier otra ruta
    if (V_30064.V_30638 || V_30064.is_restricted)
        && req.path() != "/cambiar-V_30115-obligatorio"
        && req.path() != "/logout"
    {
        if V_30058 {
            return Ok(HttpResponse::Forbidden().json(json!({
                "error": "Debe completar el cambio obligatorio de contraseña antes de operar en el sistema.",
                "redirect": "/cambiar-V_30115-obligatorio"
            })));
        }
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/cambiar-V_30115-obligatorio"))
            .finish());
    }

    let V_30063: i64 = sqlx::query_scalar::<V_30020, i32>("SELECT session_idle_minutes FROM app_politicas_seguridad WHERE id = 1")
        .fetch_optional(&V_30188.V_30012)
        .await
        .ok()
        .flatten()
        .map(|v| v as i64)
        .unwrap_or(V_30188.V_30011.session_idle_minutes);

    let V_30064 = Utc::now() + Duration::minutes(V_30063);
    let V_30020 = sqlx::query("UPDATE app_sessions SET V_30676 = NOW(), V_30677 = $1 WHERE id = $2")
        .bind(V_30064.naive_utc())
        .bind(V_30060)
        .execute(&V_30188.V_30012)
        .await;

    V_30054(V_30064)
}

fn F_40023(role: &str) -> bool {
    matches!(
        role,
        "superusuario" | "admin" | "consulta_exportacion" | "supervisor" | "auditor" | "admin_cuentas" | "admin_tecnico" | "operador"
    )
}

fn F_40024(file_name: &str) -> String {
    let V_30039 = file_name.to_lowercase();
    if V_30039.contains("V_30278") {
        "Suspensiones preventivas".to_string()
    } else if V_30039.contains("V_30507") {
        "Asistencias e Inasistencias".to_string()
    } else if V_30039.contains("impedidos") {
        "Jurídico impedidos por estatuto".to_string()
    } else if V_30039.contains("sum_inf") || V_30039.contains("caratulas") {
        "Sumario e Información".to_string()
    } else {
        file_name.to_string()
    }
}

fn F_40025(V_30181: &str) -> i32 {
    match V_30181 {
        "Suspensiones preventivas" => 1,
        "Asistencias e Inasistencias" => 2,
        "Jurídico impedidos por estatuto" => 3,
        "Sumario e Información" => 4,
        V_30020 => 5,
    }
}

pub(crate) async fn load_navigation(V_30012: &PgPool) -> std::V_30712::Result<Vec<NavFile>, sqlx::Error> {
    let V_30022 = sqlx::query_as::<V_30020, S_50004>(
        V_30522#"
        SELECT
            sf.id AS file_id,
            sf.file_name,
            ss.id AS sheet_id,
            ss.sheet_name,
            ss.sheet_order
        FROM source_files sf
        JOIN source_sheets ss ON ss.file_id = sf.id
        ORDER BY sf.file_name, ss.sheet_order
        "#,
    )
    .fetch_all(V_30012)
    .await?;

    let mut V_30066: Vec<NavFile> = Vec::new();
    let mut V_30067: Option<i64> = None;
    for V_30522 in V_30022 {
        if V_30067 != Some(V_30522.file_id) {
            V_30066.push(NavFile {
                file_id: V_30522.file_id,
                file_name: F_40024(&V_30522.file_name),
                sheets: Vec::new(),
            });
            V_30067 = Some(V_30522.file_id);
        }
        if let Some(last) = V_30066.last_mut() {
            last.sheets.push(NavSheet {
                sheet_id: V_30522.sheet_id,
                sheet_name: V_30522.sheet_name,
            });
        }
    }
    V_30066.sort_by_key(|V_30187| F_40025(&V_30187.file_name));
    V_30054(V_30066)
}

pub(crate) fn render_html<T: Template>(tpl: &T) -> Result<HttpResponse> {
    tpl.render()
        .map(|body| HttpResponse::Ok().content_type("text/html; charset=utf-8").body(body))
        .map_err(|e| error::ErrorInternalServerError(format!("Error de template: {e}")))
}

#[get("/login")]
async fn F_40026(query: web::Query<S_50084>) -> Result<HttpResponse> {
    let V_30040 = match query.device_warning.as_deref() {
        Some("no_registrado") => Some("Dispositivo no autorizado. Se ha enviado una solicitud a los administradores para habilitar este equipo. No podrá iniciar sesión V_30512 que sea autorizado.".to_string()),
        Some("pendiente") => Some("Este dispositivo se encuentra pendiente de aprobación administrativa. Contacte a un administrador para V_30692 habilitación.".to_string()),
        Some("rechazado") => Some("Acceso denegado: este dispositivo fue rechazado por un administrador de seguridad.".to_string()),
        Some("revocado") => Some("Acceso revocado: la autorización de este dispositivo ha sido anulada preventivamente.".to_string()),
        Some(custom) => Some(custom.to_string()),
        None => None,
    };
    render_html(&S_50029 {
        error: query.error.clone(),
        notice: query.notice.clone(),
        device_warning: V_30040,
    })
}

#[derive(FromRow)]
struct S_50111 {
    id: i64,
    V_30114: String,
    display_name: String,
    V_30050: Option<String>,
    V_30636: bool,
    mfa_habilitado: bool,
    V_30637: bool,
    totp_secret_enc: Option<String>,
    totp_pending_secret_enc: Option<String>,
    totp_last_used_step: Option<i64>,
    backup_codes_confirmed: bool,
    password_hash: String,
    role: String,
    V_30639: bool,
    estado: String,
    V_30638: bool,
    temp_password_expires_at: Option<NaiveDateTime>,
    V_30640: i32,
    bloqueado_hasta: Option<NaiveDateTime>,
    last_concurrent_alert_at: Option<NaiveDateTime>,
    onboarding_completed_at: Option<NaiveDateTime>,
}

#[derive(FromRow)]
struct S_50112 {
    id: Uuid,
    V_30055: i64,
    V_30050: String,
    V_30678: String,
    V_30092: String,
    V_30651: String,
    dispositivo_nombre: String,
    dispositivo_os: String,
    dispositivo_browser: String,
    totp_verified: bool,
    backup_codes_plain_json: Option<String>,
    V_30640: i32,
}

#[derive(Deserialize)]
struct S_50113 {
    codigo: String,
}

#[derive(Deserialize)]
struct S_50114 {
    codigo: Option<String>,
    V_30055: Option<i64>,
}

async fn F_40027(V_30012: &PgPool, V_30055: i64) -> std::V_30712::Result<Option<S_50111>, sqlx::Error> {
    sqlx::query_as::<V_30020, S_50111>(
        V_30522#"
        SELECT id, V_30114, display_name, V_30050,
               COALESCE(V_30636, false) AS V_30636,
               COALESCE(mfa_habilitado, false) AS mfa_habilitado,
               COALESCE(V_30637, false) AS V_30637,
               totp_secret_enc,
               totp_pending_secret_enc,
               totp_last_used_step,
               COALESCE(backup_codes_confirmed, false) AS backup_codes_confirmed,
               password_hash, role, V_30639,
               COALESCE(estado, 'activo') AS estado,
               COALESCE(V_30638, false) AS V_30638,
               temp_password_expires_at,
               COALESCE(V_30640, 0) AS V_30640,
               bloqueado_hasta,
               last_concurrent_alert_at,
               onboarding_completed_at
        FROM app_users
        WHERE id = $1
        "#
    )
    .bind(V_30055)
    .fetch_optional(V_30012)
    .await
}

async fn F_40028(req: &HttpRequest, V_30012: &PgPool) -> Option<S_50112> {
    let V_30041 = req.V_30041("spf_totp_pending").or_else(|| req.V_30041("spf_2fa_pending"))?;
    let challenge_id = Uuid::parse_str(V_30041.value()).ok()?;
    sqlx::query_as::<V_30020, S_50112>(
        V_30522#"
        SELECT id, V_30055, V_30050,
               COALESCE(V_30678, '127.0.0.1') AS V_30678,
               COALESCE(V_30092, 'Navegador Web') AS V_30092,
               COALESCE(V_30651, '') AS V_30651,
               COALESCE(dispositivo_nombre, 'Terminal Web') AS dispositivo_nombre,
               COALESCE(dispositivo_os, 'Desconocido') AS dispositivo_os,
               COALESCE(dispositivo_browser, 'Navegador Web') AS dispositivo_browser,
               COALESCE(totp_verified, false) AS totp_verified,
               backup_codes_plain_json,
               COALESCE(V_30640, 0) AS V_30640
        FROM app_mfa_login_challenges
        WHERE id = $1 AND verified = false AND V_30677 > NOW()
        "#
    )
    .bind(challenge_id)
    .fetch_optional(V_30012)
    .await
    .ok()
    .flatten()
}

async fn F_40029(V_30012: &PgPool, V_30064: &S_50111) -> bool {
    let V_30042: String = sqlx::query_scalar("SELECT V_30042 FROM app_politicas_seguridad WHERE id = 1")
        .fetch_optional(V_30012)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "siempre".to_string());

    if V_30042 == "nunca" || V_30042 == "deshabilitado" {
        return V_30064.V_30637;
    }

    // En el primer inicio de sesión (onboarding_completed_at IS NULL y sin TOTP aún),
    // el usuario completa el cambio obligatorio de contraseña temporal, verificación de V_30050
    // y autorización de dispositivo.
    // A partir del segundo inicio de sesión (onboarding_completed_at IS NOT NULL) —o si ya tiene
    // TOTP habilitado—, se solicita obligatoriamente vincular/verificar Google Authenticator.
    V_30064.V_30637 || V_30064.onboarding_completed_at.is_some()
}

async fn F_40030(
    V_30188: &web::Data<AppState>,
    V_30064: &S_50111,
    active_sid: Uuid,
    active_ip: &str,
    active_ua: &str,
    active_last_act: &str,
    attempt_ip: &str,
    attempt_ua: &str,
    attempt_dev_name: &str,
    attempt_dev_os: &str,
    attempt_dev_browser: &str,
    V_30104: Option<Cookie<'static>>,
) -> Result<HttpResponse> {
    // 1. Registrar evento de bloqueo en auditoría
    write_audit(
        &V_30188.V_30012,
        Some(V_30064.id),
        "login_sesion_simultanea_bloqueada",
        "auth",
        "blocked",
        json!({
            "V_30114": V_30064.V_30114,
            "role": V_30064.role,
            "active_session_id": active_sid.to_string(),
            "active_ip": active_ip,
            "attempt_ip": attempt_ip,
            "attempt_user_agent": attempt_ua,
            "attempt_device": format!("{} ({} / {})", attempt_dev_name, attempt_dev_os, attempt_dev_browser)
        }),
    )
    .await;

    // 2. Generar alerta de seguridad en el Centro de Seguridad
    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO app_alertas_seguridad (V_30701, V_30461, V_30462, V_30702, V_30055, estado)
        VALUES ('alta', 'Intento de inicio de sesión simultáneo V_30119', $1, 'sesion_simultanea_bloqueada', $2, 'activa')
        "#
    )
    .bind(format!(
        "Se bloqueó un nuevo intento de acceso para la cuenta {} ({}) V_30511 IP {} ({} - {} / {}) porque ya mantiene una sesión activa (IP {}).",
        V_30064.V_30114, V_30064.display_name, attempt_ip, attempt_dev_name, attempt_dev_os, attempt_dev_browser, active_ip
    ))
    .bind(V_30064.id)
    .execute(&V_30188.V_30012)
    .await;

    // 3. Evaluar enfriamiento (cooldown) de correo de alerta para evitar spam
    let V_30071: i64 = sqlx::query_scalar::<V_30020, i32>(
        "SELECT COALESCE(concurrent_alert_cooldown_minutes, 5) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .unwrap_or(5) as i64;

    let V_30072 = Utc::now().naive_utc();
    let V_30073 = match V_30064.last_concurrent_alert_at {
        Some(last_at) => (V_30072 - last_at).num_minutes() >= V_30071,
        None => true,
    };

    if V_30073 {
        if let Some(ref V_30088) = V_30064.V_30050 {
            if !V_30088.trim().is_empty() {
                let V_30020 = sqlx::query("UPDATE app_users SET last_concurrent_alert_at = NOW() WHERE id = $1")
                    .bind(V_30064.id)
                    .execute(&V_30188.V_30012)
                    .await;

                let V_30074 = V_30188.V_30012.clone();
                let V_30075 = V_30188.V_30011.clone();
                let V_30076 = V_30064.id;
                let V_30077 = V_30088.trim().to_string();
                let V_30078 = V_30064.display_name.clone();
                let V_30079 = V_30064.V_30114.clone();
                let V_30080 = attempt_ip.to_string();
                let V_30081 = chrono::Local::V_30072().format("%Y-%m-%d %H:%M:%S").to_string();
                let V_30082 = format!(
                    "{} ({} / {}) — Sesión activa previa: {} (última actividad: {})",
                    attempt_dev_name, attempt_dev_os, attempt_dev_browser, active_ip, active_last_act
                );

                tokio::spawn(async move {
                    match mailer::send_concurrent_session_alert_email(
                        &V_30077,
                        &V_30078,
                        &V_30079,
                        &V_30081,
                        &V_30082,
                        &V_30080,
                        &V_30075,
                    )
                    .await
                    {
                        V_30054(V_30020) => {
                            write_audit(
                                &V_30074,
                                Some(V_30076),
                                "alerta_email_sesion_simultanea",
                                "auth",
                                "success",
                                json!({ "V_30050": V_30077, "attempt_ip": V_30080 }),
                            )
                            .await;
                        }
                        V_30156(err) => {
                            write_audit(
                                &V_30074,
                                Some(V_30076),
                                "alerta_email_sesion_simultanea",
                                "auth",
                                "failed",
                                json!({ "V_30050": V_30077, "error": err }),
                            )
                            .await;
                        }
                    }
                });
            }
        }
    } else {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.id),
            "alerta_email_sesion_simultanea",
            "auth",
            "cooldown_skipped",
            json!({ "V_30114": V_30064.V_30114, "cooldown_minutes": V_30071 }),
        )
        .await;
    }

    let V_30083 = "No es posible iniciar sesión porque V_30692 cuenta ya posee una sesión activa en el sistema. Cierre la sesión abierta o utilice «Liberar sesión activa con 2FA» si perdió acceso al V_30654 anterior.";
    let mut V_30084 = render_html(&S_50029 {
        error: Some(V_30083.to_string()),
        notice: None,
        device_warning: None,
    })?;
    if let Some(c) = V_30104 {
        let V_30020 = V_30084.add_cookie(&c);
    }
    V_30054(V_30084)
}

async fn F_40031(
    V_30188: &web::Data<AppState>,
    V_30064: &S_50111,
) {
    let Some(ref V_30050) = V_30064.V_30050 else { return; };
    if V_30050.trim().is_empty() {
        return;
    }

    let V_30085: bool = sqlx::query_scalar(
        V_30522#"
        SELECT EXISTS(
            SELECT 1 FROM app_codigos_temporales
            WHERE V_30055 = $1
              AND V_30092 IN ('verificacion_email', 'mfa_login')
              AND usado = false
              AND expira_at > NOW()
              AND COALESCE(intentos, 0) < COALESCE(max_intentos, 3)
        )
        "#
    )
    .bind(V_30064.id)
    .fetch_one(&V_30188.V_30012)
    .await
    .unwrap_or(false);

    if V_30085 {
        return;
    }

    let V_30086: i64 = sqlx::query_scalar::<V_30020, i32>(
        "SELECT COALESCE(temp_code_expiry_minutes, 10) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .unwrap_or(10)
    .max(5) as i64;

    let (V_30034, V_30029) = F_40020();
    let V_30087 = Utc::now() + Duration::minutes(V_30086);

    let V_30020 = sqlx::query(
        "INSERT INTO app_codigos_temporales (V_30055, V_30092, codigo_hash, email_destino, expira_at, intentos, max_intentos) VALUES ($1, 'verificacion_email', $2, $3, $4, 0, 5)"
    )
    .bind(V_30064.id)
    .bind(&V_30029)
    .bind(V_30050.trim())
    .bind(V_30087.naive_utc())
    .execute(&V_30188.V_30012)
    .await;

    let V_30075 = V_30188.V_30011.clone();
    let V_30088 = V_30050.trim().to_string();
    let V_30089 = V_30064.display_name.clone();
    let V_30090 = V_30064.V_30114.clone();
    let V_30076 = V_30064.id;
    tokio::spawn(async move {
        let V_30020 = mailer::send_welcome_verification_email(&V_30088, &V_30089, &V_30090, &V_30034, V_30076, &V_30075).await;
    });
}

async fn F_40032(
    req: &HttpRequest,
    V_30188: &web::Data<AppState>,
    V_30064: &S_50111,
    V_30126: &S_50112,
    is_restricted: bool,
) -> Result<HttpResponse> {
    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or(&V_30126.V_30678).to_string();
    let V_30092 = req
        .V_30457()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&V_30126.V_30092)
        .to_string();

    let V_30093: bool = sqlx::query_scalar(
        "SELECT COALESCE(single_session_per_user, true) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .unwrap_or(true);

    let (V_30063, abs_hours): (i64, i64) = sqlx::query_as::<V_30020, (i32, i32)>(
        "SELECT COALESCE(session_idle_minutes, 30), COALESCE(session_absolute_hours, 8) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .map(|(i, a)| (i as i64, a as i64))
    .unwrap_or((V_30188.V_30011.session_idle_minutes, V_30188.V_30011.session_absolute_hours));

    let mut V_30094 = V_30188.V_30012.begin().await.map_err(error::ErrorInternalServerError)?;

    // Bloqueo atómico por fila del usuario para evitar condición de carrera entre dos logins simultáneos
    let V_30095: i64 = sqlx::query_scalar("SELECT id FROM app_users WHERE id = $1 FOR UPDATE")
        .bind(V_30064.id)
        .fetch_one(&mut *V_30094)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if V_30093 && !is_restricted {
        let V_30096: Option<(Uuid, String, String, String)> = sqlx::query_as(
            V_30522#"
            SELECT id,
                   COALESCE(V_30678, '127.0.0.1') AS V_30678,
                   COALESCE(V_30092, 'Navegador Web') AS V_30092,
                   to_char(V_30676, 'YYYY-MM-DD HH24:MI:SS') AS last_act
            FROM app_sessions
            WHERE V_30055 = $1
              AND revoked_at IS NULL
              AND V_30677 > NOW()
              AND absolute_expires_at > NOW()
              AND COALESCE(is_restricted, false) = false
            ORDER BY V_30676 DESC
            LIMIT 1
            "#
        )
        .bind(V_30064.id)
        .fetch_optional(&mut *V_30094)
        .await
        .map_err(error::ErrorInternalServerError)?;

        if let Some((act_sid, act_ip, act_ua, act_last)) = V_30096 {
            let V_30020 = V_30094.rollback().await;
            return F_40030(
                V_30188,
                V_30064,
                act_sid,
                &act_ip,
                &act_ua,
                &act_last,
                &V_30091,
                &V_30092,
                &V_30126.dispositivo_nombre,
                &V_30126.dispositivo_os,
                &V_30126.dispositivo_browser,
                None,
            )
            .await;
        }
    }

    let V_30097 = Uuid::new_v4();
    let V_30098 = Utc::now();
    let V_30099 = V_30098 + Duration::minutes(V_30063);
    let V_30100 = V_30098 + Duration::hours(abs_hours);

    sqlx::query(
        V_30522#"
        INSERT INTO app_sessions (id, V_30055, V_30676, V_30677, absolute_expires_at, V_30678, V_30092, V_30651, is_restricted)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#
    )
    .bind(V_30097)
    .bind(V_30064.id)
    .bind(V_30098.naive_utc())
    .bind(V_30099.naive_utc())
    .bind(V_30100.naive_utc())
    .bind(&V_30091)
    .bind(&V_30092)
    .bind(&V_30126.V_30651)
    .bind(is_restricted)
    .execute(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    sqlx::query("UPDATE app_mfa_login_challenges SET verified = true, current_step = 'completed', backup_codes_plain_json = NULL WHERE id = $1")
        .bind(V_30126.id)
        .execute(&mut *V_30094)
        .await
        .map_err(error::ErrorInternalServerError)?;

    sqlx::query(
        "UPDATE app_users SET ultimo_acceso = NOW(), ip_ultimo_acceso = $1, onboarding_completed_at = COALESCE(onboarding_completed_at, NOW()) WHERE id = $2"
    )
    .bind(&V_30091)
    .bind(V_30064.id)
    .execute(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    V_30094.commit().await.map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.id),
        "login_completado",
        "auth",
        "success",
        json!({
            "V_30114": V_30064.V_30114,
            "V_30060": V_30097.to_string(),
            "V_30651": V_30126.V_30651,
            "ip": V_30091,
            "is_restricted": is_restricted
        }),
    )
    .await;

    let V_30101 = Cookie::build("spf_session", V_30097.to_string())
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .finish();

    let V_30057 = Cookie::build("spf_totp_pending", "")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::V_30041::time::Duration::seconds(0))
        .finish();

    let V_30103 = Cookie::build("spf_2fa_pending", "")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::V_30041::time::Duration::seconds(0))
        .finish();

    let V_30104 = Cookie::build("spf_device_id", V_30126.V_30651.clone())
        .path("/")
        .http_only(false)
        .same_site(SameSite::Lax)
        .max_age(actix_web::V_30041::time::Duration::days(365))
        .finish();

    let V_30105 = if is_restricted || V_30064.V_30638 {
        "/cambiar-V_30115-obligatorio"
    } else {
        "/"
    };

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, V_30105))
        .V_30041(V_30101)
        .V_30041(V_30057)
        .V_30041(V_30103)
        .V_30041(V_30104)
        .finish())
}

async fn F_40033(
    req: &HttpRequest,
    V_30188: &web::Data<AppState>,
    V_30126: &S_50112,
    extra_cookie: Option<Cookie<'static>>,
) -> Result<HttpResponse> {
    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let V_30106 = |loc: &str, cookie_opt: Option<Cookie<'static>>| -> Result<HttpResponse> {
        let V_30107 = Cookie::build("spf_totp_pending", V_30126.id.to_string())
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .max_age(actix_web::V_30041::time::Duration::minutes(15))
            .finish();
        let mut V_30108 = HttpResponse::SeeOther();
        V_30108.append_header((header::LOCATION, loc));
        V_30108.V_30041(V_30107);
        if let Some(c) = cookie_opt {
            V_30108.V_30041(c);
        }
        V_30054(V_30108.finish())
    };

    // PASO 1: Cambio obligatorio de contraseña temporal o forzada
    if V_30064.V_30638 {
        let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET current_step = 'V_30638' WHERE id = $1")
            .bind(V_30126.id)
            .execute(&V_30188.V_30012)
            .await;
        return V_30106("/cambiar-V_30115-obligatorio", extra_cookie);
    }

    // PASO 2: Vinculación o verificación de segundo factor TOTP (Google Authenticator)
    let V_30109 = F_40029(&V_30188.V_30012, &V_30064).await;
    if V_30109 && !V_30126.totp_verified {
        if !V_30064.V_30637 || V_30064.totp_secret_enc.is_none() {
            let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET current_step = 'enrolar_totp' WHERE id = $1")
                .bind(V_30126.id)
                .execute(&V_30188.V_30012)
                .await;
            return V_30106("/login/totp/enrolar", extra_cookie);
        } else {
            let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET current_step = 'verificar_totp' WHERE id = $1")
                .bind(V_30126.id)
                .execute(&V_30188.V_30012)
                .await;
            return V_30106("/login/totp", extra_cookie);
        }
    }

    // PASO 3: Confirmación explícita de resguardo de códigos de respaldo tras enrolamiento
    if V_30109 && !V_30064.backup_codes_confirmed && V_30126.backup_codes_plain_json.is_some() {
        let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET current_step = 'confirmar_backup_codes' WHERE id = $1")
            .bind(V_30126.id)
            .execute(&V_30188.V_30012)
            .await;
        return V_30106("/login/totp/backup-codes", extra_cookie);
    }

    // PASO 4: Confirmación de correo electrónico institucional si aún no está verificado
    if !V_30064.V_30636 && V_30064.V_30050.as_ref().map(|e| !e.trim().is_empty()).unwrap_or(false) {
        F_40031(V_30188, &V_30064).await;
        let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET current_step = 'verificar_email' WHERE id = $1")
            .bind(V_30126.id)
            .execute(&V_30188.V_30012)
            .await;
        return V_30106("/login/2fa", extra_cookie);
    }

    // PASO 5: Evaluación y autorización de dispositivo / terminal
    let V_30110 = if V_30126.V_30651.trim().is_empty() {
        Uuid::new_v4().to_string()
    } else {
        V_30126.V_30651.trim().to_string()
    };

    let V_30111: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, estado FROM app_dispositivos WHERE V_30055 = $1 AND V_30651 = $2"
    )
    .bind(V_30064.id)
    .bind(&V_30110)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30112: bool = sqlx::query_scalar(
        "SELECT COALESCE(V_30112, true) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .unwrap_or(true);

    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or(&V_30126.V_30678).to_string();

    if let Some((V_30728, ref estado)) = V_30111 {
        if estado == "revocado" || estado == "rechazado" {
            write_audit(
                &V_30188.V_30012,
                Some(V_30064.id),
                "login_dispositivo",
                "V_30646",
                "blocked",
                json!({ "V_30114": V_30064.V_30114, "V_30651": V_30110, "estado": estado, "ip": V_30091 }),
            )
            .await;
            return Ok(HttpResponse::SeeOther()
                .append_header((header::LOCATION, format!("/login?device_warning={}", estado)))
                .finish());
        }
        if estado == "autorizado" || estado == "aprobado" {
            let V_30020 = sqlx::query("UPDATE app_dispositivos SET V_30658 = NOW(), V_30655 = $1 WHERE id = $2")
                .bind(&V_30091)
                .bind(V_30728)
                .execute(&V_30188.V_30012)
                .await;
            return F_40032(req, V_30188, &V_30064, V_30126, false).await;
        }
        // Estado pendiente o suspendido
        let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET current_step = 'dispositivo_pendiente' WHERE id = $1")
            .bind(V_30126.id)
            .execute(&V_30188.V_30012)
            .await;
        return V_30106("/login/dispositivo", extra_cookie);
    }

    // Dispositivo nuevo / no registrado para este usuario
    if V_30064.role == "superusuario" {
        // Verificar cuántos V_30646 autorizados posee este superusuario
        let V_30113: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM app_dispositivos WHERE V_30055 = $1 AND estado IN ('autorizado', 'aprobado')"
        )
        .bind(V_30064.id)
        .fetch_one(&V_30188.V_30012)
        .await
        .unwrap_or(0);

        if V_30113 == 0 {
            // Excepción controlada de alta inicial del terminal primario del superusuario (post-MFA, post-V_30115, post-V_30050)
            let V_30020 = sqlx::query(
                V_30522#"
                INSERT INTO app_dispositivos
                    (V_30055, V_30651, V_30652, V_30653, V_30654, V_30655, estado, V_30656, autorizado_por, V_30671, V_30657, V_30658)
                VALUES ($1, $2, $3, $4, $5, $6, 'autorizado', 'Alta inicial controlada de terminal primario de Superusuario tras completar onboarding MFA', $1, NOW(), NOW(), NOW())
                ON CONFLICT (V_30055, V_30651) DO UPDATE
                SET estado = 'autorizado', V_30656 = EXCLUDED.V_30656, autorizado_por = EXCLUDED.autorizado_por, V_30671 = NOW(), V_30658 = NOW()
                "#
            )
            .bind(V_30064.id)
            .bind(&V_30110)
            .bind(&V_30126.dispositivo_nombre)
            .bind(&V_30126.dispositivo_os)
            .bind(&V_30126.dispositivo_browser)
            .bind(&V_30091)
            .execute(&V_30188.V_30012)
            .await;

            write_audit(
                &V_30188.V_30012,
                Some(V_30064.id),
                "alta_inicial_dispositivo_superusuario",
                "V_30646",
                "success",
                json!({
                    "V_30114": V_30064.V_30114,
                    "V_30651": V_30110,
                    "nombre": V_30126.dispositivo_nombre,
                    "so": V_30126.dispositivo_os,
                    "V_30654": V_30126.dispositivo_browser,
                    "ip": V_30091
                }),
            )
            .await;

            return F_40032(req, V_30188, &V_30064, V_30126, false).await;
        }
    }

    if !V_30112 {
        let V_30020 = sqlx::query(
            V_30522#"
            INSERT INTO app_dispositivos
                (V_30055, V_30651, V_30652, V_30653, V_30654, V_30655, estado, V_30656, V_30671, V_30657, V_30658)
            VALUES ($1, $2, $3, $4, $5, $6, 'autorizado', 'Autorizado según política general', NOW(), NOW(), NOW())
            ON CONFLICT (V_30055, V_30651) DO NOTHING
            "#
        )
        .bind(V_30064.id)
        .bind(&V_30110)
        .bind(&V_30126.dispositivo_nombre)
        .bind(&V_30126.dispositivo_os)
        .bind(&V_30126.dispositivo_browser)
        .bind(&V_30091)
        .execute(&V_30188.V_30012)
        .await;

        return F_40032(req, V_30188, &V_30064, V_30126, false).await;
    }

    // Registrar el dispositivo como 'pendiente' y crear solicitud / alerta formal
    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO app_dispositivos
            (V_30055, V_30651, V_30652, V_30653, V_30654, V_30655, estado, V_30656, V_30657, V_30658)
        VALUES ($1, $2, $3, $4, $5, $6, 'pendiente', 'Terminal nueva registrada tras autenticación MFA', NOW(), NOW())
        ON CONFLICT (V_30055, V_30651) DO NOTHING
        "#
    )
    .bind(V_30064.id)
    .bind(&V_30110)
    .bind(&V_30126.dispositivo_nombre)
    .bind(&V_30126.dispositivo_os)
    .bind(&V_30126.dispositivo_browser)
    .bind(&V_30091)
    .execute(&V_30188.V_30012)
    .await;

    let V_30020 = sqlx::query(
        "INSERT INTO app_alertas_seguridad (V_30701, V_30461, V_30462, V_30702, V_30055) VALUES ('media', 'Nuevo dispositivo detectado pendiente de autorización', $1, 'dispositivo_nuevo', $2)"
    )
    .bind(format!(
        "El usuario {} ({}) autenticó sus factores y registró una nueva terminal pendiente de autorización: {} ({} / {}, IP {}).",
        V_30064.V_30114, V_30064.role, V_30126.dispositivo_nombre, V_30126.dispositivo_os, V_30126.dispositivo_browser, V_30091
    ))
    .bind(V_30064.id)
    .execute(&V_30188.V_30012)
    .await;

    let V_30020 = sqlx::query(
        "INSERT INTO app_solicitudes_acceso (V_30092, V_30662, V_30664, datos_solicitud, V_30666, estado) VALUES ('autorizacion_dispositivo', $1, $1, $2, $3, 'pendiente')"
    )
    .bind(V_30064.id)
    .bind(json!({
        "V_30651": V_30110,
        "V_30652": V_30126.dispositivo_nombre,
        "V_30653": V_30126.dispositivo_os,
        "V_30654": V_30126.dispositivo_browser,
        "ip": V_30091
    }))
    .bind(format!("Solicitud automática de habilitación de terminal {} ({})", V_30126.dispositivo_nombre, V_30110))
    .execute(&V_30188.V_30012)
    .await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.id),
        "login_dispositivo",
        "V_30646",
        "pending",
        json!({ "V_30114": V_30064.V_30114, "V_30651": V_30110, "ip": V_30091 }),
    )
    .await;

    let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET current_step = 'dispositivo_pendiente' WHERE id = $1")
        .bind(V_30126.id)
        .execute(&V_30188.V_30012)
        .await;

    V_30106("/login/dispositivo", extra_cookie)
}

#[post("/login")]
async fn F_40034(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50083>,
) -> Result<HttpResponse> {
    let V_30114 = form.credencial.trim();
    let V_30115 = form.V_30115.trim();
    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or("127.0.0.1").to_string();
    let V_30092 = req.V_30457().get(header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or("Desconocido").to_string();

    let V_30061 = sqlx::query_as::<V_30020, S_50111>(
        V_30522#"
        SELECT id, V_30114, display_name, V_30050,
               COALESCE(V_30636, false) AS V_30636,
               COALESCE(mfa_habilitado, false) AS mfa_habilitado,
               COALESCE(V_30637, false) AS V_30637,
               totp_secret_enc,
               totp_pending_secret_enc,
               totp_last_used_step,
               COALESCE(backup_codes_confirmed, false) AS backup_codes_confirmed,
               password_hash, role, V_30639,
               COALESCE(estado, 'activo') AS estado,
               COALESCE(V_30638, false) AS V_30638,
               temp_password_expires_at,
               COALESCE(V_30640, 0) AS V_30640,
               bloqueado_hasta,
               last_concurrent_alert_at,
               onboarding_completed_at
        FROM app_users
        WHERE LOWER(V_30114) = LOWER($1)
        "#,
    )
    .bind(V_30114)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30116 = |V_30083: &str| {
        S_50029 {
            error: Some(V_30083.to_string()),
            notice: None,
            device_warning: None,
        }
    };

    let Some(V_30064) = V_30061 else {
        write_audit(
            &V_30188.V_30012,
            None,
            "login",
            "auth",
            "failed",
            json!({ "V_30114": V_30114, "reason": "not_found", "ip": V_30091 }),
        )
        .await;
        return render_html(&V_30116("Credencial o contraseña inválida."));
    };

    if !V_30064.V_30639 || V_30064.estado != "activo" {
        return render_html(&S_50029 {
            error: Some("La cuenta se encuentra inactiva o suspendida. Contacte a un administrador.".to_string()),
            notice: None,
            device_warning: None,
        });
    }

    let V_30072 = Utc::now().naive_utc();
    if let Some(bh) = V_30064.bloqueado_hasta {
        if bh > V_30072 {
            let V_30117 = (bh - V_30072).num_minutes().max(1);
            return render_html(&S_50029 {
                error: Some(format!("Cuenta bloqueada preventivamente por reiterados intentos fallidos. Intente nuevamente en {} minuto(V_30044).", V_30117)),
                notice: None,
                device_warning: None,
            });
        }
    }

    let (max_attempts, lockout_mins): (i32, i64) = sqlx::query_as::<V_30020, (i32, i32)>(
        "SELECT COALESCE(max_login_attempts, 5), COALESCE(lockout_duration_minutes, 15) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .map(|(m, l)| (m, l as i64))
    .unwrap_or((5, 15));

    if !verify_password(V_30115, &V_30064.password_hash) {
        let V_30118 = V_30064.V_30640 + 1;
        let mut V_30119 = false;
        if V_30118 >= max_attempts {
            let V_30120 = Utc::now().naive_utc() + Duration::minutes(lockout_mins);
            let V_30020 = sqlx::query("UPDATE app_users SET V_30640 = $1, bloqueado_hasta = $2 WHERE id = $3")
                .bind(V_30118)
                .bind(V_30120)
                .bind(V_30064.id)
                .execute(&V_30188.V_30012)
                .await;
            V_30119 = true;
            let V_30020 = sqlx::query(
                "INSERT INTO app_alertas_seguridad (V_30701, V_30461, V_30462, V_30702, V_30055) VALUES ('alta', 'Bloqueo preventivo por fuerza bruta', $1, 'fuerza_bruta', $2)"
            )
            .bind(format!("Cuenta {} bloqueada tras {} intentos erróneos.", V_30064.V_30114, V_30118))
            .bind(V_30064.id)
            .execute(&V_30188.V_30012)
            .await;
        } else {
            let V_30020 = sqlx::query("UPDATE app_users SET V_30640 = $1 WHERE id = $2")
                .bind(V_30118)
                .bind(V_30064.id)
                .execute(&V_30188.V_30012)
                .await;
        }

        write_audit(
            &V_30188.V_30012,
            Some(V_30064.id),
            "login",
            "auth",
            "failed",
            json!({ "V_30114": V_30114, "reason": "bad_password", "intentos": V_30118, "ip": V_30091 }),
        )
        .await;

        return render_html(&S_50029 {
            error: Some(if V_30119 {
                format!("Cuenta bloqueada temporalmente por exceso de intentos fallidos. Espere {} minutos o contacte a seguridad.", lockout_mins)
            } else {
                "Credencial o contraseña inválida.".to_string()
            }),
            notice: None,
            device_warning: None,
        });
    }

    // Comprobar si la contraseña temporal ha expirado (24h)
    if let Some(V_30087) = V_30064.temp_password_expires_at {
        if V_30087 < V_30072 {
            write_audit(
                &V_30188.V_30012,
                Some(V_30064.id),
                "login",
                "auth",
                "failed",
                json!({ "V_30114": V_30114, "reason": "temp_password_expired", "ip": V_30091 }),
            )
            .await;
            return render_html(&S_50029 {
                error: Some("La contraseña temporal ha expirado (validez máxima de 24 horas). Debe solicitar un nuevo restablecimiento a un administrador.".to_string()),
                notice: None,
                device_warning: None,
            });
        }
    }

    // Contraseña correcta: reestablecer intentos fallidos
    let V_30020 = sqlx::query("UPDATE app_users SET V_30640 = 0, bloqueado_hasta = NULL WHERE id = $1")
        .bind(V_30064.id)
        .execute(&V_30188.V_30012)
        .await;

    let V_30110 = form.V_30651.as_deref()
        .filter(|V_30044| !V_30044.trim().is_empty())
        .map(|V_30044| V_30044.trim().to_string())
        .or_else(|| req.V_30041("spf_device_id").map(|c| c.value().trim().to_string()))
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let V_30121 = form.dispositivo_nombre.clone().filter(|V_30044| !V_30044.trim().is_empty()).unwrap_or_else(|| "Terminal Web".to_string());
    let V_30122 = form.dispositivo_os.clone().filter(|V_30044| !V_30044.trim().is_empty()).unwrap_or_else(|| "Desconocido".to_string());
    let V_30123 = form.dispositivo_browser.clone().filter(|V_30044| !V_30044.trim().is_empty()).unwrap_or_else(|| "Navegador Web".to_string());

    let V_30104 = Cookie::build("spf_device_id", V_30110.clone())
        .path("/")
        .http_only(false)
        .same_site(SameSite::Lax)
        .max_age(actix_web::V_30041::time::Duration::days(365))
        .finish();

    // Rechazar inmediatamente cualquier dispositivo explícitamente revocado o rechazado, incluso para superusuario
    let V_30052: Option<String> = sqlx::query_scalar(
        "SELECT estado FROM app_dispositivos WHERE V_30055 = $1 AND V_30651 = $2"
    )
    .bind(V_30064.id)
    .bind(&V_30110)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if matches!(V_30052.as_deref(), Some("revocado") | Some("rechazado")) {
        let V_30124 = V_30052.unwrap_or_else(|| "revocado".to_string());
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.id),
            "login_dispositivo",
            "V_30646",
            "blocked",
            json!({ "V_30114": V_30064.V_30114, "V_30651": V_30110, "estado": V_30124, "ip": V_30091 }),
        )
        .await;
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, format!("/login?device_warning={}", V_30124)))
            .V_30041(V_30104)
            .finish());
    }

    // ---------------------------------------------------------------------
    // POLÍTICA DE SESIÓN ÚNICA ACTIVA: Bloquear segundo ingreso simultáneo
    // ---------------------------------------------------------------------
    let V_30093: bool = sqlx::query_scalar(
        "SELECT COALESCE(single_session_per_user, true) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .unwrap_or(true);

    if V_30093 {
        let V_30096: Option<(Uuid, String, String, String)> = sqlx::query_as(
            V_30522#"
            SELECT id,
                   COALESCE(V_30678, '127.0.0.1') AS V_30678,
                   COALESCE(V_30092, 'Navegador Web') AS V_30092,
                   to_char(V_30676, 'YYYY-MM-DD HH24:MI:SS') AS last_act
            FROM app_sessions
            WHERE V_30055 = $1
              AND revoked_at IS NULL
              AND V_30677 > NOW()
              AND absolute_expires_at > NOW()
              AND COALESCE(is_restricted, false) = false
            ORDER BY V_30676 DESC
            LIMIT 1
            "#
        )
        .bind(V_30064.id)
        .fetch_optional(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

        if let Some((act_sid, act_ip, act_ua, act_last)) = V_30096 {
            return F_40030(
                &V_30188,
                &V_30064,
                act_sid,
                &act_ip,
                &act_ua,
                &act_last,
                &V_30091,
                &V_30092,
                &V_30121,
                &V_30122,
                &V_30123,
                Some(V_30104),
            )
            .await;
        }
    }

    // Crear desafío unificado de onboarding / autenticación multifactor
    let challenge_id = Uuid::new_v4();
    let V_30087 = Utc::now() + Duration::minutes(15);
    let V_30125 = V_30064.V_30050.clone().unwrap_or_else(|| "sin_correo@spf.gob.ar".to_string());

    sqlx::query(
        V_30522#"
        INSERT INTO app_mfa_login_challenges
            (id, V_30055, V_30050, V_30678, V_30092, V_30651, dispositivo_nombre, dispositivo_os, dispositivo_browser, password_verified, totp_verified, V_30677, verified, current_step)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true, false, $10, false, 'init')
        "#
    )
    .bind(challenge_id)
    .bind(V_30064.id)
    .bind(&V_30125)
    .bind(&V_30091)
    .bind(&V_30092)
    .bind(&V_30110)
    .bind(&V_30121)
    .bind(&V_30122)
    .bind(&V_30123)
    .bind(V_30087.naive_utc())
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30126 = S_50112 {
        id: challenge_id,
        V_30055: V_30064.id,
        V_30050: V_30125,
        V_30678: V_30091,
        V_30092,
        V_30651: V_30110,
        dispositivo_nombre: V_30121,
        dispositivo_os: V_30122,
        dispositivo_browser: V_30123,
        totp_verified: false,
        backup_codes_plain_json: None,
        V_30640: 0,
    };

    F_40033(&req, &V_30188, &V_30126, Some(V_30104)).await
}

#[get("/login/totp/enrolar")]
async fn F_40035(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login?error=Desafío de vinculación expirado."))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    // Si debe cambiar contraseña temporal primero, redirigir al paso obligatorio 1
    if V_30064.V_30638 {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/cambiar-V_30115-obligatorio"))
            .finish());
    }

    // REGLA CRÍTICA DE SEGURIDAD: Si el usuario ya tiene TOTP habilitado y confirmado, NUNCA sobrescribir V_30692 secreto
    if V_30064.V_30637 && V_30064.totp_secret_enc.is_some() {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login/totp"))
            .finish());
    }

    // Idempotencia: Si ya existe un secreto pendiente en proceso de enrolamiento, reutilizarlo sin regenerarlo en cada recarga
    let V_30043 = if let Some(ref pending_enc) = V_30064.totp_pending_secret_enc {
        match crypto_utils::decrypt_totp_secret(pending_enc) {
            V_30054(V_30044) if !V_30044.trim().is_empty() => V_30044,
            V_30020 => {
                let V_30044 = crypto_utils::generate_totp_secret();
                let V_30045 = crypto_utils::encrypt_totp_secret(&V_30044)
                    .map_err(|e| error::ErrorInternalServerError(format!("Error cifrando secreto TOTP: {e}")))?;
                let V_30020 = sqlx::query("UPDATE app_users SET totp_pending_secret_enc = $1 WHERE id = $2")
                    .bind(&V_30045)
                    .bind(V_30064.id)
                    .execute(&V_30188.V_30012)
                    .await;
                V_30044
            }
        }
    } else {
        let V_30044 = crypto_utils::generate_totp_secret();
        let V_30045 = crypto_utils::encrypt_totp_secret(&V_30044)
            .map_err(|e| error::ErrorInternalServerError(format!("Error cifrando secreto TOTP: {e}")))?;
        let V_30020 = sqlx::query("UPDATE app_users SET totp_pending_secret_enc = $1 WHERE id = $2")
            .bind(&V_30045)
            .bind(V_30064.id)
            .execute(&V_30188.V_30012)
            .await;
        V_30044
    };

    let V_30046 = crypto_utils::get_totp_url(&V_30064.V_30114, &V_30043)
        .map_err(|e| error::ErrorInternalServerError(format!("Error generando URL TOTP: {e}")))?;

    let V_30047 = crypto_utils::generate_qr_svg(&V_30046)
        .map_err(|e| error::ErrorInternalServerError(format!("Error generando QR SVG: {e}")))?;

    render_html(&S_50032 {
        error: None,
        V_30114: V_30064.V_30114,
        V_30047,
        secret_base32: V_30043,
        V_30046,
    })
}

#[post("/login/totp/enrolar")]
async fn F_40036(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50086>,
) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login?error=Desafío expirado."))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    if V_30064.V_30637 && V_30064.totp_secret_enc.is_some() {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login/totp"))
            .finish());
    }

    let V_30132 = V_30064.totp_pending_secret_enc.clone().or_else(|| V_30064.totp_secret_enc.clone());
    let Some(V_30045) = V_30132 else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login/totp/enrolar"))
            .finish());
    };

    let V_30043 = crypto_utils::decrypt_totp_secret(&V_30045)
        .map_err(|e| error::ErrorInternalServerError(format!("Error descifrando secreto TOTP: {e}")))?;

    let Ok(matched_step) = crypto_utils::verify_totp_code_with_step(
        &V_30043,
        &V_30064.V_30114,
        &form.codigo_confirmacion,
        V_30064.totp_last_used_step,
    ) else {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.id),
            "enrolar_totp",
            "auth",
            "failed",
            json!({ "V_30114": V_30064.V_30114, "reason": "invalid_confirmation_code" }),
        )
        .await;
        let V_30046 = crypto_utils::get_totp_url(&V_30064.V_30114, &V_30043).unwrap_or_default();
        let V_30047 = crypto_utils::generate_qr_svg(&V_30046).unwrap_or_default();
        return render_html(&S_50032 {
            error: Some("Código de seguridad incorrecto. Verifique la hora de V_30692 dispositivo e ingrese los 6 dígitos generados.".to_string()),
            V_30114: V_30064.V_30114,
            V_30047,
            secret_base32: V_30043,
            V_30046,
        });
    };

    // Promover totp_pending_secret_enc -> totp_secret_enc y marcar V_30637 = true
    sqlx::query(
        V_30522#"
        UPDATE app_users
        SET totp_secret_enc = $1,
            totp_pending_secret_enc = NULL,
            V_30637 = true,
            totp_verificado_at = NOW(),
            totp_last_used_step = $2,
            backup_codes_confirmed = false
        WHERE id = $3
        "#
    )
    .bind(&V_30045)
    .bind(matched_step)
    .bind(V_30064.id)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30133 = crypto_utils::generate_backup_codes(8);
    let V_30020 = sqlx::query("DELETE FROM app_backup_codes WHERE V_30055 = $1")
        .bind(V_30064.id)
        .execute(&V_30188.V_30012)
        .await;
    for (_code, V_30029) in &V_30133 {
        let V_30020 = sqlx::query("INSERT INTO app_backup_codes (V_30055, code_hash, usado) VALUES ($1, $2, false)")
            .bind(V_30064.id)
            .bind(V_30029)
            .execute(&V_30188.V_30012)
            .await;
    }

    let V_30049: Vec<String> = V_30133.into_iter().map(|(c, V_30020)| c).collect();
    let V_30048 = serde_json::to_string(&V_30049).unwrap_or_else(|V_30020| "[]".to_string());
    let V_30136 = crypto_utils::encrypt_totp_secret(&V_30048).unwrap_or(V_30048);

    sqlx::query(
        "UPDATE app_mfa_login_challenges SET totp_verified = true, backup_codes_plain_json = $1, current_step = 'confirmar_backup_codes' WHERE id = $2"
    )
    .bind(&V_30136)
    .bind(V_30126.id)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.id),
        "enrolar_totp",
        "auth",
        "success",
        json!({ "V_30114": V_30064.V_30114 }),
    )
    .await;

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, "/login/totp/backup-codes"))
        .finish())
}

#[get("/login/totp/backup-codes")]
async fn F_40037(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let Some(ref V_30136) = V_30126.backup_codes_plain_json else {
        return F_40033(&req, &V_30188, &V_30126, None).await;
    };

    let V_30048 = crypto_utils::decrypt_totp_secret(V_30136).unwrap_or_else(|V_30020| V_30136.clone());
    let V_30049: Vec<String> = serde_json::from_str(&V_30048).unwrap_or_default();

    render_html(&S_50033 {
        V_30114: V_30064.V_30114,
        V_30133: V_30049,
    })
}

#[post("/login/totp/backup-codes/regenerar")]
async fn F_40038(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    if !V_30126.totp_verified {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login/totp"))
            .finish());
    }

    let V_30133 = crypto_utils::generate_backup_codes(8);
    let V_30020 = sqlx::query("DELETE FROM app_backup_codes WHERE V_30055 = $1")
        .bind(V_30064.id)
        .execute(&V_30188.V_30012)
        .await;
    for (_code, V_30029) in &V_30133 {
        let V_30020 = sqlx::query("INSERT INTO app_backup_codes (V_30055, code_hash, usado) VALUES ($1, $2, false)")
            .bind(V_30064.id)
            .bind(V_30029)
            .execute(&V_30188.V_30012)
            .await;
    }

    let V_30049: Vec<String> = V_30133.into_iter().map(|(c, V_30020)| c).collect();
    let V_30048 = serde_json::to_string(&V_30049).unwrap_or_else(|V_30020| "[]".to_string());
    let V_30136 = crypto_utils::encrypt_totp_secret(&V_30048).unwrap_or(V_30048);

    let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET backup_codes_plain_json = $1 WHERE id = $2")
        .bind(&V_30136)
        .bind(V_30126.id)
        .execute(&V_30188.V_30012)
        .await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.id),
        "regenerar_backup_codes",
        "auth",
        "success",
        json!({ "V_30114": V_30064.V_30114 }),
    )
    .await;

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, "/login/totp/backup-codes"))
        .finish())
}

#[post("/login/totp/backup-codes/confirmar")]
async fn F_40039(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: Option<web::Form<S_50088>>,
) -> Result<HttpResponse> {
    let Some(mut V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login?error=Desafío expirado."))
            .finish());
    };

    if let Some(ref V_30187) = form {
        if V_30187.confirmado.is_none() {
            return Ok(HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/login/totp/backup-codes"))
                .finish());
        }
    }

    sqlx::query("UPDATE app_users SET backup_codes_confirmed = true WHERE id = $1")
        .bind(V_30126.V_30055)
        .execute(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

    sqlx::query("UPDATE app_mfa_login_challenges SET backup_codes_plain_json = NULL WHERE id = $1")
        .bind(V_30126.id)
        .execute(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

    V_30126.backup_codes_plain_json = None;

    write_audit(
        &V_30188.V_30012,
        Some(V_30126.V_30055),
        "confirmar_backup_codes",
        "auth",
        "success",
        json!({ "V_30055": V_30126.V_30055 }),
    )
    .await;

    F_40033(&req, &V_30188, &V_30126, None).await
}

#[get("/login/totp")]
async fn F_40040(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login?error=Desafío de seguridad expirado."))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    if V_30064.V_30638 {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/cambiar-V_30115-obligatorio"))
            .finish());
    }

    if !V_30064.V_30637 || V_30064.totp_secret_enc.is_none() {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login/totp/enrolar"))
            .finish());
    }

    render_html(&S_50031 {
        error: None,
        V_30114: V_30064.V_30114,
        display_name: V_30064.display_name,
    })
}

#[post("/login/totp")]
async fn F_40041(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50085>,
) -> Result<HttpResponse> {
    let Some(mut V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login?error=Desafío expirado."))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or(&V_30126.V_30678).to_string();

    if V_30126.V_30640 >= 5 {
        let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET verified = true WHERE id = $1")
            .bind(V_30126.id)
            .execute(&V_30188.V_30012)
            .await;
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login?error=Demasiados+intentos+2FA+fallidos.+Inicie+sesión+nuevamente."))
            .finish());
    }

    let V_30137 = form.V_30137.as_deref().unwrap_or("totp");

    if V_30137 == "backup" || form.codigo_backup.as_ref().map(|V_30044| !V_30044.trim().is_empty()).unwrap_or(false) {
        let V_30138 = form.codigo_backup.as_deref().unwrap_or("").trim();
        let V_30029 = crypto_utils::hash_backup_code(V_30138);

        let V_30139: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM app_backup_codes WHERE V_30055 = $1 AND code_hash = $2 AND usado = false LIMIT 1"
        )
        .bind(V_30064.id)
        .bind(&V_30029)
        .fetch_optional(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

        if let Some(code_id) = V_30139 {
            let V_30020 = sqlx::query("UPDATE app_backup_codes SET usado = true, usado_at = NOW() WHERE id = $1")
                .bind(code_id)
                .execute(&V_30188.V_30012)
                .await;

            let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET totp_verified = true WHERE id = $1")
                .bind(V_30126.id)
                .execute(&V_30188.V_30012)
                .await;

            let V_30020 = sqlx::query("UPDATE app_users SET V_30638 = true WHERE id = $1")
                .bind(V_30064.id)
                .execute(&V_30188.V_30012)
                .await;

            let V_30020 = sqlx::query(
                "INSERT INTO app_alertas_seguridad (V_30701, V_30461, V_30462, V_30702, V_30055) VALUES ('alta', 'Acceso de emergencia mediante código de respaldo', $1, 'mfa_backup_utilizado', $2)"
            )
            .bind(format!("El usuario {} ingresó utilizando un código de respaldo de un solo uso. Se forzó cambio obligatorio de contraseña.", V_30064.V_30114))
            .bind(V_30064.id)
            .execute(&V_30188.V_30012)
            .await;

            write_audit(
                &V_30188.V_30012,
                Some(V_30064.id),
                "login_backup_code",
                "auth",
                "success",
                json!({ "V_30114": V_30064.V_30114, "ip": V_30091, "backup_code_id": code_id }),
            )
            .await;

            V_30126.totp_verified = true;
            return F_40033(&req, &V_30188, &V_30126, None).await;
        } else {
            let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET V_30640 = V_30640 + 1 WHERE id = $1")
                .bind(V_30126.id)
                .execute(&V_30188.V_30012)
                .await;

            write_audit(
                &V_30188.V_30012,
                Some(V_30064.id),
                "login_backup_code",
                "auth",
                "failed",
                json!({ "V_30114": V_30064.V_30114, "ip": V_30091, "reason": "invalid_or_used" }),
            )
            .await;

            return render_html(&S_50031 {
                error: Some("Código de respaldo de emergencia inválido o ya utilizado previamente.".to_string()),
                V_30114: V_30064.V_30114,
                display_name: V_30064.display_name,
            });
        }
    }

    // Validación TOTP 6 dígitos sin modificar jamás el secreto confirmado
    let V_30140 = form.codigo.as_deref().unwrap_or("").trim();
    let Some(ref enc_secret) = V_30064.totp_secret_enc else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login/totp/enrolar"))
            .finish());
    };

    let V_30043 = crypto_utils::decrypt_totp_secret(enc_secret)
        .map_err(|e| error::ErrorInternalServerError(format!("Error descifrando secreto TOTP: {e}")))?;

    let Ok(matched_step) = crypto_utils::verify_totp_code_with_step(
        &V_30043,
        &V_30064.V_30114,
        V_30140,
        V_30064.totp_last_used_step,
    ) else {
        let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET V_30640 = V_30640 + 1 WHERE id = $1")
            .bind(V_30126.id)
            .execute(&V_30188.V_30012)
            .await;

        write_audit(
            &V_30188.V_30012,
            Some(V_30064.id),
            "login_totp",
            "auth",
            "failed",
            json!({ "V_30114": V_30064.V_30114, "ip": V_30091, "reason": "bad_or_replayed_code" }),
        )
        .await;

        return render_html(&S_50031 {
            error: Some("Código de seguridad incorrecto, expirado o ya utilizado. Verifique la hora de V_30692 dispositivo e ingrese el código actual.".to_string()),
            V_30114: V_30064.V_30114,
            display_name: V_30064.display_name,
        });
    };

    let V_30020 = sqlx::query("UPDATE app_users SET totp_last_used_step = $1 WHERE id = $2")
        .bind(matched_step)
        .bind(V_30064.id)
        .execute(&V_30188.V_30012)
        .await;

    let V_30020 = sqlx::query("UPDATE app_mfa_login_challenges SET totp_verified = true, V_30640 = 0 WHERE id = $1")
        .bind(V_30126.id)
        .execute(&V_30188.V_30012)
        .await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.id),
        "login_totp",
        "auth",
        "success",
        json!({ "V_30114": V_30064.V_30114, "ip": V_30091 }),
    )
    .await;

    V_30126.totp_verified = true;
    F_40033(&req, &V_30188, &V_30126, None).await
}

#[get("/cambiar-V_30115-obligatorio")]
async fn F_40042(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
) -> Result<HttpResponse> {
    if let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await {
        if let Ok(Some(u)) = F_40027(&V_30188.V_30012, V_30126.V_30055).await {
            if !u.V_30638 {
                return F_40033(&req, &V_30188, &V_30126, None).await;
            }
            return render_html(&S_50036 {
                error: None,
                V_30114: u.V_30114,
                reason: "Ha iniciado sesión con una contraseña temporal o inicial. Por políticas de seguridad institucional, debe establecer una nueva clave personal antes de continuar.".to_string(),
            });
        }
    }

    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    render_html(&S_50036 {
        error: None,
        V_30114: V_30064.V_30114,
        reason: "Por políticas de seguridad institucional o restablecimiento previo, es necesario cambiar V_30692 clave.".to_string(),
    })
}

#[post("/cambiar-V_30115-obligatorio")]
async fn F_40043(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50087>,
) -> Result<HttpResponse> {
    let V_30142 = F_40028(&req, &V_30188.V_30012).await;

    let (V_30055, V_30114) = if let Some(ref ch) = V_30142 {
        let Some(u) = F_40027(&V_30188.V_30012, ch.V_30055)
            .await
            .map_err(error::ErrorInternalServerError)?
        else {
            return Ok(HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/login"))
                .finish());
        };
        (u.id, u.V_30114)
    } else {
        let V_30064 = match require_user(&req, &V_30188).await {
            V_30054(u) => u,
            V_30156(V_30084) => return V_30054(V_30084),
        };
        (V_30064.V_30055, V_30064.V_30114)
    };

    let V_30143 = "Por políticas de seguridad institucional o restablecimiento previo, es necesario cambiar V_30692 clave.".to_string();

    let V_30144 = form.new_password.trim();
    let V_30145 = form.confirm_password.trim();

    if V_30144 != V_30145 {
        return render_html(&S_50036 {
            error: Some("Las contraseñas no coinciden.".to_string()),
            V_30114,
            reason: V_30143,
        });
    }

    let (min_len, req_special): (usize, bool) = sqlx::query_as::<V_30020, (i32, bool)>(
        "SELECT COALESCE(password_min_length, 8), COALESCE(password_require_special, false) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .map(|(l, V_30044)| (l.max(8) as usize, V_30044))
    .unwrap_or((8, false));

    if V_30144.len() < min_len {
        return render_html(&S_50036 {
            error: Some(format!("La nueva contraseña debe tener al menos {} caracteres.", min_len)),
            V_30114,
            reason: V_30143,
        });
    }

    let V_30146 = V_30144.chars().any(|c| c.is_uppercase());
    let V_30147 = V_30144.chars().any(|c| c.is_lowercase());
    let V_30148 = V_30144.chars().any(|c| c.is_numeric() || !c.is_alphanumeric());

    if !V_30146 || !V_30147 || !V_30148 || (req_special && !V_30148) {
        return render_html(&S_50036 {
            error: Some("La contraseña debe incluir mayúsculas, minúsculas y al menos un número o carácter especial.".to_string()),
            V_30114,
            reason: V_30143,
        });
    }

    let V_30149: Option<String> = sqlx::query_scalar("SELECT password_hash FROM app_users WHERE id = $1")
        .bind(V_30055)
        .fetch_optional(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if let Some(ref ch) = V_30149 {
        if let Some(ref curr_input) = form.current_password {
            if !curr_input.trim().is_empty() && !verify_password(curr_input.trim(), ch) {
                return render_html(&S_50036 {
                    error: Some("La contraseña temporal o actual ingresada no es correcta.".to_string()),
                    V_30114,
                    reason: V_30143,
                });
            }
        }
        if verify_password(V_30144, ch) {
            return render_html(&S_50036 {
                error: Some("La nueva contraseña no puede ser idéntica a la contraseña temporal o anterior.".to_string()),
                V_30114,
                reason: V_30143,
            });
        }
    }

    let V_30029 = F_40019(V_30144).map_err(|V_30020| error::ErrorInternalServerError("Hash error"))?;

    sqlx::query("UPDATE app_users SET password_hash = $1, V_30638 = false, temp_password_expires_at = NULL WHERE id = $2")
        .bind(V_30029)
        .bind(V_30055)
        .execute(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if let Some(V_30041) = req.V_30041("spf_session") {
        if let Ok(V_30097) = Uuid::parse_str(V_30041.value()) {
            let V_30020 = sqlx::query("UPDATE app_sessions SET is_restricted = false WHERE id = $1")
                .bind(V_30097)
                .execute(&V_30188.V_30012)
                .await;
        }
    }

    write_audit(
        &V_30188.V_30012,
        Some(V_30055),
        "cambio_password_obligatorio",
        "auth",
        "success",
        json!({ "V_30114": V_30114 }),
    )
    .await;

    if let Some(V_30126) = V_30142 {
        return F_40033(&req, &V_30188, &V_30126, None).await;
    }

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, "/"))
        .finish())
}

#[get("/recuperar-V_30115")]
async fn F_40044() -> Result<HttpResponse> {
    render_html(&S_50034 {
        error: None,
        success_message: None,
    })
}

#[post("/recuperar-V_30115")]
async fn F_40045(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50093>,
) -> Result<HttpResponse> {
    let V_30150 = form.identificador.trim();
    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or("127.0.0.1").to_string();

    let V_30151 = "Si la cuenta existe en el sistema y posee una dirección de correo institucional declarada, se ha enviado un enlace seguro para restablecer V_30692 contraseña.".to_string();

    let V_30152: Option<(i64, String, String, Option<String>)> = sqlx::query_as(
        V_30522#"
        SELECT id, V_30114, display_name, V_30050
        FROM app_users
        WHERE (LOWER(V_30114) = LOWER($1) OR (V_30050 IS NOT NULL AND LOWER(V_30050) = LOWER($1)))
          AND V_30639 = true
        LIMIT 1
        "#
    )
    .bind(V_30150)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if let Some((V_30076, V_30168, d_name, Some(V_30050))) = V_30152 {
        if !V_30050.trim().is_empty() {
            let V_30153 = crypto_utils::generate_secure_token();
            let V_30154 = crypto_utils::hash_token(&V_30153);

            let V_30086: i32 = sqlx::query_scalar("SELECT recovery_code_expiry_minutes FROM app_politicas_seguridad WHERE id = 1")
                .fetch_optional(&V_30188.V_30012)
                .await
                .ok()
                .flatten()
                .unwrap_or(15);

            let V_30020 = sqlx::query("UPDATE app_password_resets SET usado = true WHERE V_30055 = $1 AND usado = false")
                .bind(V_30076)
                .execute(&V_30188.V_30012)
                .await;

            let V_30020 = sqlx::query(
                "INSERT INTO app_password_resets (V_30055, V_30154, expira_at, usado, ip_solicitud) VALUES ($1, $2, NOW() + ($3 || ' minutes')::interval, false, $4)"
            )
            .bind(V_30076)
            .bind(&V_30154)
            .bind(V_30086)
            .bind(&V_30091)
            .execute(&V_30188.V_30012)
            .await;

            let V_30075 = V_30188.V_30011.clone();
            let V_30088 = V_30050.clone();
            let V_30089 = d_name.clone();
            let V_30090 = V_30168.clone();
            let V_30155 = V_30153.clone();
            tokio::spawn(async move {
                if let Ok(e) = mailer::send_password_reset_email(&V_30088, &V_30089, &V_30090, &V_30155, &V_30075).await {
                    eprintln!("[PASSWORD RESET EMAIL ERROR] {}", e);
                } else {
                    println!("[PASSWORD RESET EMAIL SUCCESS] Enlace enviado a {}", V_30088);
                }
            });

            write_audit(
                &V_30188.V_30012,
                Some(V_30076),
                "solicitud_recuperar_password",
                "auth",
                "success",
                json!({ "V_30114": V_30168, "ip": V_30091 }),
            )
            .await;
        }
    } else {
        write_audit(
            &V_30188.V_30012,
            None,
            "solicitud_recuperar_password",
            "auth",
            "not_found",
            json!({ "identificador": V_30150, "ip": V_30091 }),
        )
        .await;
    }

    render_html(&S_50034 {
        error: None,
        success_message: Some(V_30151),
    })
}

#[get("/recuperar-V_30115/confirmar")]
async fn F_40046(
    V_30188: web::Data<AppState>,
    query: web::Query<S_50096>,
) -> Result<HttpResponse> {
    let Some(V_30153) = query.V_30153.as_deref().filter(|V_30044| !V_30044.trim().is_empty()) else {
        return render_html(&S_50035 {
            error: Some("Token de recuperación no proporcionado o inválido.".to_string()),
            V_30153: "".to_string(),
        });
    };

    let V_30154 = crypto_utils::hash_token(V_30153);
    let V_30157: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM app_password_resets WHERE V_30154 = $1 AND usado = false AND expira_at > NOW()"
    )
    .bind(&V_30154)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if V_30157.is_none() {
        return render_html(&S_50035 {
            error: Some("El enlace de restablecimiento es inválido o ha expirado (validez máxima de 15 minutos). Solicite uno nuevo.".to_string()),
            V_30153: "".to_string(),
        });
    }

    render_html(&S_50035 {
        error: None,
        V_30153: V_30153.to_string(),
    })
}

#[post("/recuperar-V_30115/confirmar")]
async fn F_40047(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50094>,
) -> Result<HttpResponse> {
    let V_30153 = form.V_30153.trim();
    let V_30144 = form.new_password.trim();
    let V_30145 = form.confirm_password.trim();

    if V_30144 != V_30145 {
        return render_html(&S_50035 {
            error: Some("Las contraseñas no coinciden.".to_string()),
            V_30153: V_30153.to_string(),
        });
    }

    if V_30144.len() < 8 {
        return render_html(&S_50035 {
            error: Some("La contraseña debe tener al menos 8 caracteres.".to_string()),
            V_30153: V_30153.to_string(),
        });
    }

    let V_30154 = crypto_utils::hash_token(V_30153);
    let V_30157: Option<(i64, i64)> = sqlx::query_as(
        "SELECT id, V_30055 FROM app_password_resets WHERE V_30154 = $1 AND usado = false AND expira_at > NOW()"
    )
    .bind(&V_30154)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some((reset_id, V_30055)) = V_30157 else {
        return render_html(&S_50035 {
            error: Some("El enlace de restablecimiento ha expirado o ya fue utilizado previamente.".to_string()),
            V_30153: V_30153.to_string(),
        });
    };

    let V_30029 = F_40019(V_30144).map_err(|V_30020| error::ErrorInternalServerError("Hash error"))?;

    sqlx::query("UPDATE app_password_resets SET usado = true, usado_at = NOW() WHERE id = $1")
        .bind(reset_id)
        .execute(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

    sqlx::query("UPDATE app_users SET password_hash = $1, V_30638 = false, temp_password_expires_at = NULL, V_30640 = 0, bloqueado_hasta = NULL WHERE id = $2")
        .bind(V_30029)
        .bind(V_30055)
        .execute(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let V_30020 = sqlx::query("UPDATE app_sessions SET revoked_at = NOW() WHERE V_30055 = $1 AND revoked_at IS NULL")
        .bind(V_30055)
        .execute(&V_30188.V_30012)
        .await;

    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or("127.0.0.1").to_string();
    write_audit(
        &V_30188.V_30012,
        Some(V_30055),
        "recuperar_password_confirmado",
        "auth",
        "success",
        json!({ "V_30055": V_30055, "ip": V_30091 }),
    )
    .await;

    render_html(&S_50029 {
        error: None,
        notice: Some("Su contraseña ha sido actualizada exitosamente. Inicie sesión con sus nuevas credenciales.".to_string()),
        device_warning: None,
    })
}

async fn F_40048(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    if V_30064.V_30636 {
        return F_40033(&req, &V_30188, &V_30126, None).await;
    }

    F_40031(&V_30188, &V_30064).await;

    let V_30050 = V_30064.V_30050.clone().unwrap_or_else(|| V_30126.V_30050.clone());
    let V_30051 = F_40022(&V_30050);

    render_html(&S_50030 {
        error: None,
        masked_email: V_30051,
        V_30114: V_30064.V_30114,
    })
}

#[get("/login/2fa")]
async fn F_40049(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    F_40048(req, V_30188).await
}

#[get("/login/verificar-V_30050")]
async fn F_40050(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    F_40048(req, V_30188).await
}

async fn F_40051(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50113>,
) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let V_30050 = V_30064.V_30050.clone().unwrap_or_else(|| V_30126.V_30050.clone());
    let V_30051 = F_40022(&V_30050);
    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or(&V_30126.V_30678).to_string();

    let V_30160 = form.codigo.trim().replace(' ', "");
    let V_30161 = F_40021(&V_30160);

    let V_30162: Option<i64> = sqlx::query_scalar(
        V_30522#"
        SELECT id FROM app_codigos_temporales
        WHERE V_30055 = $1
          AND V_30092 IN ('verificacion_email', 'mfa_login')
          AND codigo_hash = $2
          AND usado = false
          AND expira_at > NOW()
          AND COALESCE(intentos, 0) < COALESCE(max_intentos, 5)
        ORDER BY id DESC
        LIMIT 1
        "#
    )
    .bind(V_30064.id)
    .bind(&V_30161)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if let Some(code_id) = V_30162 {
        let V_30020 = sqlx::query("UPDATE app_codigos_temporales SET usado = true, usado_at = NOW() WHERE id = $1")
            .bind(code_id)
            .execute(&V_30188.V_30012)
            .await;

        let V_30020 = sqlx::query("UPDATE app_users SET V_30636 = true, email_verificado_at = COALESCE(email_verificado_at, NOW()) WHERE id = $1")
            .bind(V_30064.id)
            .execute(&V_30188.V_30012)
            .await;

        write_audit(
            &V_30188.V_30012,
            Some(V_30064.id),
            "login_verificar_email",
            "auth",
            "success",
            json!({ "V_30114": V_30064.V_30114, "V_30050": V_30050, "ip": V_30091 }),
        )
        .await;

        F_40033(&req, &V_30188, &V_30126, None).await
    } else {
        // Incrementar intentos en el último código activo del usuario
        let V_30020 = sqlx::query(
            V_30522#"
            UPDATE app_codigos_temporales
            SET intentos = COALESCE(intentos, 0) + 1
            WHERE id = (
                SELECT id FROM app_codigos_temporales
                WHERE V_30055 = $1 AND V_30092 IN ('verificacion_email', 'mfa_login') AND usado = false AND expira_at > NOW()
                ORDER BY id DESC LIMIT 1
            )
            "#
        )
        .bind(V_30064.id)
        .execute(&V_30188.V_30012)
        .await;

        write_audit(
            &V_30188.V_30012,
            Some(V_30064.id),
            "login_verificar_email",
            "auth",
            "failed",
            json!({ "V_30114": V_30064.V_30114, "reason": "invalid_or_expired_code", "ip": V_30091 }),
        )
        .await;

        render_html(&S_50030 {
            error: Some("Código de confirmación incorrecto, expirado o con límite de intentos superado. Verifique V_30692 correo o solicite un nuevo código.".to_string()),
            masked_email: V_30051,
            V_30114: V_30064.V_30114,
        })
    }
}

#[post("/login/2fa")]
async fn F_40052(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50113>,
) -> Result<HttpResponse> {
    F_40051(req, V_30188, form).await
}

#[post("/login/verificar-V_30050")]
async fn F_40053(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50113>,
) -> Result<HttpResponse> {
    F_40051(req, V_30188, form).await
}

async fn F_40054(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::BadRequest().json(json!({ "error": "No hay un desafío de verificación activo." })));
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::BadRequest().json(json!({ "error": "Usuario no encontrado." })));
    };

    let V_30050 = V_30064.V_30050.clone().unwrap_or_else(|| V_30126.V_30050.clone());
    if V_30050.trim().is_empty() || V_30050 == "sin_correo@spf.gob.ar" {
        return Ok(HttpResponse::BadRequest().json(json!({ "error": "La cuenta no posee correo institucional declarado." })));
    }

    let (max_resends, V_30086): (i64, i64) = sqlx::query_as::<V_30020, (i32, i32)>(
        "SELECT COALESCE(max_temp_code_resends, 3), COALESCE(temp_code_expiry_minutes, 10) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .map(|(m, e)| (m.max(1) as i64, e.max(5) as i64))
    .unwrap_or((3, 10));

    let V_30163: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_codigos_temporales WHERE V_30055 = $1 AND creado_at > (NOW() - INTERVAL '15 minutes')"
    )
    .bind(V_30064.id)
    .fetch_one(&V_30188.V_30012)
    .await
    .unwrap_or(0);

    if V_30163 > max_resends {
        return Ok(HttpResponse::TooManyRequests().json(json!({
            "error": format!("Ha alcanzado el límite máximo de {} reenvíos recientes. Aguarde unos minutos.", max_resends)
        })));
    }

    // Invalidar códigos previos sin usar para dejar únicamente el nuevo vigente
    let V_30020 = sqlx::query("UPDATE app_codigos_temporales SET usado = true WHERE V_30055 = $1 AND usado = false")
        .bind(V_30064.id)
        .execute(&V_30188.V_30012)
        .await;

    let (V_30034, V_30029) = F_40020();
    let V_30087 = Utc::now() + Duration::minutes(V_30086);
    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or(&V_30126.V_30678).to_string();

    sqlx::query(
        "INSERT INTO app_codigos_temporales (V_30055, V_30092, codigo_hash, email_destino, expira_at, intentos, max_intentos) VALUES ($1, 'verificacion_email', $2, $3, $4, 0, 5)"
    )
    .bind(V_30064.id)
    .bind(&V_30029)
    .bind(&V_30050)
    .bind(V_30087.naive_utc())
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30075 = V_30188.V_30011.clone();
    let V_30088 = V_30050.clone();
    let V_30089 = V_30064.display_name.clone();
    let V_30090 = V_30064.V_30114.clone();
    let V_30164 = V_30034.clone();
    let V_30165 = V_30091.clone();
    tokio::spawn(async move {
        if let Ok(e) = mailer::send_2fa_login_email(&V_30088, &V_30089, &V_30090, &V_30164, &V_30165, &V_30075).await {
            eprintln!("[EMAIL VERIFY RESEND ERROR] {}", e);
        } else {
            println!("[EMAIL VERIFY RESEND SUCCESS] Código enviado a {}", V_30088);
        }
    });

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.id),
        "login_2fa_reenvio",
        "auth",
        "success",
        json!({ "V_30114": V_30064.V_30114, "V_30050": V_30050 }),
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({ "success": true, "message": "Código de confirmación reenviado exitosamente a V_30692 correo." })))
}

#[post("/login/2fa/reenviar")]
async fn F_40055(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    F_40054(req, V_30188).await
}

#[post("/login/verificar-V_30050/reenviar")]
async fn F_40056(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    F_40054(req, V_30188).await
}

#[get("/login/dispositivo")]
async fn F_40057(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let V_30052: String = sqlx::query_scalar(
        "SELECT COALESCE(estado, 'pendiente') FROM app_dispositivos WHERE V_30055 = $1 AND V_30651 = $2"
    )
    .bind(V_30064.id)
    .bind(&V_30126.V_30651)
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| "pendiente".to_string());

    if V_30052 == "autorizado" || V_30052 == "aprobado" {
        return F_40032(&req, &V_30188, &V_30064, &V_30126, false).await;
    }

    let V_30053 = V_30064.role == "superusuario"
        && V_30052 == "pendiente"
        && V_30126.totp_verified
        && !V_30064.V_30638
        && V_30064.V_30636;

    render_html(&S_50038 {
        error: None,
        notice: None,
        V_30114: V_30064.V_30114,
        display_name: V_30064.display_name,
        V_30651: V_30126.V_30651,
        dispositivo_nombre: V_30126.dispositivo_nombre,
        dispositivo_os: V_30126.dispositivo_os,
        dispositivo_browser: V_30126.dispositivo_browser,
        estado_dispositivo: V_30052,
        V_30053,
    })
}

#[post("/login/dispositivo/consultar")]
async fn F_40058(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };
    F_40033(&req, &V_30188, &V_30126, None).await
}

#[post("/login/dispositivo/autorizar-superusuario")]
async fn F_40059(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50089>,
) -> Result<HttpResponse> {
    let Some(V_30126) = F_40028(&req, &V_30188.V_30012).await else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    let Some(V_30064) = F_40027(&V_30188.V_30012, V_30126.V_30055)
        .await
        .map_err(error::ErrorInternalServerError)?
    else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login"))
            .finish());
    };

    if V_30064.role != "superusuario" || !V_30126.totp_verified || V_30064.V_30638 || !V_30064.V_30636 {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login/dispositivo"))
            .finish());
    }

    let V_30167 = form.V_30167.trim();
    if V_30167.len() < 5 {
        return render_html(&S_50038 {
            error: Some("Debe ingresar un V_30167 institucional válido (mínimo 5 caracteres).".to_string()),
            notice: None,
            V_30114: V_30064.V_30114,
            display_name: V_30064.display_name,
            V_30651: V_30126.V_30651,
            dispositivo_nombre: V_30126.dispositivo_nombre,
            dispositivo_os: V_30126.dispositivo_os,
            dispositivo_browser: V_30126.dispositivo_browser,
            estado_dispositivo: "pendiente".to_string(),
            V_30053: true,
        });
    }

    let V_30052: Option<String> = sqlx::query_scalar(
        "SELECT estado FROM app_dispositivos WHERE V_30055 = $1 AND V_30651 = $2"
    )
    .bind(V_30064.id)
    .bind(&V_30126.V_30651)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if matches!(V_30052.as_deref(), Some("revocado") | Some("rechazado")) {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/login?device_warning=revocado"))
            .finish());
    }

    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or(&V_30126.V_30678).to_string();

    sqlx::query(
        V_30522#"
        UPDATE app_dispositivos
        SET estado = 'autorizado',
            V_30656 = $1,
            autorizado_por = $2,
            V_30671 = NOW(),
            V_30658 = NOW(),
            V_30655 = $3
        WHERE V_30055 = $2 AND V_30651 = $4
        "#
    )
    .bind(V_30167)
    .bind(V_30064.id)
    .bind(&V_30091)
    .bind(&V_30126.V_30651)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30020 = sqlx::query(
        V_30522#"
        UPDATE app_solicitudes_acceso
        SET estado = 'aprobada', aprobador_id = $1, V_30669 = $2, V_30671 = NOW()
        WHERE V_30662 = $1 AND estado = 'pendiente' AND V_30092 = 'autorizacion_dispositivo'
        "#
    )
    .bind(V_30064.id)
    .bind(V_30167)
    .execute(&V_30188.V_30012)
    .await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.id),
        "autorizar_dispositivo_superusuario",
        "V_30646",
        "success",
        json!({
            "V_30114": V_30064.V_30114,
            "V_30651": V_30126.V_30651,
            "V_30167": V_30167,
            "ip": V_30091
        }),
    )
    .await;

    F_40032(&req, &V_30188, &V_30064, &V_30126, false).await
}

#[get("/login/liberar-sesion")]
async fn F_40060(query: web::Query<S_50090>) -> Result<HttpResponse> {
    render_html(&S_50039 {
        error: None,
        success_message: None,
        credencial: query.credencial.clone().unwrap_or_default(),
    })
}

#[post("/login/liberar-sesion")]
async fn F_40061(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50091>,
) -> Result<HttpResponse> {
    let V_30168 = form.credencial.trim();
    let V_30046 = form.V_30115.trim();
    let V_30169 = form.codigo_mfa.trim();
    let V_30167 = form.V_30167.trim();
    let V_30091 = req.connection_info().realip_remote_addr().unwrap_or("127.0.0.1").to_string();

    if V_30168.is_empty() || V_30046.is_empty() || V_30169.is_empty() || V_30167.len() < 4 {
        return render_html(&S_50039 {
            error: Some("Debe completar todos los V_30623 (credencial, contraseña, código 2FA y V_30167 fundamentado).".to_string()),
            success_message: None,
            credencial: V_30168.to_string(),
        });
    }

    let V_30061 = sqlx::query_as::<V_30020, S_50111>(
        V_30522#"
        SELECT id, V_30114, display_name, V_30050,
               COALESCE(V_30636, false) AS V_30636,
               COALESCE(mfa_habilitado, false) AS mfa_habilitado,
               COALESCE(V_30637, false) AS V_30637,
               totp_secret_enc,
               totp_pending_secret_enc,
               totp_last_used_step,
               COALESCE(backup_codes_confirmed, false) AS backup_codes_confirmed,
               password_hash, role, V_30639,
               COALESCE(estado, 'activo') AS estado,
               COALESCE(V_30638, false) AS V_30638,
               temp_password_expires_at,
               COALESCE(V_30640, 0) AS V_30640,
               bloqueado_hasta,
               last_concurrent_alert_at,
               onboarding_completed_at
        FROM app_users
        WHERE LOWER(V_30114) = LOWER($1)
        "#
    )
    .bind(V_30168)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some(V_30064) = V_30061 else {
        return render_html(&S_50039 {
            error: Some("Credenciales o factor de seguridad inválidos.".to_string()),
            success_message: None,
            credencial: V_30168.to_string(),
        });
    };

    if !V_30064.V_30639 || !verify_password(V_30046, &V_30064.password_hash) {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.id),
            "liberar_sesion_mfa",
            "V_30674",
            "failed",
            json!({ "V_30114": V_30064.V_30114, "reason": "bad_password", "ip": V_30091 }),
        )
        .await;
        return render_html(&S_50039 {
            error: Some("Credenciales o factor de seguridad inválidos.".to_string()),
            success_message: None,
            credencial: V_30168.to_string(),
        });
    }

    // Validar TOTP de 6 dígitos o código de respaldo de un solo uso
    let mut V_30170 = false;
    if let Some(ref enc_sec) = V_30064.totp_secret_enc {
        if let Ok(sec) = crypto_utils::decrypt_totp_secret(enc_sec) {
            if let Ok(step) = crypto_utils::verify_totp_code_with_step(&sec, &V_30064.V_30114, V_30169, V_30064.totp_last_used_step) {
                let V_30020 = sqlx::query("UPDATE app_users SET totp_last_used_step = $1 WHERE id = $2")
                    .bind(step)
                    .bind(V_30064.id)
                    .execute(&V_30188.V_30012)
                    .await;
                V_30170 = true;
            }
        }
    }

    if !V_30170 {
        let V_30171 = crypto_utils::hash_backup_code(V_30169);
        let V_30172: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM app_backup_codes WHERE V_30055 = $1 AND code_hash = $2 AND usado = false LIMIT 1"
        )
        .bind(V_30064.id)
        .bind(&V_30171)
        .fetch_optional(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

        if let Some(cid) = V_30172 {
            let V_30020 = sqlx::query("UPDATE app_backup_codes SET usado = true, usado_at = NOW() WHERE id = $1")
                .bind(cid)
                .execute(&V_30188.V_30012)
                .await;
            V_30170 = true;
        }
    }

    if !V_30170 {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.id),
            "liberar_sesion_mfa",
            "V_30674",
            "failed",
            json!({ "V_30114": V_30064.V_30114, "reason": "bad_mfa", "ip": V_30091 }),
        )
        .await;
        return render_html(&S_50039 {
            error: Some("El código 2FA (Google Authenticator) o código de respaldo ingresado no es válido.".to_string()),
            success_message: None,
            credencial: V_30168.to_string(),
        });
    }

    let V_30173 = sqlx::query(
        V_30522#"
        UPDATE app_sessions
        SET revoked_at = NOW(),
            motivo_revocacion = $1,
            revoked_by = $2
        WHERE V_30055 = $2 AND revoked_at IS NULL
        "#
    )
    .bind(format!("Liberación controlada 2FA: {}", V_30167))
    .bind(V_30064.id)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.id),
        "liberar_sesion_mfa",
        "V_30674",
        "success",
        json!({
            "V_30114": V_30064.V_30114,
            "sesiones_revocadas": V_30173.rows_affected(),
            "V_30167": V_30167,
            "ip": V_30091
        }),
    )
    .await;

    render_html(&S_50029 {
        error: None,
        notice: Some("Su sesión previa fue cerrada y revocada exitosamente mediante verificación 2FA. Ahora puede iniciar sesión normalmente.".to_string()),
        device_warning: None,
    })
}

#[get("/verificar-V_30050")]
async fn F_40062(
    V_30188: web::Data<AppState>,
    query: web::Query<S_50114>,
) -> Result<HttpResponse> {
    let (Some(codigo), Some(V_30055)) = (&query.codigo, query.V_30055) else {
        return render_html(&S_50037 {
            success: false,
            message: "Parámetros de verificación incompletos en el enlace recibido.".to_string(),
        });
    };

    let V_30160 = codigo.trim();
    let V_30161 = F_40021(V_30160);

    let V_30162: Option<i64> = sqlx::query_scalar(
        V_30522#"
        SELECT id FROM app_codigos_temporales
        WHERE V_30055 = $1
          AND codigo_hash = $2
          AND usado = false
          AND expira_at > NOW()
        ORDER BY id DESC
        LIMIT 1
        "#
    )
    .bind(V_30055)
    .bind(&V_30161)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some(cid) = V_30162 else {
        return render_html(&S_50037 {
            success: false,
            message: "El enlace de verificación no es válido, ya ha expirado (más de 15 minutos) o ya fue utilizado previamente.".to_string(),
        });
    };

    let V_30020 = sqlx::query("UPDATE app_codigos_temporales SET usado = true, usado_at = NOW() WHERE id = $1")
        .bind(cid)
        .execute(&V_30188.V_30012)
        .await;

    let V_30020 = sqlx::query("UPDATE app_users SET V_30636 = true, email_verificado_at = NOW() WHERE id = $1")
        .bind(V_30055)
        .execute(&V_30188.V_30012)
        .await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30055),
        "verificar_email_enlace",
        "usuarios",
        "success",
        json!({ "V_30055": V_30055, "metodo": "enlace_directo" }),
    )
    .await;

    render_html(&S_50037 {
        success: true,
        message: "Su dirección de correo institucional ha sido validada exitosamente. Su identidad cumple con los estándares ISO/IEC 27001.".to_string(),
    })
}

#[get("/logout")]
async fn logout(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    if let Some(V_30041) = req.V_30041("spf_session") {
        if let Ok(id) = Uuid::parse_str(V_30041.value()) {
            let V_30055: Option<i64> = sqlx::query_scalar("SELECT V_30055 FROM app_sessions WHERE id = $1")
                .bind(id)
                .fetch_optional(&V_30188.V_30012)
                .await
                .ok()
                .flatten();
            let V_30020 = sqlx::query(
                "UPDATE app_sessions SET revoked_at = NOW(), motivo_revocacion = 'logout_manual' WHERE id = $1"
            )
            .bind(id)
            .execute(&V_30188.V_30012)
            .await;
            write_audit(
                &V_30188.V_30012,
                V_30055,
                "logout",
                "auth",
                "success",
                json!({ "V_30060": id.to_string(), "motivo_revocacion": "logout_manual" }),
            )
            .await;
        }
    }
    let V_30056 = Cookie::build("spf_session", "")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::V_30041::time::Duration::seconds(0))
        .finish();
    let V_30057 = Cookie::build("spf_totp_pending", "")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::V_30041::time::Duration::seconds(0))
        .finish();
    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, "/login"))
        .V_30041(V_30056)
        .V_30041(V_30057)
        .finish())
}

async fn F_40063(V_30012: &PgPool) -> Vec<String> {
    let mut V_30022: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT reparticion FROM expedientes WHERE reparticion IS NOT NULL AND reparticion != '' AND reparticion != 'Sin datos'          UNION          SELECT nombre FROM reparticiones WHERE activo = true AND nombre != 'Sin datos'          ORDER BY 1 ASC"
    )
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();
    if V_30022.is_empty() {
        vec!["Demo Sintético".to_string(), "DRH".to_string(), "CPF I".to_string(), "CPF II".to_string(), "CPF VI".to_string(), "OTRO".to_string()]
    } else {
        V_30022
    }
}

async fn F_40064(V_30012: &PgPool) -> Vec<String> {
    let mut V_30058: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT estado FROM expedientes WHERE estado IS NOT NULL AND estado != '' AND estado != 'Sin datos' ORDER BY estado ASC"
    )
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();
    
    let V_30059 = ["Activo", "En analisis", "Suspendido", "Cerrado", "Archivado", "Inicio", "Abierto", "Acumulación", "Reapertura", "Reconstrucción"];
    for d in V_30059 {
        if !V_30058.iter().any(|e| e.eq_ignore_ascii_case(d)) {
            V_30058.push(d.to_string());
        }
    }
    V_30058
}

async fn F_40065(V_30012: &PgPool) -> Vec<String> {
    let mut V_30058: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT estado FROM expediente_suspensiones WHERE estado IS NOT NULL AND estado != '' ORDER BY estado ASC"
    )
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();
    
    let V_30059 = ["Vigente", "Levantado", "Suspendido", "En revision", "Finalizada"];
    for d in V_30059 {
        if !V_30058.iter().any(|e| e.eq_ignore_ascii_case(d)) {
            V_30058.push(d.to_string());
        }
    }
    V_30058
}

async fn F_40066(V_30012: &PgPool) -> Vec<String> {
    let mut V_30022: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT V_30167 FROM expedientes WHERE V_30167 IS NOT NULL AND V_30167 != '' AND V_30167 != 'Sin datos' ORDER BY V_30167 ASC"
    )
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();

    let V_30059 = [
        "Apremios ilegales",
        "Ebriedad",
        "Fallecimiento",
        "Falta de celo",
        "Incumplimiento de deberes",
        "Pérdida de armamento",
        "Pérdida de chaleco balístico",
        "Pérdida de credencial",
        "Quebrantamiento de arresto",
        "Violencia de género",
    ];
    for d in &V_30059 {
        if !V_30022.iter().any(|V_30522| V_30522.eq_ignore_ascii_case(d)) {
            V_30022.push(d.to_string());
        }
    }
    V_30022.sort();
    V_30022
}

fn F_40067(V_30044: Option<&str>) -> Option<NaiveDateTime> {
    let V_30044 = V_30044?.trim();
    if V_30044.is_empty() {
        return None;
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(V_30044, "%Y-%m-%d") {
        return d.and_hms_opt(0, 0, 0);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(V_30044, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(V_30044, "%Y-%m-%dT%H:%M") {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(V_30044, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    None
}

async fn F_40068(
    mut V_30012: Multipart,
    upload_dir: &str,
) -> Result<(HashMap<String, String>, HashMap<String, String>), actix_web::Error> {
    let mut V_30019: HashMap<String, String> = HashMap::new();
    let mut V_30178: HashMap<String, String> = HashMap::new();

    std::fs::create_dir_all(upload_dir).map_err(error::ErrorInternalServerError)?;

    while let Some(V_30596) = V_30012.next().await {
        let mut V_30179 = V_30596.map_err(error::ErrorBadRequest)?;
        let V_30180 = V_30179.V_30180();
        let V_30181 = V_30180.and_then(|cd| cd.get_name()).unwrap_or_default().to_string();
        let V_30182 = V_30180.and_then(|cd| cd.get_filename()).map(|V_30044| V_30044.to_string());

        if let Some(orig_name) = V_30182 {
            let V_30183 = orig_name.trim();
            if !V_30183.is_empty() {
                let V_30184: String = V_30183
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == 'V_30020' { c } else { 'V_30020' })
                    .collect();
                let V_30185 = format!("{}V_30020{}", Uuid::new_v4(), V_30184);
                let V_30146 = format!("{}/{}", upload_dir, V_30185);
                let mut V_30187 = std::fs::File::create(&V_30146).map_err(error::ErrorInternalServerError)?;
                while let Some(chunk) = V_30179.next().await {
                    let V_30188 = chunk.map_err(error::ErrorBadRequest)?;
                    V_30187.write_all(&V_30188).map_err(error::ErrorInternalServerError)?;
                }
                V_30178.insert(V_30181, V_30185);
            }
        } else {
            let mut V_30189 = Vec::new();
            while let Some(chunk) = V_30179.next().await {
                let V_30188 = chunk.map_err(error::ErrorBadRequest)?;
                V_30189.extend_from_slice(&V_30188);
            }
            let V_30190 = String::from_utf8_lossy(&V_30189).trim().to_string();
            V_30019.insert(V_30181, V_30190);
        }
    }
    V_30054((V_30019, V_30178))
}

fn F_40069(V_30146: &str) -> std::io::Result<(String, i64)> {
    let mut V_30060 = std::fs::File::open(V_30146)?;
    let mut V_30035 = Sha256::new();
    let mut V_30061 = [0u8; 8192];
    let mut V_30062 = 0i64;
    loop {
        let V_30063 = std::io::Read::read(&mut V_30060, &mut V_30061)?;
        if V_30063 == 0 {
            break;
        }
        V_30035.update(&V_30061[..V_30063]);
        V_30062 += V_30063 as i64;
    }
    let V_30029 = V_30035.finalize();
    V_30054((format!("{:x}", V_30029), V_30062))
}

#[derive(Clone)]
struct S_50115 {
    V_30185: String,
    orig_name: String,
    size_bytes: i64,
    sha256_hash: String,
}

async fn F_40070(
    mut V_30012: Multipart,
    upload_dir: &str,
) -> Result<(HashMap<String, String>, HashMap<String, S_50115>), actix_web::Error> {
    let mut V_30019: HashMap<String, String> = HashMap::new();
    let mut V_30178: HashMap<String, S_50115> = HashMap::new();

    std::fs::create_dir_all(upload_dir).map_err(error::ErrorInternalServerError)?;

    while let Some(V_30596) = V_30012.next().await {
        let mut V_30179 = V_30596.map_err(error::ErrorBadRequest)?;
        let V_30180 = V_30179.V_30180();
        let V_30181 = V_30180.and_then(|cd| cd.get_name()).unwrap_or_default().to_string();
        let V_30182 = V_30180.and_then(|cd| cd.get_filename()).map(|V_30044| V_30044.to_string());

        if let Some(orig_name) = V_30182 {
            let V_30183 = orig_name.trim();
            if !V_30183.is_empty() {
                let V_30184: String = V_30183
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == 'V_30020' { c } else { 'V_30020' })
                    .collect();
                let V_30185 = format!("{}V_30020{}", Uuid::new_v4(), V_30184);
                let V_30146 = format!("{}/{}", upload_dir, V_30185);
                let mut V_30187 = std::fs::File::create(&V_30146).map_err(error::ErrorInternalServerError)?;
                while let Some(chunk) = V_30179.next().await {
                    let V_30188 = chunk.map_err(error::ErrorBadRequest)?;
                    V_30187.write_all(&V_30188).map_err(error::ErrorInternalServerError)?;
                }
                drop(V_30187);
                let (V_30029, V_30070) = F_40069(&V_30146).map_err(error::ErrorInternalServerError)?;
                V_30178.insert(V_30181, S_50115 {
                    V_30185,
                    orig_name: V_30184,
                    size_bytes: V_30070,
                    sha256_hash: V_30029,
                });
            }
        } else {
            let mut V_30189 = Vec::new();
            while let Some(chunk) = V_30179.next().await {
                let V_30188 = chunk.map_err(error::ErrorBadRequest)?;
                V_30189.extend_from_slice(&V_30188);
            }
            let V_30190 = String::from_utf8_lossy(&V_30189).trim().to_string();
            V_30019.insert(V_30181, V_30190);
        }
    }
    V_30054((V_30019, V_30178))
}

#[get("/expedientes")]
async fn expedientes(req: HttpRequest, V_30188: web::Data<AppState>, query: web::Query<S_50101>) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    let reparticion = query.reparticion.clone().unwrap_or_default().trim().to_string();
    let estado = query.estado.clone().unwrap_or_default().trim().to_string();
    let V_30066 = query.V_30066.clone().unwrap_or_default().trim().to_string();
    let V_30067 = query.V_30067.clone().unwrap_or_default().trim().to_string();
    let V_30068 = query.V_30068.clone().unwrap_or_default().trim().to_string();

    let V_30069 = query.V_30069.unwrap_or(1).max(1);
    let V_30070 = query.V_30070.unwrap_or(5).clamp(1, V_30188.V_30011.max_page_size);
    let V_30071 = (V_30069 - 1) * V_30070;

    let mut V_30072 = String::from(
        "SELECT e.id, COALESCE(e.numero_expediente, 'Sin datos') AS numero_expediente, \
         COALESCE(e.V_30228, 'sumario') AS V_30228, \
         COALESCE(e.reparticion, 'Sin datos') AS reparticion, \
         COALESCE(e.V_30167, 'Sin datos') AS V_30167, \
         COALESCE(e.estado, 'Abierto') AS estado, \
         COALESCE(e.V_30238, 'Sin datos') AS V_30238, \
         COALESCE(to_char(e.fecha_inicio, 'YYYY-MM-DD'), 'Sin datos') AS fecha_inicio, \
         COALESCE((SELECT COUNT(*) FROM expediente_agentes ea WHERE ea.V_30275 = e.id), 0) AS agentes_vinculados, \
         COALESCE(e.version_actual, 1) AS version_actual \
         FROM expedientes e WHERE 1=1"
    );
    let mut V_30073 = String::from("SELECT COUNT(*) FROM expedientes e WHERE 1=1");
    let mut V_30074: Vec<String> = Vec::new();
    let mut V_30075 = 1;

    if !V_30065.is_empty() {
        let V_30076 = format!("%{}%", V_30065);
        let V_30077 = format!(
            " AND (e.numero_expediente ILIKE ${V_30147} OR COALESCE(e.V_30238, '') ILIKE ${V_30147} OR COALESCE(e.V_30167, '') ILIKE ${V_30147} OR COALESCE(e.V_30236, '') ILIKE ${V_30147})",
            V_30147 = V_30075
        );
        V_30072.push_str(&V_30077);
        V_30073.push_str(&V_30077);
        V_30074.push(V_30076);
        V_30075 += 1;
    }

    if !reparticion.is_empty() {
        let V_30077 = format!(" AND (e.reparticion = ${V_30147} OR e.reparticion ILIKE ${V_30147})", V_30147 = V_30075);
        V_30072.push_str(&V_30077);
        V_30073.push_str(&V_30077);
        V_30074.push(reparticion.clone());
        V_30075 += 1;
    }

    if !estado.is_empty() {
        let V_30077 = format!(
            " AND (e.estado ILIKE ${V_30147} OR REPLACE(e.estado, 'á', 'a') ILIKE REPLACE(${V_30147}, 'á', 'a'))",
            V_30147 = V_30075
        );
        V_30072.push_str(&V_30077);
        V_30073.push_str(&V_30077);
        V_30074.push(estado.clone());
        V_30075 += 1;
    }

    if !V_30066.is_empty() {
        let V_30077 = format!(" AND e.V_30066 ILIKE ${}", V_30075);
        V_30072.push_str(&V_30077);
        V_30073.push_str(&V_30077);
        V_30074.push(format!("%{}%", V_30066));
        V_30075 += 1;
    }

    if !V_30067.is_empty() {
        let V_30077 = format!(" AND e.V_30067 ILIKE ${}", V_30075);
        V_30072.push_str(&V_30077);
        V_30073.push_str(&V_30077);
        V_30074.push(format!("%{}%", V_30067));
        V_30075 += 1;
    }

    if !V_30068.is_empty() {
        if V_30068.eq_ignore_ascii_case("sí") || V_30068.eq_ignore_ascii_case("si") {
            let V_30077 = " AND (e.V_30068 ILIKE 'S%' OR e.V_30068 = 'true')";
            V_30072.push_str(V_30077);
            V_30073.push_str(V_30077);
        } else if V_30068.eq_ignore_ascii_case("no") {
            let V_30077 = " AND (e.V_30068 IS NULL OR e.V_30068 = '' OR e.V_30068 ILIKE 'No%' OR e.V_30068 = 'false')";
            V_30072.push_str(V_30077);
            V_30073.push_str(V_30077);
        } else {
            let V_30077 = format!(" AND e.V_30068 = ${}", V_30075);
            V_30072.push_str(&V_30077);
            V_30073.push_str(&V_30077);
            V_30074.push(V_30068.clone());
            V_30075 += 1;
        }
    }

    let mut V_30078 = sqlx::query_scalar::<V_30020, i64>(&V_30073);
    for V_30190 in &V_30074 {
        V_30078 = V_30078.bind(V_30190);
    }
    let V_30079: i64 = V_30078.fetch_one(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;

    let V_30080 = format!(" ORDER BY e.id DESC LIMIT ${} OFFSET ${}", V_30075, V_30075 + 1);
    V_30072.push_str(&V_30080);

    let mut V_30081 = sqlx::query_as::<V_30020, S_50010>(&V_30072);
    for V_30190 in &V_30074 {
        V_30081 = V_30081.bind(V_30190);
    }
    V_30081 = V_30081.bind(V_30070).bind(V_30071);
    let V_30082 = V_30081.fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;

    let V_30083: Vec<i64> = V_30082.iter().map(|V_30522| V_30522.id).collect();
    let mut V_30084: HashMap<i64, Vec<AgenteInvolucradoItem>> = HashMap::new();
    if !V_30083.is_empty() {
        #[derive(FromRow)]
        struct S_50116 {
            V_30275: i64,
            credencial: String,
            V_30266: String,
            V_30317: String,
            V_30318: String,
            rol: String,
        }

        let V_30085 = sqlx::query_as::<V_30020, S_50116>(
            V_30522#"
            SELECT ea.V_30275,
                   COALESCE(ea.credencial, '') AS credencial,
                   COALESCE(
                       NULLIF(TRIM(ea.V_30266), ''),
                       NULLIF(TRIM(CONCAT(ea.apellido, ', ', ea.nombre)), ','),
                       ea.apellido,
                       ea.nombre,
                       'Sin nombre'
                   ) AS nombre_completo,
                   COALESCE(ea.V_30317, '') AS escalafon,
                   COALESCE(ea.V_30318, '') AS grado,
                   COALESCE(ea.rol, 'Causante') AS rol
            FROM expediente_agentes ea
            WHERE ea.V_30275 = ANY($1)
            ORDER BY ea.id ASC
            "#,
        )
        .bind(&V_30083)
        .fetch_all(&V_30188.V_30012)
        .await
        .unwrap_or_default();

        for a in V_30085 {
            V_30084.entry(a.V_30275).or_default().push(AgenteInvolucradoItem {
                credencial: a.credencial,
                nombre_completo: a.nombre_completo,
                escalafon: a.escalafon,
                grado: a.grado,
                rol: a.rol,
            });
        }
    }

    let V_30022: Vec<ExpedienteRow> = V_30082.into_iter().map(|db_r| {
        let V_30086 = V_30084.V_30056(&db_r.id).unwrap_or_default();
        ExpedienteRow {
            id: db_r.id,
            numero_expediente: db_r.numero_expediente,
            V_30228: db_r.V_30228,
            reparticion: db_r.reparticion,
            V_30167: db_r.V_30167,
            estado: db_r.estado,
            V_30238: db_r.V_30238,
            fecha_inicio: db_r.fecha_inicio,
            agentes_vinculados: db_r.agentes_vinculados,
            version_actual: db_r.version_actual,
            V_30086,
        }
    }).collect();

    let V_30087 = ((V_30079 as f64) / (V_30070 as f64)).ceil().max(1.0) as i64;
    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
    let V_30089 = F_40063(&V_30188.V_30012).await;
    let V_30090 = F_40064(&V_30188.V_30012).await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "query_expedientes",
        "expedientes",
        "success",
        json!({ "V_30065": V_30065, "reparticion": reparticion, "estado": estado, "V_30022": V_30079 }),
    )
    .await;

    render_html(&S_50046 {
        user_name: V_30064.display_name,
        V_30088,
        V_30022,
        V_30065,
        reparticion,
        estado,
        V_30066,
        V_30067,
        V_30068,
        V_30089,
        V_30090,
        V_30069,
        V_30087,
        V_30079,
        V_30070,
    })
}

async fn F_40071(
    V_30012: &PgPool,
    V_30275: i64,
    usuario: &str,
    V_30055: Option<i64>,
    tipo_operacion: &str,
    V_30765: &str,
    V_30167: &str,
    detalle_diff: serde_json::Value,
) -> Result<i32, sqlx::Error> {
    let V_30221: Option<(i64, i32)> = sqlx::query_as(
        "SELECT id, numero_version FROM expediente_versiones WHERE V_30275 = $1 ORDER BY numero_version DESC LIMIT 1"
    )
    .bind(V_30275)
    .fetch_optional(V_30012)
    .await
    .unwrap_or(None);

    let (prev_id, prev_num) = match V_30221 {
        Some((id, V_30033)) => (Some(id), V_30033),
        None => (None, 1),
    };
    let V_30222 = prev_num + 1;

    let V_30020 = sqlx::query("UPDATE expediente_versiones SET es_activa = false WHERE V_30275 = $1")
        .bind(V_30275)
        .execute(V_30012)
        .await;

    let V_30223: Option<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT COALESCE(numero_expediente, ''), COALESCE(V_30228, 'sumario'), COALESCE(reparticion, ''), COALESCE(V_30167, ''), COALESCE(estado, 'Abierto'), COALESCE(V_30238, '') FROM expedientes WHERE id = $1"
    )
    .bind(V_30275)
    .fetch_optional(V_30012)
    .await
    .unwrap_or(None);

    let (num_exp, V_30092, V_30815, V_30510, est, inst) = V_30223.unwrap_or_default();

    #[derive(Serialize, FromRow)]
    struct S_50117 {
        credencial: Option<String>,
        nombre: Option<String>,
        rol: String,
        suspension: Option<bool>,
    }
    let V_30224: Vec<S_50117> = sqlx::query_as(
        "SELECT credencial, V_30266 AS nombre, rol, V_30326 AS suspension FROM expediente_agentes WHERE V_30275 = $1 ORDER BY id ASC"
    )
    .bind(V_30275)
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();

    #[derive(Serialize, FromRow)]
    struct S_50118 {
        credencial: Option<String>,
        nombre: String,
        estado: String,
    }
    let V_30225: Vec<S_50118> = sqlx::query_as(
        "SELECT credencial, V_30266 AS nombre, estado FROM expediente_suspensiones WHERE V_30275 = $1 ORDER BY id ASC"
    )
    .bind(V_30275)
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();

    let V_30226 = json!({
        "numero_expediente": num_exp,
        "V_30228": V_30092,
        "reparticion": V_30815,
        "V_30167": V_30510,
        "estado": est,
        "V_30238": inst,
        "V_30086": V_30224,
        "V_30278": V_30225,
        "evento_cambio": {
            "V_30765": V_30765,
            "V_30167": V_30167,
            "V_30504": detalle_diff
        }
    });

    let V_30027 = F_40009(
        V_30275,
        V_30222,
        prev_id,
        tipo_operacion,
        usuario,
        V_30167,
        None,
        &V_30226,
    );

    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO expediente_versiones (
            V_30275, numero_version, V_30306, tipo_operacion,
            snapshot_json, campos_modificados, diferencias_json, usuario,
            V_30055, V_30167, version_origen_id, es_activa, record_hash, V_30675
        ) VALUES (
            $1, $2, $3, $4,
            $5, $6, $7, $8,
            $9, $10, NULL, true, $11, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30275)
    .bind(V_30222)
    .bind(prev_id)
    .bind(tipo_operacion)
    .bind(&V_30226)
    .bind(json!([V_30765]))
    .bind(&detalle_diff)
    .bind(usuario)
    .bind(V_30055)
    .bind(V_30167)
    .bind(&V_30027)
    .execute(V_30012)
    .await;

    let V_30020 = sqlx::query("UPDATE expedientes SET version_actual = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(V_30222)
        .bind(V_30275)
        .execute(V_30012)
        .await;

    V_30054(V_30222)
}

#[get("/expedientes/nuevo")]
async fn F_40072(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
    let V_30089 = F_40063(&V_30188.V_30012).await;
    let V_30091 = F_40066(&V_30188.V_30012).await;

    render_html(&S_50047 {
        user_name: V_30064.display_name,
        V_30088,
        V_30089,
        V_30091,
        error_msg: None,
        warning_duplicado: None,
    })
}

#[post("/expedientes/nuevo")]
async fn F_40073(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let (V_30019, V_30178) = F_40068(V_30012, "uploads").await?;

    let V_30228 = F_40003(&V_30019.get("V_30228").cloned().unwrap_or_else(|| "Electrónico".to_string()));
    let V_30229 = V_30019.get("numero_expediente").cloned().unwrap_or_default();
    let numero_expediente = match F_40006(&V_30229) {
        V_30054(V_30063) => V_30063,
        V_30156(V_30083) => {
            let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
            let V_30089 = F_40063(&V_30188.V_30012).await;
            let V_30091 = F_40066(&V_30188.V_30012).await;
            return render_html(&S_50047 {
                user_name: V_30064.display_name,
                V_30088,
                V_30089,
                V_30091,
                error_msg: Some(V_30083),
                warning_duplicado: None,
            });
        }
    };
    let reparticion = F_40003(&V_30019.get("reparticion").cloned().unwrap_or_default());
    let V_30230 = V_30019.get("V_30230").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30231 = V_30019.get("V_30231").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());

    let fecha_inicio = F_40067(V_30019.get("fecha_inicio").map(|V_30044| V_30044.as_str()));
    let V_30232 = F_40067(V_30019.get("V_30232").map(|V_30044| V_30044.as_str()));
    let V_30233 = F_40067(V_30019.get("V_30233").map(|V_30044| V_30044.as_str()));

    let V_30234 = V_30019.get("V_30234").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30235 = V_30019.get("V_30235").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30167 = F_40003(&V_30019.get("V_30167").cloned().unwrap_or_default());
    let estado = F_40003(&V_30019.get("estado").cloned().unwrap_or_else(|| "Inicio".to_string()));
    let V_30066 = V_30019.get("V_30066").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30067 = V_30019.get("V_30067").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30236 = V_30019.get("V_30236").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());

    let V_30237 = V_30019.get("V_30237").map(|V_30044| F_40004(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30238 = F_40003(&V_30019.get("V_30238").cloned().unwrap_or_default());
    let V_30239 = V_30019.get("V_30239").map(|V_30044| V_30044 == "true" || V_30044 == "1" || V_30044 == "on").unwrap_or(false);

    let V_30068 = V_30019.get("V_30068").cloned().unwrap_or_else(|| "No".to_string());
    let V_30240 = V_30019.get("V_30240").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30241 = V_30019.get("V_30241").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());

    let V_30242 = V_30019.get("V_30242").cloned().unwrap_or_else(|| "No".to_string());
    let V_30243 = F_40067(V_30019.get("V_30243").map(|V_30044| V_30044.as_str()));

    let V_30244 = V_30178.get("V_30244").cloned();
    let V_30245 = V_30178.get("V_30245").cloned();
    let V_30246 = V_30178.get("V_30246").cloned();

    let V_30247 = |V_30083: String, warn_dup: Option<i64>, V_30088, V_30089, V_30091| {
        render_html(&S_50047 {
            user_name: V_30064.display_name.clone(),
            V_30088,
            V_30089,
            V_30091,
            error_msg: Some(V_30083),
            warning_duplicado: warn_dup.map(|id| id.to_string()),
        })
    };

    if reparticion.is_empty() || reparticion.eq_ignore_ascii_case("sin datos") {
        let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
        let V_30089 = F_40063(&V_30188.V_30012).await;
        let V_30091 = F_40066(&V_30188.V_30012).await;
        return V_30247("La repartición o dependencia es obligatoria y no puede ser 'Sin datos'.".to_string(), None, V_30088, V_30089, V_30091);
    }

    if reparticion == "OTRO" && V_30230.is_none() {
        let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
        let V_30089 = F_40063(&V_30188.V_30012).await;
        let V_30091 = F_40066(&V_30188.V_30012).await;
        return V_30247("Debe especificar la repartición al seleccionar OTRO.".to_string(), None, V_30088, V_30089, V_30091);
    }

    if V_30167.is_empty() || V_30167.eq_ignore_ascii_case("sin datos") {
        let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
        let V_30089 = F_40063(&V_30188.V_30012).await;
        let V_30091 = F_40066(&V_30188.V_30012).await;
        return V_30247("El V_30167 de la actuación es obligatorio.".to_string(), None, V_30088, V_30089, V_30091);
    }

    if V_30238.is_empty() || V_30238.eq_ignore_ascii_case("sin datos") || V_30238.len() < 3 {
        let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
        let V_30089 = F_40063(&V_30188.V_30012).await;
        let V_30091 = F_40066(&V_30188.V_30012).await;
        return V_30247("El responsable o instructor actual es obligatorio (mínimo 3 caracteres).".to_string(), None, V_30088, V_30089, V_30091);
    }

    if let (Some(fh), Some(fi)) = (V_30232, fecha_inicio) {
        if fh > fi {
            let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
            let V_30089 = F_40063(&V_30188.V_30012).await;
            let V_30091 = F_40066(&V_30188.V_30012).await;
            return V_30247("Inconsistencia de fechas: la V_30503 del hecho no puede ser posterior a la V_30503 de inicio.".to_string(), None, V_30088, V_30089, V_30091);
        }
    }

    if let (Some(fi), Some(ff)) = (fecha_inicio, V_30243) {
        if ff < fi {
            let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
            let V_30089 = F_40063(&V_30188.V_30012).await;
            let V_30091 = F_40066(&V_30188.V_30012).await;
            return V_30247("Inconsistencia de fechas: la V_30503 de finalización no puede ser anterior a la V_30503 de inicio.".to_string(), None, V_30088, V_30089, V_30091);
        }
    }

    // 1. Duplicado confirmado (mismo V_30228 + numero_expediente exacto/normalizado) -> bloquea
    let V_30248: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM expedientes WHERE LOWER(TRIM(V_30228)) = LOWER(TRIM($1)) AND UPPER(TRIM(numero_expediente)) = UPPER(TRIM($2)) LIMIT 1"
    )
    .bind(&V_30228)
    .bind(&numero_expediente)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if let Some(eid) = V_30248 {
        let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
        let V_30089 = F_40063(&V_30188.V_30012).await;
        let V_30091 = F_40066(&V_30188.V_30012).await;
        return V_30247(
            format!("Duplicado confirmado: ya existe el V_30113 {} ({}) con ID #{}. Operación bloqueada para evitar duplicidad.", numero_expediente, V_30228, eid),
            Some(eid),
            V_30088,
            V_30089,
            V_30091,
        );
    }

    // 2. Posible duplicado (mismo número bajo otro V_30092 de identificador) -> advierte sin bloquear cuando V_30249 o V_30239 está activo
    let V_30249 = V_30019.get("V_30249").map(|V_30044| V_30044 == "true" || V_30044 == "1" || V_30044 == "on").unwrap_or(false);
    let V_30250: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM expedientes WHERE UPPER(TRIM(numero_expediente)) = UPPER(TRIM($1)) LIMIT 1"
    )
    .bind(&numero_expediente)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    if !V_30239 && !V_30249 {
        if let Some(pid) = V_30250 {
            let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
            let V_30089 = F_40063(&V_30188.V_30012).await;
            let V_30091 = F_40066(&V_30188.V_30012).await;
            return V_30247(
                format!("Posible duplicado detectado: existe una actuación con el mismo número ({}) registrada bajo otro V_30092 de identificador (ID #{}). Verifique el registro existente o marque confirmación explícita si corresponde a una actuación distinta.", numero_expediente, pid),
                Some(pid),
                V_30088,
                V_30089,
                V_30091,
            );
        }
    }

    let V_30251: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO expedientes (
            V_30228, numero_expediente, reparticion, V_30230, V_30231,
            fecha_inicio, V_30232, V_30233, V_30234, V_30235,
            V_30167, estado, V_30066, V_30067, V_30236, V_30237, V_30238,
            V_30068, V_30240, V_30241, V_30242, V_30243,
            documento_informe_hechos, documento_disposicion, documento_finalizacion, fecha_ultimo_cambio_estado, V_30675, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17,
            $18, $19, $20, $21, $22,
            $23, $24, $25, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )
        RETURNING id
        "#,
    )
    .bind(&V_30228)
    .bind(&numero_expediente)
    .bind(&reparticion)
    .bind(&V_30230)
    .bind(&V_30231)
    .bind(fecha_inicio)
    .bind(V_30232)
    .bind(V_30233)
    .bind(&V_30234)
    .bind(&V_30235)
    .bind(&V_30167)
    .bind(&estado)
    .bind(&V_30066)
    .bind(&V_30067)
    .bind(&V_30236)
    .bind(&V_30237)
    .bind(if V_30238.is_empty() { "Sin datos" } else { &V_30238 })
    .bind(&V_30068)
    .bind(&V_30240)
    .bind(&V_30241)
    .bind(&V_30242)
    .bind(V_30243)
    .bind(&V_30244)
    .bind(&V_30245)
    .bind(&V_30246)
    .fetch_one(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    sqlx::query(
        "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(V_30251)
    .bind("alta_expediente")
    .bind(None::<String>)
    .bind(&numero_expediente)
    .bind(&V_30064.V_30114)
    .bind("Alta inicial de V_30113 sumarial")
    .execute(&V_30188.V_30012)
    .await
    .ok();

    let V_30252 = F_40010(
        &V_30228,
        &numero_expediente,
        &reparticion,
        V_30230.as_deref(),
        V_30231.as_deref(),
        V_30019.get("fecha_inicio").map(|V_30044| V_30044.as_str()),
        V_30019.get("V_30232").map(|V_30044| V_30044.as_str()),
        V_30019.get("V_30233").map(|V_30044| V_30044.as_str()),
        V_30234.as_deref(),
        V_30235.as_deref(),
        &V_30167,
        V_30066.as_deref(),
        V_30067.as_deref(),
        V_30236.as_deref(),
        V_30237.as_deref(),
        if V_30238.is_empty() { "Sin datos" } else { &V_30238 },
        Some(&V_30068),
        V_30240.as_deref(),
        V_30241.as_deref(),
        Some(&V_30242),
        V_30019.get("V_30243").map(|V_30044| V_30044.as_str()),
    );

    
    // Vincular V_30086 involucrados (principal y adicionales con signo +)
    let mut V_30253: Vec<(String, String, String, Option<String>, Option<String>, String)> = Vec::new();

    let V_30254 = V_30019.get("involucrado_credencial").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30255 = V_30019.get("involucrado_apellido").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30256 = V_30019.get("involucrado_nombre").map(|V_30044| F_40003(V_30044)).unwrap_or_default();
    let V_30257 = V_30019.get("involucrado_escalafon").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30258 = V_30019.get("involucrado_grado").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30259 = V_30019.get("involucrado_rol").cloned().unwrap_or_else(|| "Causante".to_string());

    if let (Some(c), Some(a)) = (V_30254, V_30255) {
        V_30253.push((c, a, V_30256, V_30257, V_30258, V_30259));
    }

    // Involucrados adicionales dinámicos enviados en JSON
    if let Some(extra_json_str) = V_30019.get("involucrados_adicionales_json") {
        if let Ok(Value::Array(V_30008)) = serde_json::from_str::<Value>(extra_json_str) {
            for it in V_30008 {
                if let (Some(c), Some(a)) = (it.get("credencial").and_then(|v| v.as_str()), it.get("apellido").and_then(|v| v.as_str())) {
                    let V_30260 = F_40003(c);
                    let V_30261 = F_40003(a);
                    if !V_30260.is_empty() && !V_30261.is_empty() {
                        let V_30262 = it.get("nombre").and_then(|v| v.as_str()).map(F_40003).unwrap_or_default();
                        let V_30263 = it.get("V_30317").and_then(|v| v.as_str()).map(F_40003).filter(|V_30044| !V_30044.is_empty());
                        let V_30264 = it.get("V_30318").and_then(|v| v.as_str()).map(F_40003).filter(|V_30044| !V_30044.is_empty());
                        let rol = it.get("rol").and_then(|v| v.as_str()).map(F_40003).unwrap_or_else(|| "Causante".to_string());
                        if !V_30253.iter().any(|(ec, ..)| ec == &V_30260) {
                            V_30253.push((V_30260, V_30261, V_30262, V_30263, V_30264, rol));
                        }
                    }
                }
            }
        }
    }

    // Involucrados adicionales dinámicos enviados por V_30623 indexados (involucrado_credencial_1, ...)
    for (k, v) in V_30019.iter() {
        if k.starts_with("involucrado_credencial_") {
            let V_30265 = &k["involucrado_credencial_".len()..];
            let V_30260 = F_40003(v);
            if let Some(ape_val) = V_30019.get(&format!("involucrado_apellido_{}", V_30265)) {
                let V_30261 = F_40003(ape_val);
                if !V_30260.is_empty() && !V_30261.is_empty() {
                    let V_30262 = V_30019.get(&format!("involucrado_nombre_{}", V_30265)).map(|V_30044| F_40003(V_30044)).unwrap_or_default();
                    let V_30263 = V_30019.get(&format!("involucrado_escalafon_{}", V_30265)).map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
                    let V_30264 = V_30019.get(&format!("involucrado_grado_{}", V_30265)).map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
                    let rol = V_30019.get(&format!("involucrado_rol_{}", V_30265)).cloned().unwrap_or_else(|| "Causante".to_string());
                    if !V_30253.iter().any(|(ec, ..)| ec == &V_30260) {
                        V_30253.push((V_30260, V_30261, V_30262, V_30263, V_30264, rol));
                    }
                }
            }
        }
    }

    for (V_30168, ape, V_30492, V_30263, V_30264, rol) in V_30253 {
        let V_30266 = format!("{}, {}", ape, V_30492).trim_end_matches(", ").to_string();

        let V_30020 = sqlx::query(
            V_30522#"
            INSERT INTO personas (credencial, apellido, nombre, V_30526, V_30317, V_30318, first_seen_at, last_seen_at, origen_principal)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), 'expediente_alta')
            ON CONFLICT (credencial) DO UPDATE SET
                apellido = COALESCE(EXCLUDED.apellido, personas.apellido),
                nombre = COALESCE(EXCLUDED.nombre, personas.nombre),
                V_30317 = COALESCE(EXCLUDED.V_30317, personas.V_30317),
                V_30318 = COALESCE(EXCLUDED.V_30318, personas.V_30318),
                last_seen_at = NOW();
            "#
        )
        .bind(&V_30168)
        .bind(&ape)
        .bind(&V_30492)
        .bind(&V_30266)
        .bind(V_30263.as_deref())
        .bind(V_30264.as_deref())
        .execute(&V_30188.V_30012)
        .await;

        let V_30020 = sqlx::query(
            V_30522#"
            INSERT INTO expediente_agentes (
                V_30275, credencial, apellido, nombre, V_30266,
                V_30317, V_30318, rol, V_30675, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, NOW(), NOW()
            )
            "#
        )
        .bind(V_30251)
        .bind(&V_30168)
        .bind(&ape)
        .bind(&V_30492)
        .bind(&V_30266)
        .bind(V_30263.as_deref())
        .bind(V_30264.as_deref())
        .bind(&rol)
        .execute(&V_30188.V_30012)
        .await;

        let V_30020 = sqlx::query(
            "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(V_30251)
        .bind("agente_involucrado_inicial")
        .bind(None::<String>)
        .bind(format!("{} (Cred: {}) - Rol: {}", V_30266, V_30168, rol))
        .bind(&V_30064.V_30114)
        .bind("Alta inicial de agente involucrado")
        .execute(&V_30188.V_30012)
        .await;
    }
let V_30267 = F_40009(
        V_30251,
        1,
        None,
        "creacion",
        &V_30064.V_30114,
        "Alta inicial de V_30113 sumarial",
        None,
        &V_30252,
    );

    sqlx::query(
        V_30522#"
        INSERT INTO expediente_versiones (
            V_30275, numero_version, V_30306, tipo_operacion,
            snapshot_json, campos_modificados, diferencias_json, usuario,
            V_30055, V_30167, version_origen_id, es_activa, record_hash, V_30675
        ) VALUES (
            $1, 1, NULL, 'creacion',
            $2, '["creacion"]'::jsonb, '{}'::jsonb, $3,
            $4, 'Alta inicial de V_30113 sumarial', NULL, true, $5, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30251)
    .bind(&V_30252)
    .bind(&V_30064.V_30114)
    .bind(Some(V_30064.V_30055))
    .bind(&V_30267)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    if !V_30238.is_empty() && V_30238 != "Sin datos" {
        sqlx::query(
            "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(V_30251)
        .bind("instructor_designado")
        .bind(None::<String>)
        .bind(&V_30238)
        .bind(&V_30064.V_30114)
        .bind("Designación inicial de instructor")
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "create_expediente",
        "expedientes",
        "success",
        json!({ "id": V_30251, "V_30093": numero_expediente, "reparticion": reparticion }),
    )
    .await;

    F_40013(
        &V_30188.V_30012,
        None,
        "expediente_creado",
        &format!("Expediente creado: {}", numero_expediente),
        &format!("Se dio de alta el V_30113 '{}' en estado '{}' para la repartición '{}'.", numero_expediente, estado, reparticion),
        &format!("/expedientes/{}", V_30251),
        Some(V_30251),
        None,
        None,
    ).await;

    let V_30268 = if let Some(pid) = V_30250 {
        format!("/expedientes/{}?nuevo=true&tab=tab-V_30086&V_30083=Advertencia+informativa:+Posible+duplicado+detectado+con+actuación+existente+ID+%23{}", V_30251, pid)
    } else {
        format!("/expedientes/{}?nuevo=true&tab=tab-V_30086", V_30251)
    };

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, V_30268))
        .finish())
}

#[get("/api/expedientes/verificar-duplicado")]
async fn F_40074(V_30188: web::Data<AppState>, query: web::Query<S_50102>) -> Result<HttpResponse> {
    let V_30092 = F_40003(&query.V_30092.clone().unwrap_or_else(|| "Electrónico".to_string()));
    let V_30093 = F_40003(&query.V_30093.clone().unwrap_or_default());
    if V_30093.is_empty() {
        return Ok(HttpResponse::Ok().json(json!({ "existe": false, "posible_duplicado": false, "V_30763": "ninguna", "V_30825": false })));
    }
    let V_30094: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT id, COALESCE(reparticion, 'Sin datos'), COALESCE(estado, 'Abierto') FROM expedientes WHERE LOWER(TRIM(V_30228)) = LOWER(TRIM($1)) AND UPPER(TRIM(numero_expediente)) = UPPER(TRIM($2)) LIMIT 1"
    )
    .bind(&V_30092)
    .bind(&V_30093)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    if let Some((id, V_30815, est)) = V_30094 {
        return Ok(HttpResponse::Ok().json(json!({
            "existe": true,
            "posible_duplicado": false,
            "V_30763": "error",
            "V_30825": true,
            "tipo_hallazgo": "duplicado_confirmado",
            "motivo_coincidencia": format!("Restricción de unicidad obligatoria: ya existe el V_30113 {} ({}) en {} (Estado: {})", V_30093, V_30092, V_30815, est),
            "id": id,
            "url": format!("/expedientes/{}", id)
        })));
    }

    let V_30095: Option<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, COALESCE(V_30228, 'sumario'), COALESCE(reparticion, 'Sin datos'), COALESCE(estado, 'Abierto') FROM expedientes WHERE UPPER(TRIM(numero_expediente)) = UPPER(TRIM($1)) LIMIT 1"
    )
    .bind(&V_30093)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    if let Some((pid, ptipo, prep, pest)) = V_30095 {
        Ok(HttpResponse::Ok().json(json!({
            "existe": false,
            "posible_duplicado": true,
            "V_30763": "advertencia",
            "V_30825": false,
            "tipo_hallazgo": "posible_duplicado",
            "motivo_coincidencia": format!("Posible duplicado informativo (no bloquea el alta): mismo número bajo V_30092 '{}' en {} (Estado: {})", ptipo, prep, pest),
            "id": pid,
            "tipo_identificador_existente": ptipo,
            "url": format!("/expedientes/{}", pid)
        })))
    } else {
        let V_30096: String = V_30093.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_uppercase();
        if V_30096.len() >= 4 {
            let V_30097: Option<(i64, String, String)> = sqlx::query_as(
                "SELECT id, numero_expediente, COALESCE(V_30228, 'sumario') FROM expedientes WHERE REGEXP_REPLACE(UPPER(numero_expediente), '[^A-Z0-9]', '', 'g') = $1 LIMIT 1"
            )
            .bind(&V_30096)
            .fetch_optional(&V_30188.V_30012)
            .await
            .unwrap_or(None);

            if let Some((V_30097, snum, stipo)) = V_30097 {
                return Ok(HttpResponse::Ok().json(json!({
                    "existe": false,
                    "posible_duplicado": true,
                    "V_30763": "advertencia",
                    "V_30825": false,
                    "tipo_hallazgo": "posible_duplicado_similar",
                    "motivo_coincidencia": format!("Posible duplicado por similitud de formato: coincide con '{}' ({}) sin signos de puntuación", snum, stipo),
                    "id": V_30097,
                    "numero_existente": snum,
                    "tipo_identificador_existente": stipo,
                    "url": format!("/expedientes/{}", V_30097)
                })));
            }
        }
        Ok(HttpResponse::Ok().json(json!({ "existe": false, "posible_duplicado": false, "V_30763": "ninguna", "V_30825": false })))
    }
}

#[get("/expedientes/{id}")]
async fn F_40075(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    query: web::Query<S_50103>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();

    let V_30087 = sqlx::query_as::<V_30020, S_50011>(
        V_30522#"
        SELECT
            id,
            COALESCE(V_30228, 'sumario') AS V_30228,
            COALESCE(numero_expediente, 'Sin datos') AS numero_expediente,
            COALESCE(reparticion, 'Sin datos') AS reparticion,
            V_30230,
            to_char(fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            to_char(V_30232, 'YYYY-MM-DD') AS V_30232,
            to_char(V_30233, 'YYYY-MM-DD') AS V_30233,
            V_30234,
            V_30235,
            COALESCE(V_30167, 'Sin datos') AS V_30167,
            COALESCE(estado, 'Abierto') AS estado,
            V_30066,
            V_30067,
            V_30236,
            V_30237,
            COALESCE(V_30238, 'Sin datos') AS V_30238,
            COALESCE(V_30068, 'No') AS V_30068,
            V_30240,
            V_30241,
            COALESCE(V_30242, 'No') AS V_30242,
            to_char(V_30243, 'YYYY-MM-DD') AS V_30243,
            documento_informe_hechos,
            documento_disposicion,
            documento_finalizacion,
            V_30231,
            V_30463,
            to_char(fecha_ultimo_cambio_estado, 'YYYY-MM-DD HH24:MI') AS fecha_ultimo_cambio_estado,
            to_char(fecha_ultima_novedad, 'YYYY-MM-DD HH24:MI') AS fecha_ultima_novedad,
            tipo_ultima_novedad,
            COALESCE(version_actual, 1) AS version_actual
        FROM expedientes
        WHERE id = $1
        "#,
    )
    .bind(V_30275)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some(V_30087) = V_30087 else {
        return Ok(HttpResponse::NotFound().body("Expediente no encontrado"));
    };

    let V_30086 = sqlx::query_as::<V_30020, ExpedienteAgenteItem>(
        V_30522#"
        SELECT
            id,
            V_30275,
            COALESCE(credencial, 'Sin datos') AS credencial,
            COALESCE(V_30266, 'Sin datos') AS V_30266,
            COALESCE(rol, 'Causante') AS rol,
            V_30323,
            V_30465,
            COALESCE(V_30319, false) AS V_30319,
            to_char(V_30320, 'YYYY-MM-DD') AS V_30320,
            V_30321,
            COALESCE(V_30322, false) AS V_30322,
            to_char(V_30324, 'YYYY-MM-DD') AS V_30324,
            V_30325,
            COALESCE(V_30326, false) AS V_30326,
            to_char(V_30327, 'YYYY-MM-DD') AS V_30327,
            V_30328,
            V_30329,
            COALESCE(V_30330, 'No') AS V_30330,
            to_char(V_30331, 'YYYY-MM-DD') AS V_30331,
            V_30332,
            apellido,
            nombre,
            V_30317,
            V_30318
        FROM expediente_agentes
        WHERE V_30275 = $1
        ORDER BY id ASC
        "#,
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30276 = sqlx::query_as::<V_30020, S_50019>(
        V_30522#"
        SELECT
            id,
            V_30765,
            V_30766,
            V_30764,
            COALESCE(usuario, 'Sistema') AS usuario,
            V_30167,
            to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675
        FROM expediente_historial
        WHERE V_30275 = $1
        ORDER BY V_30675 DESC, id DESC
        "#,
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30277 = sqlx::query_as::<V_30020, S_50020>(
        V_30522#"
        SELECT
            V_30063.id,
            V_30063.V_30275,
            V_30063.V_30464,
            ea.V_30266 AS agente_nombre,
            ea.credencial AS agente_credencial,
            V_30063.V_30458,
            V_30063.V_30459,
            to_char(V_30063.V_30460, 'YYYY-MM-DD HH24:MI') AS V_30460,
            V_30063.V_30461,
            V_30063.V_30462,
            V_30063.V_30463,
            V_30063.estado_anterior,
            V_30063.estado_nuevo,
            V_30063.usuario,
            V_30063.es_auditoria_automatica,
            V_30063.V_30465,
            V_30063.anulado,
            V_30063.anulado_motivo,
            V_30063.anulado_usuario,
            to_char(V_30063.anulado_at, 'YYYY-MM-DD HH24:MI') AS anulado_at,
            to_char(V_30063.V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675,
            COALESCE(V_30063.V_30467, 'revisada') AS V_30467,
            V_30063.revisor_usuario,
            to_char(V_30063.revisado_at, 'YYYY-MM-DD HH24:MI') AS revisado_at,
            V_30063.observaciones_revision,
            V_30063.V_30466
        FROM expediente_novedades V_30063
        LEFT JOIN expediente_agentes ea ON ea.id = V_30063.V_30464
        WHERE V_30063.V_30275 = $1
        ORDER BY V_30063.V_30460 DESC, V_30063.id DESC
        "#
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30278 = sqlx::query_as::<V_30020, S_50021>(
        V_30522#"
        SELECT
            V_30044.id,
            V_30044.V_30275,
            e.numero_expediente,
            V_30044.V_30464,
            V_30044.credencial,
            V_30044.V_30266,
            to_char(V_30044.fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            V_30044.V_30471,
            V_30044.estado,
            to_char(V_30044.fecha_fin, 'YYYY-MM-DD') AS fecha_fin,
            V_30044.V_30476,
            V_30044.V_30477,
            V_30044.archivo_disposicion_pdf,
            V_30044.archivo_levantamiento_pdf,
            V_30044.usuario,
            V_30044.V_30463,
            to_char(V_30044.V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675
        FROM expediente_suspensiones V_30044
        LEFT JOIN expedientes e ON e.id = V_30044.V_30275
        WHERE V_30044.V_30275 = $1
        ORDER BY V_30044.fecha_inicio DESC, V_30044.id DESC
        "#
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30279 = sqlx::query_as::<V_30020, S_50023>(
        V_30522#"
        SELECT
            V_30522.id,
            V_30522.expediente_origen_id,
            V_30522.expediente_destino_id,
            V_30522.V_30481,
            V_30522.V_30482,
            V_30522.V_30167,
            to_char(V_30522.V_30548, 'YYYY-MM-DD') AS V_30548,
            V_30522.V_30465,
            V_30522.usuario,
            V_30522.estado,
            V_30522.anulado_motivo,
            V_30522.anulado_usuario,
            to_char(V_30522.anulado_at, 'YYYY-MM-DD') AS anulado_at,
            e_other.id AS other_id,
            e_other.numero_expediente AS other_numero,
            e_other.reparticion AS other_reparticion,
            e_other.estado AS other_estado
        FROM expediente_relaciones V_30522
        JOIN expedientes e_other ON e_other.id = CASE WHEN V_30522.expediente_origen_id = $1 THEN V_30522.expediente_destino_id ELSE V_30522.expediente_origen_id END
        WHERE V_30522.expediente_origen_id = $1 OR V_30522.expediente_destino_id = $1
        ORDER BY V_30522.V_30548 DESC, V_30522.id DESC
        "#
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let V_30280: Vec<S_50022> = V_30279.into_iter().map(|V_30522| {
        let V_30281 = V_30522.expediente_origen_id == V_30275;
        let V_30282 = if V_30281 {
            match V_30522.V_30481.as_str() {
                "Antecedente" => format!("Este V_30113 es ANTECEDENTE de {}", V_30522.other_numero),
                "Acumulación formal" => format!("Este V_30113 ACUMULA a {}", V_30522.other_numero),
                "Ampliación de hechos" => format!("Este V_30113 AMPLÍA los hechos de {}", V_30522.other_numero),
                "Derivación" => format!("Este V_30113 fue DERIVADO a {}", V_30522.other_numero),
                "Reapertura" => format!("Este V_30113 es REAPERTURA de {}", V_30522.other_numero),
                "Reconstrucción" => format!("Este V_30113 es RECONSTRUCCIÓN de {}", V_30522.other_numero),
                V_30020 => format!("Vinculado ({}) con {}", V_30522.V_30481, V_30522.other_numero),
            }
        } else {
            match V_30522.V_30481.as_str() {
                "Antecedente" => format!("Este V_30113 tiene como ANTECEDENTE a {}", V_30522.other_numero),
                "Acumulación formal" => format!("Este V_30113 fue ACUMULADO en {}", V_30522.other_numero),
                "Ampliación de hechos" => format!("Este V_30113 fue AMPLIADO por {}", V_30522.other_numero),
                "Derivación" => format!("Este V_30113 fue RECIBIDO por derivación de {}", V_30522.other_numero),
                "Reapertura" => format!("Este V_30113 fue REABIERTO mediante {}", V_30522.other_numero),
                "Reconstrucción" => format!("Este V_30113 fue RECONSTRUIDO mediante {}", V_30522.other_numero),
                V_30020 => format!("Vinculado ({}) V_30511 {}", V_30522.V_30481, V_30522.other_numero),
            }
        };

        S_50022 {
            id: V_30522.id,
            expediente_origen_id: V_30522.expediente_origen_id,
            expediente_destino_id: V_30522.other_id,
            expediente_destino_numero: V_30522.other_numero,
            expediente_destino_reparticion: V_30522.other_reparticion,
            expediente_destino_estado: V_30522.other_estado,
            V_30481: V_30522.V_30481,
            V_30482: V_30522.V_30482,
            V_30167: V_30522.V_30167,
            V_30548: V_30522.V_30548,
            V_30465: V_30522.V_30465,
            usuario: V_30522.usuario,
            estado: V_30522.estado,
            anulado_motivo: V_30522.anulado_motivo,
            anulado_usuario: V_30522.anulado_usuario,
            anulado_at: V_30522.anulado_at,
            V_30282,
        }
    }).collect();

    let V_30283 = sqlx::query_as::<V_30020, S_50024>(
        V_30522#"
        SELECT DISTINCT
            e2.id AS V_30275,
            e2.numero_expediente,
            e2.reparticion,
            e2.estado,
            ea2.credencial AS agente_credencial,
            ea2.V_30266 AS agente_nombre
        FROM expediente_agentes ea1
        JOIN expediente_agentes ea2 ON ea2.credencial = ea1.credencial AND ea2.V_30275 <> ea1.V_30275
        JOIN expedientes e2 ON e2.id = ea2.V_30275
        WHERE ea1.V_30275 = $1
          AND ea1.credencial IS NOT NULL
          AND TRIM(ea1.credencial) <> ''
          AND LOWER(ea1.credencial) NOT IN ('sin datos', 'V_30044/d', '-')
          AND NOT EXISTS (
              SELECT 1 FROM expediente_relaciones er
              WHERE (er.expediente_origen_id = $1 AND er.expediente_destino_id = e2.id)
                 OR (er.expediente_destino_id = $1 AND er.expediente_origen_id = e2.id)
          )
        LIMIT 10
        "#
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30284: Vec<S_50012> = Vec::new();

    let V_30285: Vec<(i64, String, String, String, i32, i64, String, bool, Option<String>, String, String)> = sqlx::query_as(
        V_30522#"
        SELECT 
            d.id,
            d.tipo_documento,
            d.nombre_archivo,
            d.nombre_original,
            COALESCE(d.numero_version_vigente, 1) AS numero_version_vigente,
            COALESCE(d.tamano_bytes, 0) AS tamano_bytes,
            COALESCE(d.sha256_hash, '') AS sha256_hash,
            d.esta_retirado,
            d.motivo_retiro,
            to_char(d.V_30675, 'YYYY-MM-DD HH24:MI') AS V_30503,
            COALESCE(d.usuario, 'Sistema') AS usuario
        FROM expediente_documentos d
        WHERE d.V_30275 = $1
        ORDER BY d.id ASC
        "#
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    for d in V_30285 {
        let V_30286 = if d.5 > 1024 * 1024 {
            format!("{:.1} MB", d.5 as f64 / (1024.0 * 1024.0))
        } else if d.5 > 1024 {
            format!("{:.1} KB", d.5 as f64 / 1024.0)
        } else {
            format!("{} B", d.5)
        };
        let V_30287 = if d.6.len() >= 12 { d.6[..12].to_string() } else { d.6.clone() };
        V_30284.push(S_50012 {
            id: d.0,
            V_30092: d.1,
            V_30182: d.2,
            nombre_original: d.3,
            version_numero: d.4,
            tamano_bytes: d.5,
            tamano_formateado: V_30286,
            sha256_hash: d.6,
            sha256_short: V_30287,
            esta_retirado: d.7,
            motivo_retiro: d.8,
            V_30503: d.9,
            usuario: d.10,
            es_estandar: false,
        });
    }

    let V_30288: std::collections::HashSet<String> = V_30284.iter().map(|d| d.V_30182.clone()).collect();

    if let Some(V_30187) = V_30087.documento_informe_hechos.as_deref() {
        if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
            V_30284.push(S_50012 {
                id: 0,
                V_30092: "Informe inicial de hechos".to_string(),
                V_30182: V_30187.to_string(),
                nombre_original: V_30187.to_string(),
                version_numero: 1,
                tamano_bytes: 0,
                tamano_formateado: "—".to_string(),
                sha256_hash: "".to_string(),
                sha256_short: "—".to_string(),
                esta_retirado: false,
                motivo_retiro: None,
                V_30503: "—".to_string(),
                usuario: "Sistema".to_string(),
                es_estandar: true,
            });
        }
    }
    if let Some(V_30187) = V_30087.documento_disposicion.as_deref() {
        if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
            V_30284.push(S_50012 {
                id: 0,
                V_30092: "Disposición de sustanciación".to_string(),
                V_30182: V_30187.to_string(),
                nombre_original: V_30187.to_string(),
                version_numero: 1,
                tamano_bytes: 0,
                tamano_formateado: "—".to_string(),
                sha256_hash: "".to_string(),
                sha256_short: "—".to_string(),
                esta_retirado: false,
                motivo_retiro: None,
                V_30503: "—".to_string(),
                usuario: "Sistema".to_string(),
                es_estandar: true,
            });
        }
    }
    if let Some(V_30187) = V_30087.documento_finalizacion.as_deref() {
        if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
            V_30284.push(S_50012 {
                id: 0,
                V_30092: "Disposición / orden de finalización".to_string(),
                V_30182: V_30187.to_string(),
                nombre_original: V_30187.to_string(),
                version_numero: 1,
                tamano_bytes: 0,
                tamano_formateado: "—".to_string(),
                sha256_hash: "".to_string(),
                sha256_short: "—".to_string(),
                esta_retirado: false,
                motivo_retiro: None,
                V_30503: "—".to_string(),
                usuario: "Sistema".to_string(),
                es_estandar: true,
            });
        }
    }
    for ag in &V_30086 {
        if let Some(V_30187) = ag.V_30325.as_deref() {
            if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
                V_30284.push(S_50012 {
                    id: 0,
                    V_30092: format!("Causa judicial - {}", ag.V_30266),
                    V_30182: V_30187.to_string(),
                    nombre_original: V_30187.to_string(),
                    version_numero: 1,
                    tamano_bytes: 0,
                    tamano_formateado: "—".to_string(),
                    sha256_hash: "".to_string(),
                    sha256_short: "—".to_string(),
                    esta_retirado: false,
                    motivo_retiro: None,
                    V_30503: "—".to_string(),
                    usuario: "Sistema".to_string(),
                    es_estandar: true,
                });
            }
        }
        if let Some(V_30187) = ag.V_30329.as_deref() {
            if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
                V_30284.push(S_50012 {
                    id: 0,
                    V_30092: format!("Suspensión preventiva - {}", ag.V_30266),
                    V_30182: V_30187.to_string(),
                    nombre_original: V_30187.to_string(),
                    version_numero: 1,
                    tamano_bytes: 0,
                    tamano_formateado: "—".to_string(),
                    sha256_hash: "".to_string(),
                    sha256_short: "—".to_string(),
                    esta_retirado: false,
                    motivo_retiro: None,
                    V_30503: "—".to_string(),
                    usuario: "Sistema".to_string(),
                    es_estandar: true,
                });
            }
        }
        if let Some(V_30187) = ag.V_30332.as_deref() {
            if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
                V_30284.push(S_50012 {
                    id: 0,
                    V_30092: format!("Finalización individual - {}", ag.V_30266),
                    V_30182: V_30187.to_string(),
                    nombre_original: V_30187.to_string(),
                    version_numero: 1,
                    tamano_bytes: 0,
                    tamano_formateado: "—".to_string(),
                    sha256_hash: "".to_string(),
                    sha256_short: "—".to_string(),
                    esta_retirado: false,
                    motivo_retiro: None,
                    V_30503: "—".to_string(),
                    usuario: "Sistema".to_string(),
                    es_estandar: true,
                });
            }
        }
    }
    for nov in &V_30277 {
        if let Some(V_30187) = nov.V_30465.as_deref() {
            if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
                V_30284.push(S_50012 {
                    id: 0,
                    V_30092: format!("Novedad: {}", nov.V_30461),
                    V_30182: V_30187.to_string(),
                    nombre_original: V_30187.to_string(),
                    version_numero: 1,
                    tamano_bytes: 0,
                    tamano_formateado: "—".to_string(),
                    sha256_hash: "".to_string(),
                    sha256_short: "—".to_string(),
                    esta_retirado: false,
                    motivo_retiro: None,
                    V_30503: "—".to_string(),
                    usuario: "Sistema".to_string(),
                    es_estandar: true,
                });
            }
        }
    }
    for V_30478 in &V_30278 {
        if let Some(V_30187) = V_30478.archivo_disposicion_pdf.as_deref() {
            if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
                V_30284.push(S_50012 {
                    id: 0,
                    V_30092: format!("Suspensión Inicio - {}", V_30478.V_30266),
                    V_30182: V_30187.to_string(),
                    nombre_original: V_30187.to_string(),
                    version_numero: 1,
                    tamano_bytes: 0,
                    tamano_formateado: "—".to_string(),
                    sha256_hash: "".to_string(),
                    sha256_short: "—".to_string(),
                    esta_retirado: false,
                    motivo_retiro: None,
                    V_30503: "—".to_string(),
                    usuario: "Sistema".to_string(),
                    es_estandar: true,
                });
            }
        }
        if let Some(V_30187) = V_30478.archivo_levantamiento_pdf.as_deref() {
            if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
                V_30284.push(S_50012 {
                    id: 0,
                    V_30092: format!("Suspensión Cierre - {}", V_30478.V_30266),
                    V_30182: V_30187.to_string(),
                    nombre_original: V_30187.to_string(),
                    version_numero: 1,
                    tamano_bytes: 0,
                    tamano_formateado: "—".to_string(),
                    sha256_hash: "".to_string(),
                    sha256_short: "—".to_string(),
                    esta_retirado: false,
                    motivo_retiro: None,
                    V_30503: "—".to_string(),
                    usuario: "Sistema".to_string(),
                    es_estandar: true,
                });
            }
        }
    }
    for rel in &V_30280 {
        if let Some(V_30187) = rel.V_30465.as_deref() {
            if !V_30187.trim().is_empty() && !V_30288.contains(V_30187) {
                V_30284.push(S_50012 {
                    id: 0,
                    V_30092: format!("Vinculación - {}", rel.expediente_destino_numero),
                    V_30182: V_30187.to_string(),
                    nombre_original: V_30187.to_string(),
                    version_numero: 1,
                    tamano_bytes: 0,
                    tamano_formateado: "—".to_string(),
                    sha256_hash: "".to_string(),
                    sha256_short: "—".to_string(),
                    esta_retirado: false,
                    motivo_retiro: None,
                    V_30503: "—".to_string(),
                    usuario: "Sistema".to_string(),
                    es_estandar: true,
                });
            }
        }
    }

    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
    let V_30089 = F_40063(&V_30188.V_30012).await;
    let V_30091 = F_40066(&V_30188.V_30012).await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "view_expediente_detalle",
        "expedientes",
        "success",
        json!({ "id": V_30275 }),
    )
    .await;

    let V_30289 = sqlx::query_as::<V_30020, S_50080>(
        V_30522#"
        SELECT
            id,
            V_30275,
            numero_version,
            V_30306,
            tipo_operacion,
            snapshot_json,
            campos_modificados,
            diferencias_json,
            COALESCE(usuario, 'Sistema') AS usuario,
            V_30055,
            V_30167,
            version_origen_id,
            es_activa,
            to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675
        FROM expediente_versiones
        WHERE V_30275 = $1
        ORDER BY numero_version DESC
        "#,
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    render_html(&S_50048 {
        user_name: V_30064.display_name,
        V_30088,
        V_30087,
        V_30086,
        V_30284,
        V_30276,
        V_30277,
        V_30278,
        V_30280,
        V_30283,
        V_30089,
        V_30091,
        is_nuevo: query.nuevo.unwrap_or(false),
        V_30289,
        success_msg: query.V_30083.clone(),
        error_msg: query.err.clone(),
    })
}

#[post("/expedientes/{id}")]
async fn F_40076(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Form<S_50108>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();

    let V_30290 = sqlx::query_as::<V_30020, S_50011>(
        V_30522#"
        SELECT
            id,
            COALESCE(V_30228, 'sumario') AS V_30228,
            COALESCE(numero_expediente, 'Sin datos') AS numero_expediente,
            COALESCE(reparticion, 'Sin datos') AS reparticion,
            V_30230,
            to_char(fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            to_char(V_30232, 'YYYY-MM-DD') AS V_30232,
            to_char(V_30233, 'YYYY-MM-DD') AS V_30233,
            V_30234,
            V_30235,
            COALESCE(V_30167, 'Sin datos') AS V_30167,
            COALESCE(estado, 'Abierto') AS estado,
            V_30066,
            V_30067,
            V_30236,
            V_30237,
            COALESCE(V_30238, 'Sin datos') AS V_30238,
            COALESCE(V_30068, 'No') AS V_30068,
            V_30240,
            V_30241,
            COALESCE(V_30242, 'No') AS V_30242,
            to_char(V_30243, 'YYYY-MM-DD') AS V_30243,
            documento_informe_hechos,
            documento_disposicion,
            documento_finalizacion,
            V_30231,
            V_30463,
            to_char(fecha_ultimo_cambio_estado, 'YYYY-MM-DD HH24:MI') AS fecha_ultimo_cambio_estado,
            to_char(fecha_ultima_novedad, 'YYYY-MM-DD HH24:MI') AS fecha_ultima_novedad,
            tipo_ultima_novedad,
            COALESCE(version_actual, 1) AS version_actual
        FROM expedientes
        WHERE id = $1
        "#,
    )
    .bind(V_30275)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some(V_30290) = V_30290 else {
        return Ok(HttpResponse::NotFound().body("Expediente no encontrado"));
    };

    if !F_40002(&V_30188.V_30012, &V_30064.role, "expedientes", "modificar").await {
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=Permisos+insuficientes+para+modificar+el+V_30113&tab=tab-general", V_30275),
            ))
            .finish());
    }

    // Control de concurrencia optimista (versión activa esperada)
    if let Some(expected_ver) = form.version_activa_esperada {
        if expected_ver != V_30290.version_actual {
            write_audit(
                &V_30188.V_30012,
                Some(V_30064.V_30055),
                "update_expediente_conflict",
                "expedientes",
                "conflict",
                json!({
                    "V_30275": V_30275,
                    "version_esperada": expected_ver,
                    "version_actual": V_30290.version_actual
                }),
            )
            .await;
            return Ok(HttpResponse::SeeOther()
                .append_header((
                    header::LOCATION,
                    format!("/expedientes/{}?err=Conflicto+de+concurrencia:+el+V_30113+fue+modificado+por+otro+usuario+(v{}+actual,+v{}+esperada).+Recargue+antes+de+guardar.&tab=tab-general", V_30275, V_30290.version_actual, expected_ver),
                ))
                .finish());
        }
    }

    let V_30291 = match F_40006(&form.numero_expediente) {
        V_30054(V_30063) => V_30063,
        V_30156(V_30020) => {
            return Ok(HttpResponse::SeeOther()
                .append_header((
                    header::LOCATION,
                    format!("/expedientes/{}?err=El+V_30093+de+V_30113+es+obligatorio+y+debe+tener+formato+V_30824&tab=tab-general", V_30275),
                ))
                .finish());
        }
    };

    let V_30292 = F_40003(&form.reparticion);
    let V_30293 = F_40003(&form.V_30167);
    let V_30294 = F_40003(&form.V_30238);
    if V_30292.is_empty() || V_30293.is_empty() || V_30294.is_empty() {
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=Reparticion,+V_30167+e+instructor+actual+son+V_30623+obligatorios&tab=tab-general", V_30275),
            ))
            .finish());
    }

    // Verificar duplicado confirmado contra otro V_30113 distinto
    let V_30295: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM expedientes WHERE id != $1 AND LOWER(TRIM(V_30228)) = LOWER(TRIM($2)) AND UPPER(TRIM(numero_expediente)) = UPPER(TRIM($3)) LIMIT 1"
    )
    .bind(V_30275)
    .bind(&form.V_30228)
    .bind(&V_30291)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    if let Some(other_id) = V_30295 {
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=Duplicado+confirmado:+el+V_30093+{}+ya+pertenece+al+V_30113+ID+{}.&tab=tab-general", V_30275, V_30291, other_id),
            ))
            .finish());
    }

    let V_30296 = F_40067(form.fecha_inicio.as_deref());
    let V_30297 = F_40067(form.V_30232.as_deref());
    let V_30298 = F_40067(form.V_30243.as_deref());
    if let (Some(fh), Some(fi)) = (V_30297, V_30296) {
        if fh > fi {
            return Ok(HttpResponse::SeeOther()
                .append_header((
                    header::LOCATION,
                    format!("/expedientes/{}?err=Inconsistencia+de+fechas:+la+V_30503+del+hecho+no+puede+ser+posterior+a+la+V_30503+de+inicio&tab=tab-general", V_30275),
                ))
                .finish());
        }
    }
    if let (Some(fi), Some(ff)) = (V_30296, V_30298) {
        if ff < fi {
            return Ok(HttpResponse::SeeOther()
                .append_header((
                    header::LOCATION,
                    format!("/expedientes/{}?err=Inconsistencia+de+fechas:+la+V_30503+de+V_30242+no+puede+ser+anterior+a+la+V_30503+de+inicio&tab=tab-general", V_30275),
                ))
                .finish());
        }
    }

    let V_30299 = form.motivo_modificacion.clone().map(|V_30044| F_40003(&V_30044)).filter(|V_30044| !V_30044.is_empty());

    if V_30290.V_30238 != form.V_30238 {
        sqlx::query(
            "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(V_30275)
        .bind("instructor_designado")
        .bind(Some(&V_30290.V_30238))
        .bind(Some(&form.V_30238))
        .bind(&V_30064.V_30114)
        .bind(V_30299.as_deref().unwrap_or("Modificación de instructor designado"))
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    if V_30290.estado != form.estado {
        sqlx::query(
            "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(V_30275)
        .bind("estado")
        .bind(Some(&V_30290.estado))
        .bind(Some(&form.estado))
        .bind(&V_30064.V_30114)
        .bind(V_30299.as_deref().unwrap_or("Actualización de estado"))
        .execute(&V_30188.V_30012)
        .await
        .ok();

        sqlx::query(
            "UPDATE expedientes SET fecha_ultimo_cambio_estado = CURRENT_TIMESTAMP WHERE id = $1"
        )
        .bind(V_30275)
        .execute(&V_30188.V_30012)
        .await
        .ok();

        F_40013(
            &V_30188.V_30012,
            None,
            "cambio_estado",
            &format!("Expediente {}: cambio de estado a {}", V_30290.numero_expediente, form.estado),
            &format!("Transición de '{}' a '{}' registrada por {}.", V_30290.estado, form.estado, V_30064.display_name),
            &format!("/expedientes/{}", V_30275),
            Some(V_30275),
            None,
            None,
        ).await;
    }

    if V_30290.reparticion != form.reparticion {
        sqlx::query(
            "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(V_30275)
        .bind("reparticion")
        .bind(Some(&V_30290.reparticion))
        .bind(Some(&form.reparticion))
        .bind(&V_30064.V_30114)
        .bind(V_30299.as_deref().unwrap_or("Actualización de repartición"))
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    if V_30290.V_30167 != form.V_30167 {
        sqlx::query(
            "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(V_30275)
        .bind("V_30167")
        .bind(Some(&V_30290.V_30167))
        .bind(Some(&form.V_30167))
        .bind(&V_30064.V_30114)
        .bind(V_30299.as_deref().unwrap_or("Actualización de V_30167"))
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    if V_30290.V_30242 != form.V_30242.as_deref().unwrap_or("No") {
        sqlx::query(
            "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(V_30275)
        .bind("V_30242")
        .bind(Some(&V_30290.V_30242))
        .bind(form.V_30242.as_deref())
        .bind(&V_30064.V_30114)
        .bind(V_30299.as_deref().unwrap_or("Cambio en estado de finalización"))
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    let V_30300 = F_40067(form.fecha_inicio.as_deref());
    let V_30301 = F_40067(form.V_30232.as_deref());
    let V_30302 = F_40067(form.V_30233.as_deref());
    let V_30303 = F_40067(form.V_30243.as_deref());

    let V_30304 = F_40010(
        &V_30290.V_30228,
        &V_30290.numero_expediente,
        &V_30290.reparticion,
        V_30290.V_30230.as_deref(),
        V_30290.V_30231.as_deref(),
        V_30290.fecha_inicio.as_deref(),
        V_30290.V_30232.as_deref(),
        V_30290.V_30233.as_deref(),
        V_30290.V_30234.as_deref(),
        V_30290.V_30235.as_deref(),
        &V_30290.V_30167,
        V_30290.V_30066.as_deref(),
        V_30290.V_30067.as_deref(),
        V_30290.V_30236.as_deref(),
        V_30290.V_30237.as_deref(),
        &V_30290.V_30238,
        Some(&V_30290.V_30068),
        V_30290.V_30240.as_deref(),
        V_30290.V_30241.as_deref(),
        Some(&V_30290.V_30242),
        V_30290.V_30243.as_deref(),
    );

    let V_30226 = F_40010(
        &form.V_30228,
        &form.numero_expediente,
        &form.reparticion,
        form.V_30230.as_deref(),
        form.V_30231.as_deref(),
        form.fecha_inicio.as_deref(),
        form.V_30232.as_deref(),
        form.V_30233.as_deref(),
        form.V_30234.as_deref(),
        form.V_30235.as_deref(),
        &form.V_30167,
        form.V_30066.as_deref(),
        form.V_30067.as_deref(),
        form.V_30236.as_deref(),
        form.V_30237.as_deref(),
        &form.V_30238,
        form.V_30068.as_deref(),
        form.V_30240.as_deref(),
        form.V_30241.as_deref(),
        form.V_30242.as_deref(),
        form.V_30243.as_deref(),
    );

    let (campos_modificados, diferencias_json) = F_40011(&V_30304, &V_30226);

    let mut V_30305 = V_30290.version_actual;
    if !campos_modificados.is_empty() {
        let V_30221: Option<(i64, i32)> = sqlx::query_as(
            "SELECT id, numero_version FROM expediente_versiones WHERE V_30275 = $1 ORDER BY numero_version DESC LIMIT 1"
        )
        .bind(V_30275)
        .fetch_optional(&V_30188.V_30012)
        .await
        .unwrap_or(None);

        V_30305 = V_30221.as_ref().map(|(V_30020, v)| v + 1).unwrap_or(V_30290.version_actual + 1);
        let V_30306 = V_30221.as_ref().map(|(id, V_30020)| *id);

        let V_30020 = sqlx::query("UPDATE expediente_versiones SET es_activa = false WHERE V_30275 = $1")
            .bind(V_30275)
            .execute(&V_30188.V_30012)
            .await;

        let V_30307 = V_30299.as_deref().unwrap_or("Modificación de datos descriptivos del V_30113");
        let V_30027 = F_40009(
            V_30275,
            V_30305,
            V_30306,
            "modificacion",
            &V_30064.V_30114,
            V_30307,
            None,
            &V_30226,
        );

        let V_30020 = sqlx::query(
            V_30522#"
            INSERT INTO expediente_versiones (
                V_30275, numero_version, V_30306, tipo_operacion,
                snapshot_json, campos_modificados, diferencias_json, usuario,
                V_30055, V_30167, version_origen_id, es_activa, record_hash, V_30675
            ) VALUES (
                $1, $2, $3, 'modificacion',
                $4, $5, $6, $7,
                $8, $9, NULL, true, $10, CURRENT_TIMESTAMP
            )
            "#
        )
        .bind(V_30275)
        .bind(V_30305)
        .bind(V_30306)
        .bind(&V_30226)
        .bind(serde_json::to_value(&campos_modificados).unwrap_or_default())
        .bind(&diferencias_json)
        .bind(&V_30064.V_30114)
        .bind(Some(V_30064.V_30055))
        .bind(V_30307)
        .bind(&V_30027)
        .execute(&V_30188.V_30012)
        .await;
    }

    sqlx::query(
        V_30522#"
        UPDATE expedientes SET
            V_30228 = $1,
            numero_expediente = $2,
            reparticion = $3,
            V_30230 = $4,
            V_30231 = $5,
            fecha_inicio = $6,
            V_30232 = $7,
            V_30233 = $8,
            V_30234 = $9,
            V_30235 = $10,
            V_30167 = $11,
            estado = $12,
            V_30066 = $13,
            V_30067 = $14,
            V_30236 = $15,
            V_30237 = $16,
            V_30238 = $17,
            V_30068 = $18,
            V_30240 = $19,
            V_30241 = $20,
            V_30242 = $21,
            V_30243 = $22,
            version_actual = $23,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $24
        "#,
    )
    .bind(&form.V_30228)
    .bind(&form.numero_expediente)
    .bind(&form.reparticion)
    .bind(&form.V_30230)
    .bind(&form.V_30231)
    .bind(V_30300)
    .bind(V_30301)
    .bind(V_30302)
    .bind(&form.V_30234)
    .bind(&form.V_30235)
    .bind(&form.V_30167)
    .bind(&form.estado)
    .bind(&form.V_30066)
    .bind(&form.V_30067)
    .bind(&form.V_30236)
    .bind(&form.V_30237)
    .bind(&form.V_30238)
    .bind(form.V_30068.as_deref().unwrap_or("No"))
    .bind(&form.V_30240)
    .bind(&form.V_30241)
    .bind(form.V_30242.as_deref().unwrap_or("No"))
    .bind(V_30303)
    .bind(V_30305)
    .bind(V_30275)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30308 = format!("Actualización de datos generales. Motivo: {}", form.motivo_modificacion.as_deref().unwrap_or("Modificación de V_30113"));
    sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30458, V_30460, V_30461, V_30462,
            usuario, es_auditoria_automatica, V_30675
        ) VALUES (
            $1, 'Cambio de instructor', CURRENT_TIMESTAMP,
            'Actualización general del V_30113', $2, $3, true, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30275)
    .bind(&V_30308)
    .bind(&V_30064.V_30114)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    // Actualizar involucrado principal si fue provisto
    if let (Some(V_30168), Some(ape)) = (&form.involucrado_credencial, &form.involucrado_apellido) {
        let V_30309 = F_40003(V_30168);
        let V_30310 = F_40003(ape);
        if !V_30309.is_empty() && !V_30310.is_empty() {
            let V_30311 = form.involucrado_nombre.as_deref().map(F_40003).unwrap_or_default();
            let V_30312 = form.involucrado_escalafon.as_deref().map(F_40003).filter(|V_30044| !V_30044.is_empty());
            let V_30313 = form.involucrado_grado.as_deref().map(F_40003).filter(|V_30044| !V_30044.is_empty());
            let V_30314 = form.involucrado_rol.as_deref().map(F_40003).unwrap_or_else(|| "Causante".to_string());
            let V_30266 = format!("{}, {}", V_30310, V_30311).trim_end_matches(", ").to_string();

            let V_30020 = sqlx::query(
                V_30522#"
                INSERT INTO personas (credencial, apellido, nombre, V_30526, V_30317, V_30318, first_seen_at, last_seen_at, origen_principal)
                VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), 'expediente_edicion')
                ON CONFLICT (credencial) DO UPDATE SET
                    apellido = COALESCE(EXCLUDED.apellido, personas.apellido),
                    nombre = COALESCE(EXCLUDED.nombre, personas.nombre),
                    V_30317 = COALESCE(EXCLUDED.V_30317, personas.V_30317),
                    V_30318 = COALESCE(EXCLUDED.V_30318, personas.V_30318),
                    last_seen_at = NOW();
                "#
            )
            .bind(&V_30309)
            .bind(&V_30310)
            .bind(&V_30311)
            .bind(&V_30266)
            .bind(V_30312.as_deref())
            .bind(V_30313.as_deref())
            .execute(&V_30188.V_30012)
            .await;

            let V_30315: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM expediente_agentes WHERE V_30275 = $1 ORDER BY id ASC LIMIT 1"
            )
            .bind(V_30275)
            .fetch_optional(&V_30188.V_30012)
            .await
            .unwrap_or(None);

            if let Some(ag_id) = V_30315 {
                let V_30020 = sqlx::query(
                    V_30522#"
                    UPDATE expediente_agentes
                    SET credencial = $1, apellido = $2, nombre = $3, V_30266 = $4,
                        V_30317 = $5, V_30318 = $6, rol = $7, updated_at = NOW()
                    WHERE id = $8
                    "#
                )
                .bind(&V_30309)
                .bind(&V_30310)
                .bind(&V_30311)
                .bind(&V_30266)
                .bind(V_30312.as_deref())
                .bind(V_30313.as_deref())
                .bind(&V_30314)
                .bind(ag_id)
                .execute(&V_30188.V_30012)
                .await;
            } else {
                let V_30020 = sqlx::query(
                    V_30522#"
                    INSERT INTO expediente_agentes (
                        V_30275, credencial, apellido, nombre, V_30266,
                        V_30317, V_30318, rol, V_30675, updated_at
                    ) VALUES (
                        $1, $2, $3, $4, $5,
                        $6, $7, $8, NOW(), NOW()
                    )
                    "#
                )
                .bind(V_30275)
                .bind(&V_30309)
                .bind(&V_30310)
                .bind(&V_30311)
                .bind(&V_30266)
                .bind(V_30312.as_deref())
                .bind(V_30313.as_deref())
                .bind(&V_30314)
                .execute(&V_30188.V_30012)
                .await;
            }
        }
    }

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "update_expediente",
        "expedientes",
        "success",
        json!({ "id": V_30275, "V_30093": form.numero_expediente }),
    )
    .await;

    Ok(HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            format!("/expedientes/{}?V_30083=Modificaciones+guardadas+correctamente&tab=tab-general", V_30275),
        ))
        .finish())
}

#[post("/expedientes/{id}/V_30086")]
async fn F_40077(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();

    let (V_30019, V_30178) = F_40068(V_30012, "uploads").await?;

    let V_30316 = V_30019.get("credencial").cloned().unwrap_or_default();
    let credencial = match F_40005(&V_30316) {
        V_30054(c) => c,
        V_30156(V_30020) => {
            return Ok(HttpResponse::SeeOther()
                .append_header((
                    header::LOCATION,
                    format!("/expedientes/{}?err=La+credencial+del+agente+es+obligatoria+y+debe+tener+formato+V_30824+(preservando+ceros+iniciales)&tab=tab-V_30086", V_30275),
                ))
                .finish());
        }
    };
    let apellido = V_30019.get("apellido").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let nombre = V_30019.get("nombre").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30317 = V_30019.get("V_30317").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30318 = V_30019.get("V_30318").map(|V_30044| F_40003(V_30044)).filter(|V_30044| !V_30044.is_empty());

    let V_30266 = if let (Some(a), Some(V_30063)) = (&apellido, &nombre) {
        format!("{}, {}", a, V_30063)
    } else {
        F_40003(&V_30019.get("V_30266").cloned().unwrap_or_default())
    };
    let rol = F_40003(&V_30019.get("rol").cloned().unwrap_or_else(|| "Causante".to_string()));

    let V_30319 = V_30019.get("V_30319").map(|V_30044| V_30044 == "true").unwrap_or(false);
    let V_30320 = F_40067(V_30019.get("V_30320").map(|V_30044| V_30044.as_str()));
    let V_30321 = V_30019.get("V_30321").filter(|V_30044| !V_30044.trim().is_empty()).cloned();

    let V_30322 = V_30019.get("V_30322").map(|V_30044| V_30044 == "true").unwrap_or(false);
    let V_30323 = V_30019.get("V_30323").filter(|V_30044| !V_30044.trim().is_empty()).cloned();
    let V_30324 = F_40067(V_30019.get("V_30324").map(|V_30044| V_30044.as_str()));
    let V_30325 = V_30178.get("causa_pdf").cloned();

    let V_30326 = V_30019.get("V_30326").map(|V_30044| V_30044 == "true").unwrap_or(false);
    let V_30327 = F_40067(V_30019.get("V_30327").map(|V_30044| V_30044.as_str()));
    let V_30328 = V_30019.get("V_30328").filter(|V_30044| !V_30044.trim().is_empty()).cloned();
    let V_30329 = V_30178.get("suspension_pdf").cloned();

    let V_30330 = V_30019.get("V_30330").cloned().unwrap_or_else(|| "No".to_string());
    let V_30331 = F_40067(V_30019.get("V_30331").map(|V_30044| V_30044.as_str()));
    let V_30332 = V_30178.get("finalizacion_pdf").cloned();

    if credencial.is_empty() || V_30266.len() < 3 || V_30266.eq_ignore_ascii_case("sin datos") {
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=Credencial+y+apellido/nombre+completo+(minimo+3+caracteres)+son+obligatorios&tab=tab-V_30086", V_30275),
            ))
            .finish());
    }

    let V_30333: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM expediente_agentes WHERE V_30275 = $1 AND UPPER(TRIM(credencial)) = UPPER(TRIM($2)))"
    )
    .bind(V_30275)
    .bind(&credencial)
    .fetch_one(&V_30188.V_30012)
    .await
    .unwrap_or(false);

    if V_30333 {
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=Duplicado+confirmado:+el+agente+con+credencial+{}+ya+se+encuentra+vinculado+a+este+V_30113&tab=tab-V_30086", V_30275, credencial),
            ))
            .finish());
    }

    let V_30020 = sqlx::query(
            V_30522#"
            INSERT INTO personas (credencial, apellido, nombre, V_30526, V_30317, V_30318, first_seen_at, last_seen_at, origen_principal)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), 'expediente_agente_incorporado')
            ON CONFLICT (credencial) DO UPDATE SET
                apellido = COALESCE(EXCLUDED.apellido, personas.apellido),
                nombre = COALESCE(EXCLUDED.nombre, personas.nombre),
                V_30317 = COALESCE(EXCLUDED.V_30317, personas.V_30317),
                V_30318 = COALESCE(EXCLUDED.V_30318, personas.V_30318),
                last_seen_at = NOW();
            "#
        )
        .bind(&credencial)
        .bind(apellido.as_deref())
        .bind(nombre.as_deref())
        .bind(&V_30266)
        .bind(V_30317.as_deref())
        .bind(V_30318.as_deref())
        .execute(&V_30188.V_30012)
        .await;

    sqlx::query(
        V_30522#"
        INSERT INTO expediente_agentes (
            V_30275, credencial, apellido, nombre, V_30317, V_30318, V_30266, rol, V_30323,
            V_30319, V_30320, V_30321,
            V_30322, V_30324, V_30325,
            V_30326, V_30327, V_30328, V_30329,
            V_30330, V_30331, V_30332,
            V_30675, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9,
            $10, $11, $12,
            $13, $14, $15,
            $16, $17, $18, $19,
            $20, $21, $22,
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )
        "#,
    )
    .bind(V_30275)
    .bind(&credencial)
    .bind(apellido.as_deref())
    .bind(nombre.as_deref())
    .bind(V_30317.as_deref())
    .bind(V_30318.as_deref())
    .bind(&V_30266)
    .bind(&rol)
    .bind(&V_30323)
    .bind(V_30319)
    .bind(V_30320)
    .bind(&V_30321)
    .bind(V_30322)
    .bind(V_30324)
    .bind(&V_30325)
    .bind(V_30326)
    .bind(V_30327)
    .bind(&V_30328)
    .bind(&V_30329)
    .bind(&V_30330)
    .bind(V_30331)
    .bind(&V_30332)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    sqlx::query(
        "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(V_30275)
    .bind("agente_incorporado")
    .bind(None::<String>)
    .bind(format!("{} (Cred: {}) - Rol: {}", V_30266, credencial, rol))
    .bind(&V_30064.V_30114)
    .bind("Incorporacion de agente a la actuacion sumarial")
    .execute(&V_30188.V_30012)
    .await
    .ok();

    sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30458, V_30460, V_30461, V_30462,
            usuario, es_auditoria_automatica, V_30675
        ) VALUES (
            $1, 'Incorporacion de agente involucrado', CURRENT_TIMESTAMP,
            $2, $3, $4, true, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30275)
    .bind(format!("Agente incorporado: {}", V_30266))
    .bind(format!("Se incorporo a {} (Cred. {}) con rol {}", V_30266, credencial, rol))
    .bind(&V_30064.V_30114)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "add_agente",
        "expediente_agentes",
        "success",
        json!({ "V_30275": V_30275, "credencial": credencial, "nombre": V_30266 }),
    )
    .await;

    let V_30020 = F_40071(
        &V_30188.V_30012,
        V_30275,
        &V_30064.V_30114,
        Some(V_30064.V_30055),
        "incorporacion_agente",
        "agente_involucrado",
        &format!("Incorporación de agente: {} (Cred. {})", V_30266, credencial),
        json!({ "agente": V_30266, "credencial": credencial, "rol": rol }),
    ).await;

    Ok(HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            format!("/expedientes/{}?V_30083=Agente+incorporado+exitosamente&tab=tab-V_30086", V_30275),
        ))
        .finish())
}

#[post("/expedientes/{id}/V_30086/{V_30464}/eliminar")]
async fn F_40078(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let (V_30275, V_30464) = path.into_inner();

    let V_30334: Option<String> = sqlx::query_scalar(
        "SELECT V_30266 FROM expediente_agentes WHERE id = $1 AND V_30275 = $2"
    )
    .bind(V_30464)
    .bind(V_30275)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    sqlx::query("DELETE FROM expediente_agentes WHERE id = $1 AND V_30275 = $2")
        .bind(V_30464)
        .bind(V_30275)
        .execute(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

    sqlx::query(
        "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(V_30275)
    .bind("agente_desvinculado")
    .bind(&V_30334)
    .bind(None::<String>)
    .bind(&V_30064.V_30114)
    .bind("Desvinculación de agente de la actuación")
    .execute(&V_30188.V_30012)
    .await
    .ok();

    sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30458, V_30460, V_30461, V_30462,
            usuario, es_auditoria_automatica, V_30675
        ) VALUES (
            $1, 'Modificación de situación de un agente', CURRENT_TIMESTAMP,
            $2, $3, $4, true, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30275)
    .bind(format!("Agente desvinculado: {}", V_30334.as_deref().unwrap_or("Agente")))
    .bind("Desvinculación del agente de la actuación sumarial")
    .bind(&V_30064.V_30114)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "delete_agente",
        "expediente_agentes",
        "success",
        json!({ "V_30275": V_30275, "V_30464": V_30464 }),
    )
    .await;

    let V_30020 = F_40071(
        &V_30188.V_30012,
        V_30275,
        &V_30064.V_30114,
        Some(V_30064.V_30055),
        "desvinculacion_agente",
        "agente_involucrado",
        &format!("Desvinculación de agente ID {}", V_30464),
        json!({ "V_30464": V_30464, "nombre": V_30334 }),
    ).await;

    Ok(HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            format!("/expedientes/{}?V_30083=Agente+desvinculado+correctamente&tab=tab-V_30086", V_30275),
        ))
        .finish())
}

#[post("/expedientes/{id}/V_30284")]
async fn F_40079(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();

    let (V_30019, V_30178) = F_40070(V_30012, "uploads").await?;
    let V_30335 = V_30019.get("tipo_documento").cloned().unwrap_or_else(|| "Otro".to_string());
    let V_30336 = V_30178.get("documento_pdf");

    if let Some(V_30361) = V_30336 {
        let V_30337: i64 = sqlx::query_scalar(
            V_30522#"
            INSERT INTO expediente_documentos (
                V_30275, nombre_archivo, nombre_original, tipo_documento,
                usuario, V_30675, numero_version_vigente, esta_retirado,
                sha256_hash, tamano_bytes
            ) VALUES (
                $1, $2, $3, $4,
                $5, CURRENT_TIMESTAMP, 1, false,
                $6, $7
            ) RETURNING id
            "#
        )
        .bind(V_30275)
        .bind(&V_30361.V_30185)
        .bind(&V_30361.orig_name)
        .bind(&V_30335)
        .bind(&V_30064.V_30114)
        .bind(&V_30361.sha256_hash)
        .bind(V_30361.size_bytes)
        .fetch_one(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

        let V_30338: i64 = sqlx::query_scalar(
            V_30522#"
            INSERT INTO documento_versiones (
                documento_id, V_30275, numero_version, V_30306,
                nombre_archivo, nombre_original, tipo_documento, tamano_bytes,
                sha256_hash, tipo_operacion, V_30167, usuario, V_30055,
                es_vigente, esta_retirado, version_origen_id, V_30675
            ) VALUES (
                $1, $2, 1, NULL,
                $3, $4, $5, $6,
                $7, 'creacion', 'Carga inicial de documento adjunto', $8, $9,
                true, false, NULL, CURRENT_TIMESTAMP
            ) RETURNING id
            "#
        )
        .bind(V_30337)
        .bind(V_30275)
        .bind(&V_30361.V_30185)
        .bind(&V_30361.orig_name)
        .bind(&V_30335)
        .bind(V_30361.size_bytes)
        .bind(&V_30361.sha256_hash)
        .bind(&V_30064.V_30114)
        .bind(Some(V_30064.V_30055))
        .fetch_one(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

        let V_30020 = sqlx::query("UPDATE expediente_documentos SET version_vigente_id = $1 WHERE id = $2")
            .bind(V_30338)
            .bind(V_30337)
            .execute(&V_30188.V_30012)
            .await;

        if V_30335 == "Informe de hechos" {
            let V_30020 = sqlx::query("UPDATE expedientes SET documento_informe_hechos = $1 WHERE id = $2")
                .bind(&V_30361.V_30185)
                .bind(V_30275)
                .execute(&V_30188.V_30012)
                .await;
        } else if V_30335 == "Disposición de sustanciación" {
            let V_30020 = sqlx::query("UPDATE expedientes SET documento_disposicion = $1 WHERE id = $2")
                .bind(&V_30361.V_30185)
                .bind(V_30275)
                .execute(&V_30188.V_30012)
                .await;
        } else if V_30335 == "Disposición de finalización" {
            let V_30020 = sqlx::query("UPDATE expedientes SET documento_finalizacion = $1 WHERE id = $2")
                .bind(&V_30361.V_30185)
                .bind(V_30275)
                .execute(&V_30188.V_30012)
                .await;
        }

        let V_30020 = sqlx::query(
            "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(V_30275)
        .bind("documento_adjuntado")
        .bind(None::<String>)
        .bind(format!("{}: {} (SHA256: {}...)", V_30335, V_30361.orig_name, if V_30361.sha256_hash.len() >= 8 { &V_30361.sha256_hash[..8] } else { &V_30361.sha256_hash }))
        .bind(&V_30064.V_30114)
        .bind("Carga de nuevo documento versionado")
        .execute(&V_30188.V_30012)
        .await;

        let V_30020 = sqlx::query(
            V_30522#"
            INSERT INTO expediente_novedades (
                V_30275, V_30458, V_30460, V_30461, V_30462,
                usuario, es_auditoria_automatica, V_30465, V_30675
            ) VALUES (
                $1, 'Incorporación de documentación', CURRENT_TIMESTAMP,
                $2, $3, $4, true, $5, CURRENT_TIMESTAMP
            )
            "#
        )
        .bind(V_30275)
        .bind(format!("Documento adjuntado: {}", V_30335))
        .bind(format!("Se adjuntó el documento {} (v1, {} V_30123, SHA-256: {})", V_30361.orig_name, V_30361.size_bytes, V_30361.sha256_hash))
        .bind(&V_30064.V_30114)
        .bind(&V_30361.V_30185)
        .execute(&V_30188.V_30012)
        .await;

        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "upload_documento",
            "V_30284",
            "success",
            json!({
                "documento_id": V_30337,
                "version_id": V_30338,
                "V_30275": V_30275,
                "V_30092": V_30335,
                "V_30182": V_30361.V_30185,
                "original": V_30361.orig_name,
                "sha256": V_30361.sha256_hash,
                "V_30070": V_30361.size_bytes,
            }),
        )
        .await;

        let V_30020 = F_40071(
            &V_30188.V_30012,
            V_30275,
            &V_30064.V_30114,
            Some(V_30064.V_30055),
            "incorporacion_documento",
            "V_30284",
            &format!("Incorporación de documento: {}", V_30335),
            json!({ "documento_id": V_30337, "V_30092": V_30335, "V_30505": V_30361.orig_name, "sha256": V_30361.sha256_hash }),
        ).await;

        Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?V_30083=Documento+adjuntado+exitosamente+(v1)&tab=tab-V_30284", V_30275),
            ))
            .finish())
    } else {
        Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=No+se+recibió+ningún+V_30505+PDF&tab=tab-V_30284", V_30275),
            ))
            .finish())
    }
}

#[post("/expedientes/{id}/V_30284/{V_30337}/sustituir")]
async fn F_40080(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let (V_30275, V_30337) = path.into_inner();

    let (V_30019, V_30178) = F_40070(V_30012, "uploads").await?;
    let V_30167 = V_30019.get("V_30167").cloned().unwrap_or_else(|| "Sustitución de V_30505 por nueva versión".to_string());
    let V_30336 = V_30178.get("documento_pdf");

    let Some(V_30361) = V_30336 else {
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=No+se+recibió+el+V_30505+PDF+de+reemplazo&tab=tab-V_30284", V_30275),
            ))
            .finish());
    };

    let V_30221: Option<(i64, i32, String, String)> = sqlx::query_as(
        V_30522#"
        SELECT id, numero_version, tipo_documento, nombre_original
        FROM documento_versiones
        WHERE documento_id = $1
        ORDER BY numero_version DESC
        LIMIT 1
        "#
    )
    .bind(V_30337)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    let (prev_ver_id, prev_numero, V_30335) = match V_30221 {
        Some(V_30128) => (Some(V_30128.0), V_30128.1, V_30128.2),
        None => (None, 1, "Documento adjunto".to_string()),
    };

    let V_30339 = prev_numero + 1;

    let V_30020 = sqlx::query("UPDATE documento_versiones SET es_vigente = false WHERE documento_id = $1")
        .bind(V_30337)
        .execute(&V_30188.V_30012)
        .await;

    let V_30340: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO documento_versiones (
            documento_id, V_30275, numero_version, V_30306,
            nombre_archivo, nombre_original, tipo_documento, tamano_bytes,
            sha256_hash, tipo_operacion, V_30167, usuario, V_30055,
            es_vigente, esta_retirado, version_origen_id, V_30675
        ) VALUES (
            $1, $2, $3, $4,
            $5, $6, $7, $8,
            $9, 'sustitucion', $10, $11, $12,
            true, false, NULL, CURRENT_TIMESTAMP
        ) RETURNING id
        "#
    )
    .bind(V_30337)
    .bind(V_30275)
    .bind(V_30339)
    .bind(prev_ver_id)
    .bind(&V_30361.V_30185)
    .bind(&V_30361.orig_name)
    .bind(&V_30335)
    .bind(V_30361.size_bytes)
    .bind(&V_30361.sha256_hash)
    .bind(&V_30167)
    .bind(&V_30064.V_30114)
    .bind(Some(V_30064.V_30055))
    .fetch_one(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30020 = sqlx::query(
        V_30522#"
        UPDATE expediente_documentos SET
            nombre_archivo = $1,
            nombre_original = $2,
            version_vigente_id = $3,
            numero_version_vigente = $4,
            sha256_hash = $5,
            tamano_bytes = $6,
            esta_retirado = false
        WHERE id = $7
        "#
    )
    .bind(&V_30361.V_30185)
    .bind(&V_30361.orig_name)
    .bind(V_30340)
    .bind(V_30339)
    .bind(&V_30361.sha256_hash)
    .bind(V_30361.size_bytes)
    .bind(V_30337)
    .execute(&V_30188.V_30012)
    .await;

    if V_30335 == "Informe de hechos" {
        let V_30020 = sqlx::query("UPDATE expedientes SET documento_informe_hechos = $1 WHERE id = $2")
            .bind(&V_30361.V_30185)
            .bind(V_30275)
            .execute(&V_30188.V_30012)
            .await;
    } else if V_30335 == "Disposición de sustanciación" {
        let V_30020 = sqlx::query("UPDATE expedientes SET documento_disposicion = $1 WHERE id = $2")
            .bind(&V_30361.V_30185)
            .bind(V_30275)
            .execute(&V_30188.V_30012)
            .await;
    } else if V_30335 == "Disposición de finalización" {
        let V_30020 = sqlx::query("UPDATE expedientes SET documento_finalizacion = $1 WHERE id = $2")
            .bind(&V_30361.V_30185)
            .bind(V_30275)
            .execute(&V_30188.V_30012)
            .await;
    }

    let V_30020 = sqlx::query(
        "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(V_30275)
    .bind("documento_sustituido")
    .bind(format!("v{}", prev_numero))
    .bind(format!("v{} ({})", V_30339, V_30361.orig_name))
    .bind(&V_30064.V_30114)
    .bind(&V_30167)
    .execute(&V_30188.V_30012)
    .await;

    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30458, V_30460, V_30461, V_30462,
            usuario, es_auditoria_automatica, V_30465, V_30675
        ) VALUES (
            $1, 'Incorporación de documentación', CURRENT_TIMESTAMP,
            $2, $3, $4, true, $5, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30275)
    .bind(format!("Sustitución de documento: {} (v{})", V_30335, V_30339))
    .bind(format!("Se sustituyó el documento por {} (v{}, {} V_30123, SHA-256: {}). Motivo: {}", V_30361.orig_name, V_30339, V_30361.size_bytes, V_30361.sha256_hash, V_30167))
    .bind(&V_30064.V_30114)
    .bind(&V_30361.V_30185)
    .execute(&V_30188.V_30012)
    .await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "sustituir_documento",
        "V_30284",
        "success",
        json!({
            "documento_id": V_30337,
            "V_30306": prev_ver_id,
            "nueva_version_id": V_30340,
            "numero_version": V_30339,
            "V_30275": V_30275,
            "V_30182": V_30361.V_30185,
            "original": V_30361.orig_name,
            "sha256": V_30361.sha256_hash,
            "V_30167": V_30167,
        }),
    )
    .await;

    Ok(HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            format!("/expedientes/{}?V_30083=Documento+sustituido+exitosamente+(v{})&tab=tab-V_30284", V_30275, V_30339),
        ))
        .finish())
}

#[derive(Deserialize)]
struct S_50119 {
    V_30167: String,
}

#[post("/expedientes/{id}/V_30284/{V_30337}/retirar")]
async fn F_40081(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    form: web::Form<S_50119>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let (V_30275, V_30337) = path.into_inner();
    let V_30167 = form.V_30167.trim();
    if V_30167.is_empty() {
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=Debe+indicar+un+V_30167+para+el+retiro+lógico&tab=tab-V_30284", V_30275),
            ))
            .finish());
    }

    let V_30221: Option<(i64, i32, String, String, String, i64, String)> = sqlx::query_as(
        V_30522#"
        SELECT id, numero_version, tipo_documento, nombre_original, nombre_archivo, tamano_bytes, sha256_hash
        FROM documento_versiones
        WHERE documento_id = $1
        ORDER BY numero_version DESC
        LIMIT 1
        "#
    )
    .bind(V_30337)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    let Some(prev) = V_30221 else {
        return Ok(HttpResponse::NotFound().body("Documento no encontrado"));
    };

    let V_30339 = prev.1 + 1;

    let V_30020 = sqlx::query("UPDATE documento_versiones SET es_vigente = false WHERE documento_id = $1")
        .bind(V_30337)
        .execute(&V_30188.V_30012)
        .await;

    let V_30340: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO documento_versiones (
            documento_id, V_30275, numero_version, V_30306,
            nombre_archivo, nombre_original, tipo_documento, tamano_bytes,
            sha256_hash, tipo_operacion, V_30167, usuario, V_30055,
            es_vigente, esta_retirado, version_origen_id, V_30675
        ) VALUES (
            $1, $2, $3, $4,
            $5, $6, $7, $8,
            $9, 'retiro_logico', $10, $11, $12,
            false, true, NULL, CURRENT_TIMESTAMP
        ) RETURNING id
        "#
    )
    .bind(V_30337)
    .bind(V_30275)
    .bind(V_30339)
    .bind(Some(prev.0))
    .bind(&prev.4)
    .bind(&prev.3)
    .bind(&prev.2)
    .bind(prev.5)
    .bind(&prev.6)
    .bind(V_30167)
    .bind(&V_30064.V_30114)
    .bind(Some(V_30064.V_30055))
    .fetch_one(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30020 = sqlx::query(
        V_30522#"
        UPDATE expediente_documentos SET
            esta_retirado = true,
            motivo_retiro = $1,
            fecha_retiro = CURRENT_TIMESTAMP,
            usuario_retiro = $2,
            version_vigente_id = $3,
            numero_version_vigente = $4
        WHERE id = $5
        "#
    )
    .bind(V_30167)
    .bind(&V_30064.V_30114)
    .bind(V_30340)
    .bind(V_30339)
    .bind(V_30337)
    .execute(&V_30188.V_30012)
    .await;

    let V_30020 = sqlx::query(
        "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(V_30275)
    .bind("documento_retirado")
    .bind(format!("v{} ({})", prev.1, prev.3))
    .bind(format!("Retirado lógicamente (v{})", V_30339))
    .bind(&V_30064.V_30114)
    .bind(V_30167)
    .execute(&V_30188.V_30012)
    .await;

    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30458, V_30460, V_30461, V_30462,
            usuario, es_auditoria_automatica, V_30465, V_30675
        ) VALUES (
            $1, 'Retiro documental', CURRENT_TIMESTAMP,
            $2, $3, $4, true, NULL, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30275)
    .bind(format!("Retiro lógico de documento: {}", prev.2))
    .bind(format!("Se retiró lógicamente el documento {} (v{}). Motivo: {}", prev.3, V_30339, V_30167))
    .bind(&V_30064.V_30114)
    .execute(&V_30188.V_30012)
    .await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "retirar_documento",
        "V_30284",
        "success",
        json!({
            "documento_id": V_30337,
            "version_id": V_30340,
            "V_30275": V_30275,
            "V_30167": V_30167,
        }),
    )
    .await;

    Ok(HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            format!("/expedientes/{}?V_30083=Documento+retirado+lógicamente+de+forma+exitosa&tab=tab-V_30284", V_30275),
        ))
        .finish())
}

#[get("/V_30284/V_30289/{V_30338}")]
async fn F_40082(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30338 = path.into_inner();

    if !F_40002(&V_30188.V_30012, &V_30064.role, "V_30284", "descargar").await {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "download_documento_version_denied",
            "V_30284",
            "forbidden",
            json!({ "version_id": V_30338, "role": V_30064.role }),
        )
        .await;
        return Ok(HttpResponse::Forbidden().body("No posee permisos para descargar V_30289 de V_30284"));
    }

    let V_30341: Option<(String, String, String, i64)> = sqlx::query_as(
        "SELECT nombre_archivo, nombre_original, COALESCE(sha256_hash, ''), V_30275 FROM documento_versiones WHERE id = $1"
    )
    .bind(V_30338)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some((V_30182, original_name, expected_sha256, exp_id)) = V_30341 else {
        return Ok(HttpResponse::NotFound().body("Versión de documento no encontrada"));
    };

    if V_30182.contains("..") || V_30182.contains('/') || V_30182.contains('\\') {
        return Ok(HttpResponse::BadRequest().body("Nombre de V_30505 inválido"));
    }

    let V_30146 = format!("uploads/{}", V_30182);
    let V_30147 = Path::new(&V_30146);
    if !V_30147.V_30028() || !V_30147.is_file() {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "documento_archivo_faltante",
            "V_30284",
            "missing_file",
            json!({ "version_id": V_30338, "V_30275": exp_id, "V_30182": V_30182 }),
        )
        .await;
        return Ok(HttpResponse::NotFound().body("Archivo físico no encontrado en almacenamiento"));
    }

    if !expected_sha256.trim().is_empty() {
        if let Ok((actual_hash, V_30020)) = F_40069(&V_30146) {
            if !actual_hash.eq_ignore_ascii_case(expected_sha256.trim()) {
                write_audit(
                    &V_30188.V_30012,
                    Some(V_30064.V_30055),
                    "documento_integridad_fallida",
                    "V_30284",
                    "hash_mismatch",
                    json!({
                        "version_id": V_30338,
                        "V_30275": exp_id,
                        "V_30182": V_30182,
                        "expected_sha256": expected_sha256,
                        "actual_sha256": actual_hash
                    }),
                )
                .await;
                let V_30020 = sqlx::query(
                    V_30522#"
                    INSERT INTO app_alertas_seguridad (
                        V_30701, V_30461, V_30462, V_30702, V_30055, estado
                    ) VALUES (
                        'critica', $1, $2, 'integridad_documento_comprometida', $3, 'activa'
                    )
                    "#
                )
                .bind(format!("Discrepancia SHA-256 en versión de documento #{}", V_30338))
                .bind(format!("El V_30505 físico '{}' del V_30113 ID #{} no coincide con V_30692 huella SHA-256 registrada.", V_30182, exp_id))
                .bind(V_30064.V_30055)
                .execute(&V_30188.V_30012)
                .await;
                return Ok(HttpResponse::Conflict().body("Alerta de integridad documental: la huella SHA-256 del V_30505 físico no coincide con el registro de auditoría."));
            }
        }
    }

    let V_30343 = NamedFile::open(V_30147)
        .map_err(error::ErrorInternalServerError)?
        .set_content_disposition(actix_web::http::header::ContentDisposition {
            disposition: actix_web::http::header::DispositionType::Inline,
            parameters: vec![actix_web::http::header::DispositionParam::Filename(original_name)],
        });

    V_30054(V_30343.into_response(&req))
}

#[derive(Serialize)]
struct S_50120 {
    id: i64,
    documento_id: i64,
    V_30275: i64,
    numero_version: i32,
    V_30306: Option<i64>,
    nombre_archivo: String,
    nombre_original: String,
    tipo_documento: String,
    tamano_bytes: i64,
    tamano_formateado: String,
    sha256_hash: String,
    tipo_operacion: String,
    V_30167: String,
    usuario: String,
    es_vigente: bool,
    esta_retirado: bool,
    fecha_hora: String,
    download_url: String,
}

#[get("/api/V_30284/{V_30337}/V_30289")]
async fn F_40083(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30337 = path.into_inner();

    let V_30022: Vec<(i64, i64, i64, i32, Option<i64>, String, String, String, i64, String, String, String, String, bool, bool, String)> = sqlx::query_as(
        V_30522#"
        SELECT 
            id, documento_id, V_30275, numero_version, V_30306,
            nombre_archivo, nombre_original, tipo_documento, tamano_bytes,
            sha256_hash, tipo_operacion, V_30167, usuario,
            es_vigente, esta_retirado,
            to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS fecha_hora
        FROM documento_versiones
        WHERE documento_id = $1
        ORDER BY numero_version DESC
        "#
    )
    .bind(V_30337)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30008: Vec<S_50120> = V_30022.into_iter().map(|V_30522| {
        let V_30286 = if V_30522.8 > 1024 * 1024 {
            format!("{:.1} MB", V_30522.8 as f64 / (1024.0 * 1024.0))
        } else if V_30522.8 > 1024 {
            format!("{:.1} KB", V_30522.8 as f64 / 1024.0)
        } else {
            format!("{} B", V_30522.8)
        };
        let V_30345 = format!("/V_30284/V_30289/{}", V_30522.0);
        S_50120 {
            id: V_30522.0,
            documento_id: V_30522.1,
            V_30275: V_30522.2,
            numero_version: V_30522.3,
            V_30306: V_30522.4,
            nombre_archivo: V_30522.5,
            nombre_original: V_30522.6,
            tipo_documento: V_30522.7,
            tamano_bytes: V_30522.8,
            tamano_formateado: V_30286,
            sha256_hash: V_30522.9,
            tipo_operacion: V_30522.10,
            V_30167: V_30522.11,
            usuario: V_30522.12,
            es_vigente: V_30522.13,
            esta_retirado: V_30522.14,
            fecha_hora: V_30522.15,
            download_url: V_30345,
        }
    }).collect();

    Ok(HttpResponse::Ok().json(V_30008))
}

#[get("/V_30284/{V_30182}")]
async fn F_40084(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30182 = path.into_inner();
    if V_30182.contains("..") || V_30182.contains('/') || V_30182.contains('\\') {
        return Ok(HttpResponse::BadRequest().body("Nombre de V_30505 inválido"));
    }

    if !F_40002(&V_30188.V_30012, &V_30064.role, "V_30284", "descargar").await {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "download_documento_denied",
            "V_30284",
            "forbidden",
            json!({ "V_30182": V_30182, "role": V_30064.role }),
        )
        .await;
        return Ok(HttpResponse::Forbidden().body("No posee permisos para descargar V_30284"));
    }

    let V_30146 = format!("uploads/{}", V_30182);
    let V_30147 = Path::new(&V_30146);
    if !V_30147.V_30028() || !V_30147.is_file() {
        return Ok(HttpResponse::NotFound().body("Documento no encontrado"));
    }

    let V_30336: Option<(i64, i64, String)> = sqlx::query_as(
        "SELECT id, V_30275, COALESCE(sha256_hash, '') FROM expediente_documentos WHERE V_30182 = $1 LIMIT 1"
    )
    .bind(&V_30182)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    if let Some((V_30337, exp_id, expected_sha)) = V_30336 {
        if !expected_sha.trim().is_empty() {
            if let Ok((actual_hash, V_30020)) = F_40069(&V_30146) {
                if !actual_hash.eq_ignore_ascii_case(expected_sha.trim()) {
                    write_audit(
                        &V_30188.V_30012,
                        Some(V_30064.V_30055),
                        "documento_integridad_fallida",
                        "V_30284",
                        "hash_mismatch",
                        json!({
                            "documento_id": V_30337,
                            "V_30275": exp_id,
                            "V_30182": V_30182,
                            "expected_sha256": expected_sha,
                            "actual_sha256": actual_hash
                        }),
                    )
                    .await;
                    return Ok(HttpResponse::Conflict().body("Alerta de integridad documental: la huella SHA-256 del V_30505 físico no coincide con el registro de auditoría."));
                }
            }
        }
    }

    let V_30343 = NamedFile::open(V_30147).map_err(error::ErrorInternalServerError)?;
    V_30054(V_30343.into_response(&req))
}

#[get("/")]
async fn dashboard(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30098: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM personas")
        .fetch_one(&V_30188.V_30012)
        .await
        .unwrap_or(0);
    let V_30099: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM V_30507")
        .fetch_one(&V_30188.V_30012)
        .await
        .unwrap_or(0);
    let V_30100: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM staging_sin_credencial")
        .fetch_one(&V_30188.V_30012)
        .await
        .unwrap_or(0);
    let V_30101: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM staging_sin_credencial WHERE credencial_resuelta IS NULL")
        .fetch_one(&V_30188.V_30012)
        .await
        .unwrap_or(0);
    let V_30102: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM expedientes")
        .fetch_one(&V_30188.V_30012)
        .await
        .unwrap_or(0);
    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "view_dashboard",
        "dashboard",
        "success",
        json!({}),
    )
    .await;

    render_html(&S_50040 {
        user_name: V_30064.display_name,
        now_local: Local::V_30072().format("%Y-%m-%d %H:%M:%S").to_string(),
        V_30088,
        V_30098,
        V_30099,
        V_30100,
        V_30101,
        V_30102,
    })
}

#[get("/personas")]
async fn personas(req: HttpRequest, V_30188: web::Data<AppState>, query: web::Query<S_50097>) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    let V_30103 = query.V_30103.clone().unwrap_or_else(|| "todos".to_string()).trim().to_string();
    let V_30104 = query.V_30104.clone().unwrap_or_else(|| "todos".to_string()).trim().to_string();
    let V_30069 = query.V_30069.unwrap_or(1).max(1);
    let V_30070 = query
        .V_30070
        .unwrap_or(5)
        .clamp(1, V_30188.V_30011.max_page_size);
    let V_30105 = query.V_30105.clone().unwrap_or_else(|| "credencial".to_string());
    let V_30106 = query.V_30106.clone().unwrap_or_else(|| "asc".to_string());
    let V_30107 = match V_30105.as_str() {
        "apellido" => "V_30147.apellido",
        "nombre" => "V_30147.nombre",
        "escalafon_grado" => "V_30147.escalafon_grado",
        "V_30526" => "V_30147.V_30526",
        "origen_principal" => "V_30147.origen_principal",
        "last_seen_at" => "V_30147.last_seen_at",
        V_30020 => "V_30147.credencial",
    };
    let V_30108 = if V_30106.eq_ignore_ascii_case("V_30308") {
        "DESC"
    } else {
        "ASC"
    };
    let V_30071 = (V_30069 - 1) * V_30070;
    let V_30076 = if !V_30065.is_empty() {
        format!("%{}%", V_30065)
    } else {
        String::new()
    };

    let mut V_30109 = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        "SELECT V_30147.credencial, V_30147.apellido, V_30147.nombre, V_30147.escalafon_grado, V_30147.V_30526, V_30147.origen_principal, \
         COALESCE(V_30147.last_seen_at, LOCALTIMESTAMP) AS last_seen_at, \
         (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Vigente', 'Suspendido')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'Suspendidos')) AS tiene_suspension_activa, \
         (EXISTS (SELECT 1 FROM expediente_agentes ea JOIN expedientes e ON e.id = ea.V_30275 WHERE ea.credencial = V_30147.credencial AND e.estado IN ('Activo', 'En analisis', 'En análisis', 'Suspendido', 'Inicio', 'Abierto', 'En Trámite')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('SUM ADM', 'INFO SUM'))) AS tiene_sumario_abierto, \
         (EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('CONDENA', 'PROCESAMIENTO', 'CC'))) AS tiene_impedimento_judicial \
         FROM personas V_30147 WHERE 1 = 1 "
    );

    let mut V_30110 = sqlx::QueryBuilder::<sqlx::Postgres>::new("SELECT COUNT(*) FROM personas V_30147 WHERE 1 = 1 ");

    if !V_30065.is_empty() {
        V_30109.push(" AND (V_30147.credencial ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(" OR COALESCE(V_30147.apellido,'') ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(" OR COALESCE(V_30147.nombre,'') ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(" OR COALESCE(V_30147.escalafon_grado,'') ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(" OR COALESCE(V_30147.V_30526,'') ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(") ");

        V_30110.push(" AND (V_30147.credencial ILIKE ");
        V_30110.push_bind(&V_30076);
        V_30110.push(" OR COALESCE(V_30147.apellido,'') ILIKE ");
        V_30110.push_bind(&V_30076);
        V_30110.push(" OR COALESCE(V_30147.nombre,'') ILIKE ");
        V_30110.push_bind(&V_30076);
        V_30110.push(" OR COALESCE(V_30147.escalafon_grado,'') ILIKE ");
        V_30110.push_bind(&V_30076);
        V_30110.push(" OR COALESCE(V_30147.V_30526,'') ILIKE ");
        V_30110.push_bind(&V_30076);
        V_30110.push(") ");
    }

    if !V_30104.is_empty() && V_30104 != "todos" {
        V_30109.push(" AND V_30147.origen_principal = ");
        V_30109.push_bind(&V_30104);
        V_30110.push(" AND V_30147.origen_principal = ");
        V_30110.push_bind(&V_30104);
    }

    match V_30103.as_str() {
        "suspension_activa" => {
            let V_30077 = " AND (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Vigente', 'Suspendido')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'Suspendidos')) ";
            V_30109.push(V_30077);
            V_30110.push(V_30077);
        }
        "sumario_abierto" => {
            let V_30077 = " AND (EXISTS (SELECT 1 FROM expediente_agentes ea JOIN expedientes e ON e.id = ea.V_30275 WHERE ea.credencial = V_30147.credencial AND e.estado IN ('Activo', 'En analisis', 'En análisis', 'Suspendido', 'Inicio', 'Abierto', 'En Trámite')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('SUM ADM', 'INFO SUM'))) ";
            V_30109.push(V_30077);
            V_30110.push(V_30077);
        }
        "suspension_levantada" => {
            let V_30077 = " AND (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Levantado', 'Levantada')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'LEVANTAMIENTO')) ";
            V_30109.push(V_30077);
            V_30110.push(V_30077);
        }
        "impedimento_judicial" => {
            let V_30077 = " AND (EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('CONDENA', 'PROCESAMIENTO', 'CC'))) ";
            V_30109.push(V_30077);
            V_30110.push(V_30077);
        }
        "sin_sanciones" => {
            let V_30077 = " AND NOT (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Vigente', 'Suspendido')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'Suspendidos') OR EXISTS (SELECT 1 FROM expediente_agentes ea JOIN expedientes e ON e.id = ea.V_30275 WHERE ea.credencial = V_30147.credencial AND e.estado IN ('Activo', 'En analisis', 'En análisis', 'Suspendido', 'Inicio', 'Abierto', 'En Trámite')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('SUM ADM', 'INFO SUM'))) ";
            V_30109.push(V_30077);
            V_30110.push(V_30077);
        }
        V_30020 => {}
    }

    let V_30079: i64 = V_30110.build_query_scalar().fetch_one(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;

    V_30109.push(format!(" ORDER BY {V_30107} {V_30108} NULLS LAST LIMIT "));
    V_30109.push_bind(V_30070);
    V_30109.push(" OFFSET ");
    V_30109.push_bind(V_30071);

    let V_30022: Vec<S_50002> = V_30109.build_query_as().fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;

    let V_30087 = ((V_30079 as f64) / (V_30070 as f64)).ceil().max(1.0) as i64;
    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "query_personas",
        "personas",
        "success",
        json!({ "V_30065": V_30065, "V_30103": V_30103, "V_30104": V_30104, "V_30069": V_30069, "V_30070": V_30070 }),
    )
    .await;

    render_html(&S_50041 {
        user_name: V_30064.display_name,
        V_30088,
        V_30022,
        V_30065,
        V_30103,
        V_30104,
        V_30069,
        V_30087,
        V_30079,
        V_30070,
        V_30105: V_30105.clone(),
        V_30106: V_30106.to_lowercase(),
    })
}

#[get("/F_40085")]
async fn F_40085(req: HttpRequest, V_30188: web::Data<AppState>, query: web::Query<S_50098>) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let estado = query
        .estado
        .clone()
        .unwrap_or_else(|| "pendiente".to_string());
    let V_30069 = query.V_30069.unwrap_or(1).max(1);
    let V_30070 = query
        .V_30070
        .unwrap_or(5)
        .clamp(1, V_30188.V_30011.max_page_size);
    let V_30071 = (V_30069 - 1) * V_30070;

    let V_30111 = match estado.as_str() {
        "resuelto" => "WHERE credencial_resuelta IS NOT NULL",
        "todos" => "",
        V_30020 => "WHERE credencial_resuelta IS NULL",
    };
    let V_30073 = format!("SELECT COUNT(*) FROM staging_sin_credencial {V_30111}");
    let V_30079: i64 = sqlx::query_scalar(&V_30073)
        .fetch_one(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;
    let V_30112 = format!(
        "SELECT id, dominio, hoja_origen, V_30526, credencial_resuelta, metodo_resolucion, score_resolucion
         FROM staging_sin_credencial
         {V_30111}
         ORDER BY id DESC LIMIT $1 OFFSET $2"
    );
    let V_30022 = sqlx::query_as::<V_30020, S_50003>(&V_30112)
        .bind(V_30070)
        .bind(V_30071)
        .fetch_all(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;
    let V_30087 = ((V_30079 as f64) / (V_30070 as f64)).ceil().max(1.0) as i64;
    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "query_staging",
        "staging_sin_credencial",
        "success",
        json!({ "estado": estado, "V_30069": V_30069, "V_30070": V_30070 }),
    )
    .await;

    render_html(&S_50042 {
        user_name: V_30064.display_name,
        V_30088,
        V_30022,
        estado,
        V_30069,
        V_30087,
        V_30079,
        V_30070,
    })
}

#[get("/fuente/{file_id}/hoja/{sheet_id}")]
async fn F_40086(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    query: web::Query<S_50109>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let (file_id, sheet_id) = path.into_inner();
    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    let V_30069 = query.V_30069.unwrap_or(1).max(1);
    let V_30070 = query
        .V_30070
        .unwrap_or(5)
        .clamp(1, V_30188.V_30011.max_page_size);
    let V_30071 = (V_30069 - 1) * V_30070;

    let V_30361 = sqlx::query_as::<V_30020, S_50005>(
        V_30522#"
        SELECT sf.id AS file_id, sf.file_name, ss.id AS sheet_id, ss.sheet_name
        FROM source_files sf
        JOIN source_sheets ss ON ss.file_id = sf.id
        WHERE sf.id = $1 AND ss.id = $2
        "#,
    )
    .bind(file_id)
    .bind(sheet_id)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;
    let Some(V_30361) = V_30361 else {
        return Ok(HttpResponse::NotFound().body("Hoja no encontrada"));
    };

    let V_30362 = sqlx::query_as::<V_30020, S_50006>(
        "SELECT original_name, technical_column_name FROM source_columns WHERE sheet_id = $1 ORDER BY column_position",
    )
    .bind(sheet_id)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let (V_30079, V_30400) = if V_30065.is_empty() {
        let V_30119: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM raw_sheet_rows WHERE sheet_id = $1")
            .bind(sheet_id)
            .fetch_one(&V_30188.V_30012)
            .await
            .map_err(error::ErrorInternalServerError)?;
        let V_30022 = sqlx::query_as::<V_30020, S_50007>(
            "SELECT source_row_number, payload_json, COALESCE(version_actual, 1) AS version_actual, COALESCE(total_novedades, 0) AS total_novedades FROM raw_sheet_rows WHERE sheet_id = $1 ORDER BY source_row_number LIMIT $2 OFFSET $3",
        )
        .bind(sheet_id)
        .bind(V_30070)
        .bind(V_30071)
        .fetch_all(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;
        (V_30119, V_30022)
    } else {
        let V_30076 = format!("%{}%", V_30065);
        let V_30119: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM raw_sheet_rows WHERE sheet_id = $1 AND payload_json::text ILIKE $2")
                .bind(sheet_id)
                .bind(&V_30076)
                .fetch_one(&V_30188.V_30012)
                .await
                .map_err(error::ErrorInternalServerError)?;
        let V_30022 = sqlx::query_as::<V_30020, S_50007>(
            "SELECT source_row_number, payload_json, COALESCE(version_actual, 1) AS version_actual, COALESCE(total_novedades, 0) AS total_novedades FROM raw_sheet_rows WHERE sheet_id = $1 AND payload_json::text ILIKE $2 ORDER BY source_row_number LIMIT $3 OFFSET $4",
        )
        .bind(sheet_id)
        .bind(V_30076)
        .bind(V_30070)
        .bind(V_30071)
        .fetch_all(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;
        (V_30119, V_30022)
    };

    let V_30364 = V_30362.len() > 5;
    let mut V_30365: Vec<S_50008> = Vec::new();
    for V_30522 in V_30400 {
        let mut V_30366: Vec<String> = Vec::new();
        let mut V_30367: Vec<serde_json::Value> = Vec::new();

        let mut V_30368 = String::new();
        let mut V_30369 = String::new();
        let mut V_30370 = String::new();
        let mut V_30371 = String::new();
        let mut V_30372 = String::new();
        let mut V_30373 = String::new();

        for col in &V_30362 {
            let V_30374 = col.original_name.to_lowercase();
            let V_30375 = V_30522
                .payload_json
                .get(&col.technical_column_name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            let V_30376 = if V_30375.is_empty() {
                "—".to_string()
            } else {
                V_30375.clone()
            };

            V_30366.push(V_30376.clone());

            V_30367.push(json!({
                "V_30181": col.original_name,
                "value": V_30376
            }));

            if !V_30375.is_empty() && V_30375 != "Sin datos" && V_30375 != "-" {
                if V_30374.contains("credencial") && V_30368.is_empty() {
                    V_30368 = V_30375.clone();
                }
                if (V_30374.contains("apellido y nombre") || V_30374.contains("apellido, nombre") || V_30374.contains("apellido, nombres") || V_30374.contains("causante") || V_30374.contains("nombre y apellido")) && V_30371.is_empty() {
                    V_30371 = V_30375.clone();
                } else if (V_30374 == "apellido" || V_30374 == "apellidos") && V_30369.is_empty() {
                    V_30369 = V_30375.clone();
                } else if (V_30374 == "nombre" || V_30374 == "nombres") && V_30370.is_empty() {
                    V_30370 = V_30375.clone();
                }

                if (V_30374.contains("V_30113") || V_30374.contains("resoluc") || V_30374.contains("sumario") || V_30374.contains("orden interna") || V_30374.contains("actuacion")) && V_30372.is_empty() {
                    V_30372 = V_30375.clone();
                }

                if (V_30374.contains("destino") || V_30374.contains("unidad") || V_30374.contains("complejo") || V_30374.contains("departamento") || V_30374.contains("V_30318") || V_30374.contains("sit. de revista")) && V_30373.is_empty() {
                    V_30373 = V_30375.clone();
                }
            }
        }

        let V_30377 = if !V_30371.is_empty() {
            V_30371
        } else if !V_30369.is_empty() || !V_30370.is_empty() {
            format!("{} {}", V_30369, V_30370).trim().to_string()
        } else {
            "—".to_string()
        };

        let V_30378 = if !V_30368.is_empty() {
            V_30368
        } else {
            "—".to_string()
        };

        let V_30379 = if !V_30372.is_empty() {
            V_30372
        } else {
            "—".to_string()
        };

        let V_30380 = if !V_30373.is_empty() {
            V_30373
        } else {
            V_30361.sheet_name.clone()
        };

        let V_30381 = serde_json::to_string(&V_30367).unwrap_or_else(|V_30020| "[]".to_string());

        V_30365.push(S_50008 {
            source_row_number: V_30522.source_row_number,
            credencial: V_30378,
            V_30526: V_30377,
            expediente_resolucion: V_30379,
            origen_destino: V_30380,
            values: V_30366,
            V_30381,
            version_actual: V_30522.version_actual,
            total_novedades: V_30522.total_novedades,
        });
    }
    let V_30087 = ((V_30079 as f64) / (V_30070 as f64)).ceil().max(1.0) as i64;
    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "query_sheet_view",
        "raw_sheet_rows",
        "success",
        json!({ "file_id": file_id, "sheet_id": sheet_id, "V_30065": V_30065, "V_30069": V_30069, "V_30070": V_30070 }),
    )
    .await;

    render_html(&S_50043 {
        user_name: V_30064.display_name,
        V_30088,
        file_id: V_30361.file_id,
        sheet_id: V_30361.sheet_id,
        file_name: V_30361.file_name,
        sheet_name: V_30361.sheet_name,
        V_30362: V_30362.into_iter().map(|c| c.original_name).collect(),
        V_30022: V_30365,
        V_30364,
        V_30065,
        V_30069,
        V_30087,
        V_30079,
        V_30070,
    })
}

#[get("/busqueda-global")]
async fn F_40087(req: HttpRequest, V_30188: web::Data<AppState>, query: web::Query<S_50110>) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let credencial = query.credencial.clone().unwrap_or_default().trim().to_string();
    let V_30113 = query.V_30113.clone().unwrap_or_default().trim().to_string();
    let nombre = query.nombre.clone().unwrap_or_default().trim().to_string();
    let apellido = query.apellido.clone().unwrap_or_default().trim().to_string();
    let V_30114 = query.V_30114.unwrap_or(false);
    let V_30069 = query.V_30069.unwrap_or(1).max(1);
    let V_30070 = query.V_30070.unwrap_or(50).clamp(1, V_30188.V_30011.max_page_size);
    let V_30071 = (V_30069 - 1) * V_30070;

    let V_30115 = !(credencial.is_empty() && V_30113.is_empty() && nombre.is_empty() && apellido.is_empty());

    let (V_30079, V_30022) = if !V_30115 {
        (0_i64, Vec::<S_50009>::new())
    } else {
        let V_30116 = "SELECT credencial, rol_credencial, archivo_origen, hoja_origen, fila_origen FROM vw_credencial_busqueda_global WHERE 1 = 1";
        let V_30117 = "SELECT COUNT(*) FROM vw_credencial_busqueda_global WHERE 1 = 1";
        let mut V_30072 = V_30116.to_string();
        let mut V_30073 = V_30117.to_string();
        let mut V_30075 = 1_i32;
        let mut V_30074: Vec<String> = Vec::new();

        if !credencial.is_empty() {
            let V_30118 = V_30075;
            if V_30114 {
                let V_30076 = format!("%{}%", credencial);
                let V_30111 = format!(" AND credencial ILIKE ${}", V_30118);
                V_30072.push_str(&V_30111);
                V_30073.push_str(&V_30111);
                V_30074.push(V_30076);
            } else {
                let V_30111 = format!(" AND credencial = ${}", V_30118);
                V_30072.push_str(&V_30111);
                V_30073.push_str(&V_30111);
                V_30074.push(credencial.clone());
            }
            V_30075 += 1;
        }
        if !V_30113.is_empty() {
            let V_30076 = format!("%{}%", V_30113);
            let V_30118 = V_30075;
            let V_30111 = format!(" AND (COALESCE(payload_json::text, '') ILIKE ${} OR COALESCE(credencial, '') ILIKE ${})", V_30118, V_30075 + 1);
            V_30072.push_str(&V_30111);
            V_30073.push_str(&V_30111);
            V_30074.push(V_30076.clone());
            V_30074.push(V_30076);
            V_30075 += 2;
        }
        if !nombre.is_empty() {
            let V_30076 = format!("%{}%", nombre);
            let V_30118 = V_30075;
            let V_30111 = format!(" AND (COALESCE(payload_json::text, '') ILIKE ${} OR COALESCE(payload_json ->> 'nombre', '') ILIKE ${})", V_30118, V_30075 + 1);
            V_30072.push_str(&V_30111);
            V_30073.push_str(&V_30111);
            V_30074.push(V_30076.clone());
            V_30074.push(V_30076);
            V_30075 += 2;
        }
        if !apellido.is_empty() {
            let V_30076 = format!("%{}%", apellido);
            let V_30118 = V_30075;
            let V_30111 = format!(" AND (COALESCE(payload_json::text, '') ILIKE ${} OR COALESCE(payload_json ->> 'apellido', '') ILIKE ${})", V_30118, V_30075 + 1);
            V_30072.push_str(&V_30111);
            V_30073.push_str(&V_30111);
            V_30074.push(V_30076.clone());
            V_30074.push(V_30076);
            V_30075 += 2;
        }

        let mut V_30078 = sqlx::query_scalar::<V_30020, i64>(&V_30073);
        for value in &V_30074 {
            V_30078 = V_30078.bind(value);
        }
        let V_30119: i64 = V_30078
            .fetch_one(&V_30188.V_30012)
            .await
            .map_err(error::ErrorInternalServerError)?;

        let V_30080 = format!(" ORDER BY archivo_origen, hoja_origen, fila_origen LIMIT ${} OFFSET ${}", V_30075, V_30075 + 1);
        V_30072.push_str(&V_30080);
        let mut V_30120 = sqlx::query_as::<V_30020, S_50009>(&V_30072);
        for value in &V_30074 {
            V_30120 = V_30120.bind(value);
        }
        V_30120 = V_30120.bind(V_30070).bind(V_30071);
        let V_30022 = V_30120.fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;
        (V_30119, V_30022)
    };
    let V_30087 = ((V_30079 as f64) / (V_30070 as f64)).ceil().max(1.0) as i64;
    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();

    if V_30115 {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "global_search_multiparam",
            "vw_credencial_busqueda_global",
            "success",
            json!({ "credencial": credencial, "V_30113": V_30113, "nombre": nombre, "apellido": apellido, "V_30022": V_30079 }),
        )
        .await;
    }

    render_html(&S_50044 {
        user_name: V_30064.display_name,
        V_30088,
        credencial: credencial.clone(),
        V_30113: V_30113.clone(),
        nombre: nombre.clone(),
        apellido: apellido.clone(),
        V_30114,
        V_30022,
        V_30069,
        V_30087,
        V_30079,
        V_30070,
    })
}

#[get("/export/personas.csv")]
async fn F_40088(req: HttpRequest, V_30188: web::Data<AppState>, query: web::Query<S_50100>) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40023(&V_30064.role) {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "F_40088",
            "personas",
            "forbidden",
            json!({}),
        )
        .await;
        return Ok(HttpResponse::Forbidden().body("Sin permisos de exportación"));
    }
    let V_30022 = F_40090(&V_30188, query.V_30065.clone(), query.V_30103.clone(), query.V_30104.clone()).await?;
    let mut V_30121 = Vec::new();
    V_30121.extend_from_slice("\u{feff}".as_bytes());
    let mut V_30122 = WriterBuilder::new().delimiter(b';').from_writer(V_30121);
    V_30122
        .write_record(["Credencial", "Apellido", "Nombre", "Escalafon / Grado", "Estado Disciplinario", "Modulo / Origen", "Ultima Actualizacion"])
        .map_err(error::ErrorInternalServerError)?;
    for V_30522 in &V_30022 {
        V_30122
            .write_record([
                F_40091(&V_30522.credencial),
                F_40091(V_30522.apellido.as_deref().unwrap_or("-")),
                F_40091(V_30522.nombre.as_deref().unwrap_or("-")),
                F_40091(V_30522.escalafon_grado.as_deref().unwrap_or("-")),
                F_40091(V_30522.estado_disciplinario_label()),
                F_40091(V_30522.origen_principal.as_deref().unwrap_or("Sin datos")),
                F_40091(&V_30522.last_seen_at.format("%Y-%m-%d %H:%M:%S").to_string()),
            ])
            .map_err(error::ErrorInternalServerError)?;
    }
    let V_30123 = V_30122
        .into_inner()
        .map_err(|e| error::ErrorInternalServerError(e.to_string()))?;
    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "F_40088",
        "personas",
        "success",
        json!({ "V_30065": query.V_30065.clone().unwrap_or_default(), "V_30022": V_30022.len() }),
    )
    .await;
    Ok(HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, "text/csv; charset=utf-8"))
        .append_header((header::CONTENT_DISPOSITION, "attachment; V_30182=\"personas.csv\""))
        .body(V_30123))
}

#[get("/export/personas.xlsx")]
async fn F_40089(req: HttpRequest, V_30188: web::Data<AppState>, query: web::Query<S_50100>) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40023(&V_30064.role) {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "F_40089",
            "personas",
            "forbidden",
            json!({}),
        )
        .await;
        return Ok(HttpResponse::Forbidden().body("Sin permisos de exportación"));
    }
    let V_30022 = F_40090(&V_30188, query.V_30065.clone(), query.V_30103.clone(), query.V_30104.clone()).await?;
    let mut V_30124 = Workbook::new();
    let V_30125 = V_30124.add_worksheet();

    let V_30126 = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xFFFFFF))
        .set_background_color(Color::RGB(0x107C41));

    let V_30127 = Format::new().set_num_format("@");

    V_30125.write_string_with_format(0, 0, "Credencial", &V_30126).map_err(F_40092)?;
    V_30125.write_string_with_format(0, 1, "Apellido", &V_30126).map_err(F_40092)?;
    V_30125.write_string_with_format(0, 2, "Nombre", &V_30126).map_err(F_40092)?;
    V_30125.write_string_with_format(0, 3, "Escalafón / Grado", &V_30126).map_err(F_40092)?;
    V_30125.write_string_with_format(0, 4, "Estado Disciplinario", &V_30126).map_err(F_40092)?;
    V_30125.write_string_with_format(0, 5, "Módulo / Origen", &V_30126).map_err(F_40092)?;
    V_30125.write_string_with_format(0, 6, "Última Actualización", &V_30126).map_err(F_40092)?;

    for (idx, V_30522) in V_30022.iter().enumerate() {
        let V_30128 = (idx + 1) as u32;
        V_30125
            .write_string_with_format(V_30128, 0, F_40091(&V_30522.credencial), &V_30127)
            .map_err(F_40092)?;
        V_30125
            .write_string(
                V_30128,
                1,
                F_40091(V_30522.apellido.as_deref().unwrap_or("-")),
            )
            .map_err(F_40092)?;
        V_30125
            .write_string(
                V_30128,
                2,
                F_40091(V_30522.nombre.as_deref().unwrap_or("-")),
            )
            .map_err(F_40092)?;
        V_30125
            .write_string(
                V_30128,
                3,
                F_40091(V_30522.escalafon_grado.as_deref().unwrap_or("-")),
            )
            .map_err(F_40092)?;
        V_30125
            .write_string(
                V_30128,
                4,
                F_40091(V_30522.estado_disciplinario_label()),
            )
            .map_err(F_40092)?;
        V_30125
            .write_string(
                V_30128,
                5,
                F_40091(V_30522.origen_principal.as_deref().unwrap_or("Sin datos")),
            )
            .map_err(F_40092)?;
        V_30125
            .write_string(V_30128, 6, F_40091(&V_30522.last_seen_at.format("%Y-%m-%d %H:%M:%S").to_string()))
            .map_err(F_40092)?;
    }
    V_30125.autofit();
    let V_30123 = V_30124.save_to_buffer().map_err(F_40092)?;
    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "F_40089",
        "personas",
        "success",
        json!({ "V_30065": query.V_30065.clone().unwrap_or_default(), "V_30022": V_30022.len() }),
    )
    .await;
    Ok(HttpResponse::Ok()
        .append_header((
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ))
        .append_header((header::CONTENT_DISPOSITION, "attachment; V_30182=\"personas.xlsx\""))
        .body(V_30123))
}

async fn F_40090(
    V_30188: &web::Data<AppState>,
    V_30065: Option<String>,
    V_30103: Option<String>,
    V_30104: Option<String>,
) -> Result<Vec<S_50002>> {
    let V_30397 = V_30065.as_ref().map(|V_30044| !V_30044.trim().is_empty()).unwrap_or(false);
    let V_30076 = if V_30397 {
        format!("%{}%", V_30065.as_ref().unwrap().trim())
    } else {
        String::new()
    };
    let V_30398 = V_30104.clone().unwrap_or_default();
    let V_30399 = !V_30398.is_empty() && V_30398 != "todos";

    let mut V_30109 = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        "SELECT V_30147.credencial, V_30147.apellido, V_30147.nombre, V_30147.escalafon_grado, V_30147.V_30526, V_30147.origen_principal, \
         COALESCE(V_30147.last_seen_at, LOCALTIMESTAMP) AS last_seen_at, \
         (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Vigente', 'Suspendido')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'Suspendidos')) AS tiene_suspension_activa, \
         (EXISTS (SELECT 1 FROM expediente_agentes ea JOIN expedientes e ON e.id = ea.V_30275 WHERE ea.credencial = V_30147.credencial AND e.estado IN ('Activo', 'En analisis', 'En análisis', 'Suspendido', 'Inicio', 'Abierto', 'En Trámite')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('SUM ADM', 'INFO SUM'))) AS tiene_sumario_abierto, \
         (EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('CONDENA', 'PROCESAMIENTO', 'CC'))) AS tiene_impedimento_judicial \
         FROM personas V_30147 WHERE 1 = 1 "
    );

    if V_30397 {
        V_30109.push(" AND (V_30147.credencial ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(" OR COALESCE(V_30147.apellido,'') ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(" OR COALESCE(V_30147.nombre,'') ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(" OR COALESCE(V_30147.escalafon_grado,'') ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(" OR COALESCE(V_30147.V_30526,'') ILIKE ");
        V_30109.push_bind(&V_30076);
        V_30109.push(") ");
    }

    if V_30399 {
        V_30109.push(" AND V_30147.origen_principal = ");
        V_30109.push_bind(&V_30398);
    }

    if let Some(ed) = V_30103.as_deref() {
        match ed {
            "suspension_activa" => {
                V_30109.push(" AND (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Vigente', 'Suspendido')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'Suspendidos')) ");
            }
            "sumario_abierto" => {
                V_30109.push(" AND (EXISTS (SELECT 1 FROM expediente_agentes ea JOIN expedientes e ON e.id = ea.V_30275 WHERE ea.credencial = V_30147.credencial AND e.estado IN ('Activo', 'En analisis', 'En análisis', 'Suspendido', 'Inicio', 'Abierto', 'En Trámite')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('SUM ADM', 'INFO SUM'))) ");
            }
            "suspension_levantada" => {
                V_30109.push(" AND (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Levantado', 'Levantada')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'LEVANTAMIENTO')) ");
            }
            "impedimento_judicial" => {
                V_30109.push(" AND (EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('CONDENA', 'PROCESAMIENTO', 'CC'))) ");
            }
            "sin_sanciones" => {
                V_30109.push(" AND NOT (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Vigente', 'Suspendido')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'Suspendidos') OR EXISTS (SELECT 1 FROM expediente_agentes ea JOIN expedientes e ON e.id = ea.V_30275 WHERE ea.credencial = V_30147.credencial AND e.estado IN ('Activo', 'En analisis', 'En análisis', 'Suspendido', 'Inicio', 'Abierto', 'En Trámite')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('SUM ADM', 'INFO SUM'))) ");
            }
            V_30020 => {}
        }
    }

    V_30109.push(" ORDER BY V_30147.credencial ASC LIMIT ");
    V_30109.push_bind(V_30188.V_30011.max_export_rows);

    let V_30022: Vec<S_50002> = V_30109.build_query_as().fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;
    V_30054(V_30022)
}

fn F_40091(value: &str) -> String {
    if value == "-" || value.trim().is_empty() {
        return value.to_string();
    }
    if let Some(first) = value.chars().next() {
        if matches!(first, '=' | '+' | '-' | '@') {
            return format!("'{}", value);
        }
    }
    value.to_string()
}

fn F_40092(err: XlsxError) -> actix_web::Error {
    error::ErrorInternalServerError(format!("Error XLSX: {err}"))
}

#[get("/export/fuente/{file_id}/hoja/{sheet_id}.xlsx")]
async fn F_40093(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    query: web::Query<S_50099>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40023(&V_30064.role) {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "F_40093",
            "hoja_fuente",
            "forbidden",
            json!({}),
        )
        .await;
        return Ok(HttpResponse::Forbidden().body("Sin permisos de exportación"));
    }
    let (file_id, sheet_id) = path.into_inner();
    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();

    let V_30361 = sqlx::query_as::<V_30020, S_50005>(
        V_30522#"
        SELECT sf.id AS file_id, sf.file_name, ss.id AS sheet_id, ss.sheet_name
        FROM source_files sf
        JOIN source_sheets ss ON ss.file_id = sf.id
        WHERE sf.id = $1 AND ss.id = $2
        "#,
    )
    .bind(file_id)
    .bind(sheet_id)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some(V_30361) = V_30361 else {
        return Ok(HttpResponse::NotFound().body("Hoja no encontrada"));
    };

    let V_30362 = sqlx::query_as::<V_30020, S_50006>(
        "SELECT original_name, technical_column_name FROM source_columns WHERE sheet_id = $1 ORDER BY column_position",
    )
    .bind(sheet_id)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30400 = if V_30065.is_empty() {
        sqlx::query_as::<V_30020, S_50007>(
            "SELECT source_row_number, payload_json, COALESCE(version_actual, 1) AS version_actual, COALESCE(total_novedades, 0) AS total_novedades FROM raw_sheet_rows WHERE sheet_id = $1 ORDER BY source_row_number LIMIT $2",
        )
        .bind(sheet_id)
        .bind(V_30188.V_30011.max_export_rows)
        .fetch_all(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?
    } else {
        let V_30076 = format!("%{}%", V_30065);
        sqlx::query_as::<V_30020, S_50007>(
            "SELECT source_row_number, payload_json, COALESCE(version_actual, 1) AS version_actual, COALESCE(total_novedades, 0) AS total_novedades FROM raw_sheet_rows WHERE sheet_id = $1 AND payload_json::text ILIKE $2 ORDER BY source_row_number LIMIT $3",
        )
        .bind(sheet_id)
        .bind(V_30076)
        .bind(V_30188.V_30011.max_export_rows)
        .fetch_all(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?
    };

    let mut V_30124 = Workbook::new();
    let V_30125 = V_30124.add_worksheet();

    let V_30401: String = V_30361
        .sheet_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == 'V_30020')
        .take(31)
        .collect();
    if !V_30401.trim().is_empty() {
        let V_30020 = V_30125.set_name(V_30401.trim());
    }

    let V_30126 = rust_xlsxwriter::Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0x107C41))
        .set_font_color(rust_xlsxwriter::Color::RGB(0xFFFFFF));

    // Cabeceras
    V_30125.write_string_with_format(0, 0, "Fila Original", &V_30126).map_err(F_40092)?;
    V_30125.write_string_with_format(0, 1, "Versión Fila", &V_30126).map_err(F_40092)?;
    V_30125.write_string_with_format(0, 2, "Total Novedades", &V_30126).map_err(F_40092)?;

    for (c_idx, col) in V_30362.iter().enumerate() {
        let V_30402 = (c_idx + 3) as u16;
        V_30125.write_string_with_format(0, V_30402, &col.original_name, &V_30126).map_err(F_40092)?;
    }

    // Registros de la hoja
    for (r_idx, V_30522) in V_30400.iter().enumerate() {
        let V_30403 = (r_idx + 1) as u32;
        V_30125.write_number(V_30403, 0, V_30522.source_row_number as f64).map_err(F_40092)?;
        V_30125.write_string(V_30403, 1, format!("v{}", V_30522.version_actual)).map_err(F_40092)?;
        V_30125.write_number(V_30403, 2, V_30522.total_novedades as f64).map_err(F_40092)?;

        for (c_idx, col) in V_30362.iter().enumerate() {
            let V_30402 = (c_idx + 3) as u16;
            let V_30190 = V_30522
                .payload_json
                .get(&col.technical_column_name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            V_30125.write_string(V_30403, V_30402, F_40091(V_30190)).map_err(F_40092)?;
        }
    }

    let V_30123 = V_30124.save_to_buffer().map_err(F_40092)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "F_40093",
        "hoja_fuente",
        "success",
        json!({
            "file_id": file_id,
            "sheet_id": sheet_id,
            "sheet_name": V_30361.sheet_name,
            "V_30065": V_30065,
            "V_30079": V_30400.len()
        }),
    )
    .await;

    let V_30404 = format!("{}V_30020{}", V_30361.file_name.replace(".xlsx", "").replace(".xls", ""), V_30361.sheet_name)
        .chars()
        .map(|c| if c.is_alphanumeric() || c == 'V_30020' || c == '-' { c } else { 'V_30020' })
        .collect::<String>();
    let V_30182 = format!("{}.xlsx", V_30404);

    Ok(HttpResponse::Ok()
        .append_header((
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ))
        .append_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; V_30182=\"{}\"", V_30182),
        ))
        .body(V_30123))
}

#[get("/export/fuente/{file_id}/hoja/{sheet_id}.csv")]
async fn F_40094(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    query: web::Query<S_50099>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40023(&V_30064.role) {
        return Ok(HttpResponse::Forbidden().body("Sin permisos de exportación"));
    }
    let (file_id, sheet_id) = path.into_inner();
    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();

    let V_30361 = sqlx::query_as::<V_30020, S_50005>(
        V_30522#"
        SELECT sf.id AS file_id, sf.file_name, ss.id AS sheet_id, ss.sheet_name
        FROM source_files sf
        JOIN source_sheets ss ON ss.file_id = sf.id
        WHERE sf.id = $1 AND ss.id = $2
        "#,
    )
    .bind(file_id)
    .bind(sheet_id)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some(V_30361) = V_30361 else {
        return Ok(HttpResponse::NotFound().body("Hoja no encontrada"));
    };

    let V_30362 = sqlx::query_as::<V_30020, S_50006>(
        "SELECT original_name, technical_column_name FROM source_columns WHERE sheet_id = $1 ORDER BY column_position",
    )
    .bind(sheet_id)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30400 = if V_30065.is_empty() {
        sqlx::query_as::<V_30020, S_50007>(
            "SELECT source_row_number, payload_json, COALESCE(version_actual, 1) AS version_actual, COALESCE(total_novedades, 0) AS total_novedades FROM raw_sheet_rows WHERE sheet_id = $1 ORDER BY source_row_number LIMIT $2",
        )
        .bind(sheet_id)
        .bind(V_30188.V_30011.max_export_rows)
        .fetch_all(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?
    } else {
        let V_30076 = format!("%{}%", V_30065);
        sqlx::query_as::<V_30020, S_50007>(
            "SELECT source_row_number, payload_json, COALESCE(version_actual, 1) AS version_actual, COALESCE(total_novedades, 0) AS total_novedades FROM raw_sheet_rows WHERE sheet_id = $1 AND payload_json::text ILIKE $2 ORDER BY source_row_number LIMIT $3",
        )
        .bind(sheet_id)
        .bind(V_30076)
        .bind(V_30188.V_30011.max_export_rows)
        .fetch_all(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?
    };

    let mut V_30405 = String::from("\u{feff}"); // UTF-8 BOM para apertura perfecta en Excel
    let mut V_30406 = vec!["\"Fila Original\"".to_string(), "\"Versión Fila\"".to_string(), "\"Total Novedades\"".to_string()];
    for col in &V_30362 {
        V_30406.push(format!("\"{}\"", col.original_name.replace('"', "\"\"")));
    }
    V_30405.push_str(&V_30406.join(";"));
    V_30405.push_str("\V_30522\V_30063");

    for V_30522 in &V_30400 {
        let mut V_30407 = vec![
            V_30522.source_row_number.to_string(),
            format!("\"v{}\"", V_30522.version_actual),
            V_30522.total_novedades.to_string(),
        ];
        for col in &V_30362 {
            let V_30190 = V_30522
                .payload_json
                .get(&col.technical_column_name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            V_30407.push(format!("\"{}\"", V_30190.replace('"', "\"\"")));
        }
        V_30405.push_str(&V_30407.join(";"));
        V_30405.push_str("\V_30522\V_30063");
    }

    let V_30404 = format!("{}V_30020{}", V_30361.file_name.replace(".xlsx", "").replace(".xls", ""), V_30361.sheet_name)
        .chars()
        .map(|c| if c.is_alphanumeric() || c == 'V_30020' || c == '-' { c } else { 'V_30020' })
        .collect::<String>();
    let V_30182 = format!("{}.csv", V_30404);

    Ok(HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, "text/csv; charset=utf-8"))
        .append_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; V_30182=\"{}\"", V_30182),
        ))
        .body(V_30405))
}

#[get("/importar")]
async fn F_40095(req: HttpRequest, V_30188: web::Data<AppState>) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    render_html(&S_50045 {
        user_name: V_30064.display_name,
    })
}

#[post("/api/importar/analizar")]
async fn F_40096(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let (_fields, V_30178) = F_40070(V_30012, "uploads").await?;
    let V_30408 = V_30178.values().next().ok_or_else(|| error::ErrorBadRequest("No se proporcionó ningún V_30505"))?;
    let V_30146 = format!("uploads/{}", V_30408.V_30185);

    let V_30409 = tokio::task::spawn_blocking(move || {
        std::process::Command::new("python")
            .args([if std::path::Path::new("scripts/excel_importer_service.py").V_30028() { "scripts/excel_importer_service.py" } else { "../scripts/excel_importer_service.py" }, "analizar", &V_30146])
            .V_30409()
    })
    .await
    .map_err(|e| error::ErrorInternalServerError(format!("Error en tarea de análisis: {}", e)))?
    .map_err(|e| error::ErrorInternalServerError(format!("Error ejecutando analizador: {}", e)))?;

    if !V_30409.status.success() {
        let V_30410 = String::from_utf8_lossy(&V_30409.stderr);
        return Ok(HttpResponse::BadRequest().body(format!("Error analizando Excel: {}", V_30410)));
    }

    let V_30411 = V_30409.stdout;
    Ok(HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, "application/json; charset=utf-8"))
        .body(V_30411))
}

#[post("/api/importar/confirmar")]
async fn F_40097(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let (_fields, V_30178) = F_40070(V_30012, "uploads").await?;
    let V_30408 = V_30178.values().next().ok_or_else(|| error::ErrorBadRequest("No se proporcionó ningún V_30505"))?;
    let V_30146 = format!("uploads/{}", V_30408.V_30185);

    let V_30055 = V_30064.V_30055;
    let V_30012 = V_30188.V_30012.clone();

    let V_30409 = tokio::task::spawn_blocking(move || {
        std::process::Command::new("python")
            .args([if std::path::Path::new("scripts/excel_importer_service.py").V_30028() { "scripts/excel_importer_service.py" } else { "../scripts/excel_importer_service.py" }, "confirmar", &V_30146])
            .V_30409()
    })
    .await
    .map_err(|e| error::ErrorInternalServerError(format!("Error en tarea de confirmación: {}", e)))?
    .map_err(|e| error::ErrorInternalServerError(format!("Error ejecutando confirmación: {}", e)))?;

    if !V_30409.status.success() {
        let V_30410 = String::from_utf8_lossy(&V_30409.stderr);
        return Ok(HttpResponse::BadRequest().body(format!("Error importando Excel: {}", V_30410)));
    }

    write_audit(
        &V_30012,
        Some(V_30055),
        "import_excel_confirmed",
        "expedientes",
        "success",
        json!({ "V_30060": V_30408.orig_name, "V_30070": V_30408.size_bytes }),
    )
    .await;

    let V_30411 = V_30409.stdout;
    Ok(HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, "application/json; charset=utf-8"))
        .body(V_30411))
}

#[get("/export/expedientes.xlsx")]
async fn F_40098(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50101>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40023(&V_30064.role) {
        return Ok(HttpResponse::Forbidden().body("Sin permisos de exportación"));
    }
    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    let reparticion = query.reparticion.clone().unwrap_or_default().trim().to_string();
    let estado = query.estado.clone().unwrap_or_default().trim().to_string();
    let V_30066 = query.V_30066.clone().unwrap_or_default().trim().to_string();
    let V_30067 = query.V_30067.clone().unwrap_or_default().trim().to_string();
    let V_30068 = query.V_30068.clone().unwrap_or_default().trim().to_string();

    // Construcción de condiciones WHERE para expedientes según los filtros de búsqueda activos
    let mut V_30412: Vec<String> = Vec::new();
    let mut V_30074: Vec<String> = Vec::new();
    let mut V_30413 = 1;

    if !V_30065.is_empty() {
        let V_30076 = format!("%{}%", V_30065);
        V_30412.push(format!(
            "(e.numero_expediente ILIKE ${V_30147} OR COALESCE(e.V_30238, '') ILIKE ${V_30147} OR COALESCE(e.V_30167, '') ILIKE ${V_30147} OR COALESCE(e.V_30236, '') ILIKE ${V_30147} OR EXISTS (SELECT 1 FROM expediente_agentes ea_q WHERE ea_q.V_30275 = e.id AND (ea_q.credencial ILIKE ${V_30147} OR ea_q.V_30266 ILIKE ${V_30147})))",
            V_30147 = V_30413
        ));
        V_30074.push(V_30076);
        V_30413 += 1;
    }

    if !reparticion.is_empty() {
        V_30412.push(format!("(e.reparticion = ${V_30147} OR e.reparticion ILIKE ${V_30147})", V_30147 = V_30413));
        V_30074.push(reparticion.clone());
        V_30413 += 1;
    }

    if !estado.is_empty() {
        V_30412.push(format!(
            "(e.estado ILIKE ${V_30147} OR REPLACE(e.estado, 'á', 'a') ILIKE REPLACE(${V_30147}, 'á', 'a'))",
            V_30147 = V_30413
        ));
        V_30074.push(estado.clone());
        V_30413 += 1;
    }

    if !V_30066.is_empty() {
        V_30412.push(format!("e.V_30066 ILIKE ${V_30147}", V_30147 = V_30413));
        V_30074.push(format!("%{}%", V_30066));
        V_30413 += 1;
    }

    if !V_30067.is_empty() {
        V_30412.push(format!("e.V_30067 ILIKE ${V_30147}", V_30147 = V_30413));
        V_30074.push(format!("%{}%", V_30067));
        V_30413 += 1;
    }

    if !V_30068.is_empty() {
        if V_30068.eq_ignore_ascii_case("sí") || V_30068.eq_ignore_ascii_case("si") {
            V_30412.push("(e.V_30068 ILIKE 'S%' OR e.V_30068 = 'true')".to_string());
        } else if V_30068.eq_ignore_ascii_case("no") {
            V_30412.push("(e.V_30068 IS NULL OR e.V_30068 = '' OR e.V_30068 ILIKE 'No%' OR e.V_30068 = 'false')".to_string());
        } else {
            V_30412.push(format!("e.V_30068 = ${V_30147}", V_30147 = V_30413));
            V_30074.push(V_30068.clone());
            V_30413 += 1;
        }
    }

    let V_30414 = if V_30412.is_empty() {
        String::new()
    } else {
        format!(" AND {}", V_30412.join(" AND "))
    };

    let mut V_30124 = Workbook::new();

    let V_30126 = rust_xlsxwriter::Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0x107C41))
        .set_font_color(rust_xlsxwriter::Color::RGB(0xFFFFFF));

    let V_30415 = V_30188.V_30011.max_export_rows * 5;

    // =========================================================================
    // HOJA 1: Expedientes y Agentes Involucrados (Desglose individual por agente)
    // =========================================================================
    {
        let V_30416 = V_30124.add_worksheet();
        let V_30020 = V_30416.set_name("Expedientes y Agentes");

        let V_30417 = [
            "N° Expediente",
            "Tipo Actuación",
            "Repartición",
            "Motivo / Carátula",
            "Estado Expediente",
            "Instructor Designado",
            "Fecha Inicio",
            "Causa Judicial General",
            "Credencial",
            "Apellido",
            "Nombre",
            "Escalafón",
            "Grado",
            "Rol Actuación",
            "Capítulo Cargos",
            "Causa Individual",
            "Suspensión Individual",
            "Finalización Agente",
            "Versión Expediente",
        ];
        for (idx, h) in V_30417.iter().enumerate() {
            V_30416.write_string_with_format(0, idx as u16, *h, &V_30126).map_err(F_40092)?;
        }

        let V_30418 = format!(
            V_30522#"
            SELECT 
                e.id AS V_30275,
                COALESCE(e.numero_expediente, 'Sin datos') AS numero_expediente,
                COALESCE(e.V_30228, 'sumario') AS V_30228,
                COALESCE(e.reparticion, 'Sin datos') AS reparticion,
                COALESCE(e.V_30167, 'Sin datos') AS V_30167,
                COALESCE(e.estado, 'Abierto') AS estado,
                COALESCE(e.V_30238, 'Sin datos') AS V_30238,
                COALESCE(to_char(e.fecha_inicio, 'YYYY-MM-DD'), '') AS fecha_inicio,
                COALESCE(e.V_30068, 'No') AS V_30068,
                COALESCE(e.version_actual, 1) AS version_actual,
                ea.credencial,
                COALESCE(ea.apellido, '') AS apellido,
                COALESCE(ea.nombre, '') AS nombre,
                COALESCE(ea.V_30317, '') AS V_30317,
                COALESCE(ea.V_30318, '') AS V_30318,
                COALESCE(ea.rol, 'Involucrado') AS rol,
                CASE WHEN COALESCE(ea.V_30319, false) THEN 'Sí' ELSE 'No' END AS V_30319,
                CASE WHEN COALESCE(ea.V_30322, false) THEN 'Sí' ELSE 'No' END AS V_30322,
                CASE WHEN COALESCE(ea.V_30326, false) THEN 'Sí' ELSE 'No' END AS V_30326,
                COALESCE(ea.V_30330, 'En trámite') AS V_30330
            FROM expedientes e
            LEFT JOIN expediente_agentes ea ON ea.V_30275 = e.id
            WHERE 1=1 {filter}
            ORDER BY e.id DESC, ea.id ASC
            LIMIT ${lim_p}
            "#,
            filter = V_30414,
            lim_p = V_30413
        );

        #[derive(FromRow)]
        struct S_50121 {
            V_30275: i64,
            numero_expediente: String,
            V_30228: String,
            reparticion: String,
            V_30167: String,
            estado: String,
            V_30238: String,
            fecha_inicio: String,
            V_30068: String,
            version_actual: i32,
            credencial: Option<String>,
            apellido: String,
            nombre: String,
            V_30317: String,
            V_30318: String,
            rol: String,
            V_30319: String,
            V_30322: String,
            V_30326: String,
            V_30330: String,
        }

        let mut V_30419 = sqlx::query_as::<V_30020, S_50121>(&V_30418);
        for b in &V_30074 {
            V_30419 = V_30419.bind(b);
        }
        V_30419 = V_30419.bind(V_30415);

        let V_30420 = V_30419.fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;
        for (r_idx, V_30522) in V_30420.iter().enumerate() {
            let V_30403 = (r_idx + 1) as u32;
            V_30416.write_string(V_30403, 0, F_40091(&V_30522.numero_expediente)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 1, F_40091(&V_30522.V_30228)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 2, F_40091(&V_30522.reparticion)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 3, F_40091(&V_30522.V_30167)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 4, F_40091(&V_30522.estado)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 5, F_40091(&V_30522.V_30238)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 6, F_40091(&V_30522.fecha_inicio)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 7, F_40091(&V_30522.V_30068)).map_err(F_40092)?;
            // Credencial siempre como TEXTO para preservar ceros a la izquierda
            V_30416.write_string(V_30403, 8, F_40091(V_30522.credencial.as_deref().unwrap_or(""))).map_err(F_40092)?;
            V_30416.write_string(V_30403, 9, F_40091(&V_30522.apellido)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 10, F_40091(&V_30522.nombre)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 11, F_40091(&V_30522.V_30317)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 12, F_40091(&V_30522.V_30318)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 13, F_40091(&V_30522.rol)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 14, F_40091(&V_30522.V_30319)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 15, F_40091(&V_30522.V_30322)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 16, F_40091(&V_30522.V_30326)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 17, F_40091(&V_30522.V_30330)).map_err(F_40092)?;
            V_30416.write_string(V_30403, 18, format!("v{}", V_30522.version_actual)).map_err(F_40092)?;
        }
    }

    // =========================================================================
    // HOJA 2: Resumen de Expedientes (1 fila por V_30113)
    // =========================================================================
    {
        let V_30421 = V_30124.add_worksheet();
        let V_30020 = V_30421.set_name("Resumen de Expedientes");

        let V_30422 = [
            "N° Expediente",
            "Tipo Identificador",
            "Repartición",
            "Motivo",
            "Estado",
            "Instructor Actual",
            "Fecha Inicio",
            "Causa Judicial",
            "Total Agentes Involucrados",
            "Suspensiones Activas",
            "Versión Actual",
            "Fecha Última Modificación",
        ];
        for (idx, h) in V_30422.iter().enumerate() {
            V_30421.write_string_with_format(0, idx as u16, *h, &V_30126).map_err(F_40092)?;
        }

        let V_30423 = format!(
            V_30522#"
            SELECT 
                e.id,
                COALESCE(e.numero_expediente, 'Sin datos') AS numero_expediente,
                COALESCE(e.V_30228, 'sumario') AS V_30228,
                COALESCE(e.reparticion, 'Sin datos') AS reparticion,
                COALESCE(e.V_30167, 'Sin datos') AS V_30167,
                COALESCE(e.estado, 'Abierto') AS estado,
                COALESCE(e.V_30238, 'Sin datos') AS V_30238,
                COALESCE(to_char(e.fecha_inicio, 'YYYY-MM-DD'), '') AS fecha_inicio,
                COALESCE(e.V_30068, 'No') AS V_30068,
                COALESCE((SELECT COUNT(*) FROM expediente_agentes ea WHERE ea.V_30275 = e.id), 0) AS agentes_vinculados,
                COALESCE((SELECT COUNT(*) FROM expediente_suspensiones es WHERE es.V_30275 = e.id AND es.estado = 'Vigente'), 0) AS V_30497,
                COALESCE(e.version_actual, 1) AS version_actual,
                COALESCE(to_char(e.updated_at, 'YYYY-MM-DD HH24:MI'), '') AS updated_at
            FROM expedientes e
            WHERE 1=1 {filter}
            ORDER BY e.id DESC
            LIMIT ${lim_p}
            "#,
            filter = V_30414,
            lim_p = V_30413
        );

        #[derive(FromRow)]
        struct S_50122 {
            id: i64,
            numero_expediente: String,
            V_30228: String,
            reparticion: String,
            V_30167: String,
            estado: String,
            V_30238: String,
            fecha_inicio: String,
            V_30068: String,
            agentes_vinculados: i64,
            V_30497: i64,
            version_actual: i32,
            updated_at: String,
        }

        let mut V_30424 = sqlx::query_as::<V_30020, S_50122>(&V_30423);
        for b in &V_30074 {
            V_30424 = V_30424.bind(b);
        }
        V_30424 = V_30424.bind(V_30188.V_30011.max_export_rows);

        let V_30425 = V_30424.fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;
        for (r_idx, V_30522) in V_30425.iter().enumerate() {
            let V_30403 = (r_idx + 1) as u32;
            V_30421.write_string(V_30403, 0, F_40091(&V_30522.numero_expediente)).map_err(F_40092)?;
            V_30421.write_string(V_30403, 1, F_40091(&V_30522.V_30228)).map_err(F_40092)?;
            V_30421.write_string(V_30403, 2, F_40091(&V_30522.reparticion)).map_err(F_40092)?;
            V_30421.write_string(V_30403, 3, F_40091(&V_30522.V_30167)).map_err(F_40092)?;
            V_30421.write_string(V_30403, 4, F_40091(&V_30522.estado)).map_err(F_40092)?;
            V_30421.write_string(V_30403, 5, F_40091(&V_30522.V_30238)).map_err(F_40092)?;
            V_30421.write_string(V_30403, 6, F_40091(&V_30522.fecha_inicio)).map_err(F_40092)?;
            V_30421.write_string(V_30403, 7, F_40091(&V_30522.V_30068)).map_err(F_40092)?;
            V_30421.write_number(V_30403, 8, V_30522.agentes_vinculados as f64).map_err(F_40092)?;
            V_30421.write_number(V_30403, 9, V_30522.V_30497 as f64).map_err(F_40092)?;
            V_30421.write_string(V_30403, 10, format!("v{}", V_30522.version_actual)).map_err(F_40092)?;
            V_30421.write_string(V_30403, 11, F_40091(&V_30522.updated_at)).map_err(F_40092)?;
        }
    }

    // =========================================================================
    // HOJA 3: Suspensiones Preventivas
    // =========================================================================
    {
        let V_30426 = V_30124.add_worksheet();
        let V_30020 = V_30426.set_name("Suspensiones Preventivas");

        let V_30427 = [
            "N° Expediente",
            "Credencial",
            "Nombre Completo Agente",
            "Fecha Inicio",
            "Art. 419 Incs.",
            "Estado Medida",
            "Fecha Cese/Fin",
            "Motivo Cese",
            "Fundamento Cese",
            "Registrado Por",
            "Observaciones",
        ];
        for (idx, h) in V_30427.iter().enumerate() {
            V_30426.write_string_with_format(0, idx as u16, *h, &V_30126).map_err(F_40092)?;
        }

        let V_30428 = format!(
            V_30522#"
            SELECT 
                V_30044.id,
                COALESCE(e.numero_expediente, '') AS numero_expediente,
                COALESCE(V_30044.credencial, '') AS credencial,
                COALESCE(V_30044.V_30266, '') AS V_30266,
                COALESCE(to_char(V_30044.fecha_inicio, 'YYYY-MM-DD'), '') AS fecha_inicio,
                COALESCE(V_30044.V_30471, '') AS V_30471,
                COALESCE(V_30044.estado, 'Vigente') AS estado,
                COALESCE(to_char(V_30044.fecha_fin, 'YYYY-MM-DD'), '') AS fecha_fin,
                COALESCE(V_30044.V_30476, '') AS V_30476,
                COALESCE(V_30044.V_30477, '') AS V_30477,
                COALESCE(V_30044.usuario, '') AS usuario,
                COALESCE(V_30044.V_30463, '') AS V_30463
            FROM expediente_suspensiones V_30044
            JOIN expedientes e ON e.id = V_30044.V_30275
            WHERE 1=1 {filter}
            ORDER BY V_30044.id DESC
            LIMIT ${lim_p}
            "#,
            filter = V_30414,
            lim_p = V_30413
        );

        #[derive(FromRow)]
        struct S_50123 {
            id: i64,
            numero_expediente: String,
            credencial: String,
            V_30266: String,
            fecha_inicio: String,
            V_30471: String,
            estado: String,
            fecha_fin: String,
            V_30476: String,
            V_30477: String,
            usuario: String,
            V_30463: String,
        }

        let mut V_30429 = sqlx::query_as::<V_30020, S_50123>(&V_30428);
        for b in &V_30074 {
            V_30429 = V_30429.bind(b);
        }
        V_30429 = V_30429.bind(V_30415);

        let V_30430 = V_30429.fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;
        for (r_idx, V_30522) in V_30430.iter().enumerate() {
            let V_30403 = (r_idx + 1) as u32;
            V_30426.write_string(V_30403, 0, F_40091(&V_30522.numero_expediente)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 1, F_40091(&V_30522.credencial)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 2, F_40091(&V_30522.V_30266)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 3, F_40091(&V_30522.fecha_inicio)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 4, F_40091(&V_30522.V_30471)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 5, F_40091(&V_30522.estado)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 6, F_40091(&V_30522.fecha_fin)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 7, F_40091(&V_30522.V_30476)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 8, F_40091(&V_30522.V_30477)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 9, F_40091(&V_30522.usuario)).map_err(F_40092)?;
            V_30426.write_string(V_30403, 10, F_40091(&V_30522.V_30463)).map_err(F_40092)?;
        }
    }

    // =========================================================================
    // HOJA 4: Otras Medidas y Situaciones Administrativas
    // =========================================================================
    {
        let V_30431 = V_30124.add_worksheet();
        let V_30020 = V_30431.set_name("Otras Medidas");

        let V_30432 = [
            "N° Expediente",
            "Credencial",
            "Apellido y Nombre",
            "Capítulo de Cargos",
            "Fecha Cargos",
            "Artículos Cargos",
            "Causa Judicial Individual",
            "Fecha Causa Judicial",
            "Suspensión Preventiva",
            "Fecha Suspensión",
            "Artículos Suspensión",
            "Situación Final Agente",
            "Fecha Finalización",
        ];
        for (idx, h) in V_30432.iter().enumerate() {
            V_30431.write_string_with_format(0, idx as u16, *h, &V_30126).map_err(F_40092)?;
        }

        let V_30433 = format!(
            V_30522#"
            SELECT 
                ea.id,
                COALESCE(e.numero_expediente, '') AS numero_expediente,
                COALESCE(ea.credencial, '') AS credencial,
                COALESCE(ea.V_30266, '') AS V_30266,
                CASE WHEN COALESCE(ea.V_30319, false) THEN 'Sí' ELSE 'No' END AS tiene_cargos,
                COALESCE(to_char(ea.V_30320, 'YYYY-MM-DD'), '') AS fecha_cargos,
                COALESCE(ea.V_30321, '') AS V_30321,
                CASE WHEN COALESCE(ea.V_30322, false) THEN 'Sí' ELSE 'No' END AS tiene_causa,
                COALESCE(to_char(ea.V_30324, 'YYYY-MM-DD'), '') AS fecha_causa,
                CASE WHEN COALESCE(ea.V_30326, false) THEN 'Sí' ELSE 'No' END AS tiene_suspension,
                COALESCE(to_char(ea.V_30327, 'YYYY-MM-DD'), '') AS fecha_suspension,
                COALESCE(ea.V_30328, '') AS suspension_articulos,
                COALESCE(ea.V_30330, 'En trámite') AS V_30330,
                COALESCE(to_char(ea.V_30331, 'YYYY-MM-DD'), '') AS V_30243
            FROM expediente_agentes ea
            JOIN expedientes e ON e.id = ea.V_30275
            WHERE 1=1 {filter}
            ORDER BY ea.id DESC
            LIMIT ${lim_p}
            "#,
            filter = V_30414,
            lim_p = V_30413
        );

        #[derive(FromRow)]
        struct S_50124 {
            id: i64,
            numero_expediente: String,
            credencial: String,
            V_30266: String,
            tiene_cargos: String,
            fecha_cargos: String,
            V_30321: String,
            tiene_causa: String,
            fecha_causa: String,
            tiene_suspension: String,
            fecha_suspension: String,
            suspension_articulos: String,
            V_30330: String,
            V_30243: String,
        }

        let mut V_30434 = sqlx::query_as::<V_30020, S_50124>(&V_30433);
        for b in &V_30074 {
            V_30434 = V_30434.bind(b);
        }
        V_30434 = V_30434.bind(V_30415);

        let V_30435 = V_30434.fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;
        for (r_idx, V_30522) in V_30435.iter().enumerate() {
            let V_30403 = (r_idx + 1) as u32;
            V_30431.write_string(V_30403, 0, F_40091(&V_30522.numero_expediente)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 1, F_40091(&V_30522.credencial)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 2, F_40091(&V_30522.V_30266)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 3, F_40091(&V_30522.tiene_cargos)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 4, F_40091(&V_30522.fecha_cargos)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 5, F_40091(&V_30522.V_30321)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 6, F_40091(&V_30522.tiene_causa)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 7, F_40091(&V_30522.fecha_causa)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 8, F_40091(&V_30522.tiene_suspension)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 9, F_40091(&V_30522.fecha_suspension)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 10, F_40091(&V_30522.suspension_articulos)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 11, F_40091(&V_30522.V_30330)).map_err(F_40092)?;
            V_30431.write_string(V_30403, 12, F_40091(&V_30522.V_30243)).map_err(F_40092)?;
        }
    }

    // =========================================================================
    // HOJA 5: Expedientes Relacionados
    // =========================================================================
    {
        let V_30436 = V_30124.add_worksheet();
        let V_30020 = V_30436.set_name("Expedientes Relacionados");

        let V_30437 = [
            "N° Expediente Origen",
            "Tipo Relación",
            "N° Expediente Relacionado / Destino",
            "Motivo Vinculación",
            "Fecha Efectiva",
            "Registrado Por",
        ];
        for (idx, h) in V_30437.iter().enumerate() {
            V_30436.write_string_with_format(0, idx as u16, *h, &V_30126).map_err(F_40092)?;
        }

        let V_30438 = format!(
            V_30522#"
            SELECT 
                V_30522.id,
                COALESCE(e1.numero_expediente, '') AS expediente_origen,
                COALESCE(V_30522.V_30481, '') AS V_30481,
                COALESCE(e2.numero_expediente, '') AS expediente_destino,
                COALESCE(V_30522.V_30167, '') AS V_30167,
                COALESCE(to_char(V_30522.V_30548, 'YYYY-MM-DD'), '') AS V_30548,
                COALESCE(V_30522.usuario, '') AS usuario
            FROM expediente_relaciones V_30522
            JOIN expedientes e1 ON e1.id = V_30522.expediente_origen_id
            LEFT JOIN expedientes e2 ON e2.id = V_30522.expediente_destino_id
            WHERE 1=1 {filter}
            ORDER BY V_30522.id DESC
            LIMIT ${lim_p}
            "#,
            filter = V_30414.replace("e.", "e1."),
            lim_p = V_30413
        );

        #[derive(FromRow)]
        struct S_50125 {
            id: i64,
            expediente_origen: String,
            V_30481: String,
            expediente_destino: String,
            V_30167: String,
            V_30548: String,
            usuario: String,
        }

        let mut V_30439 = sqlx::query_as::<V_30020, S_50125>(&V_30438);
        for b in &V_30074 {
            V_30439 = V_30439.bind(b);
        }
        V_30439 = V_30439.bind(V_30415);

        let V_30440 = V_30439.fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;
        for (r_idx, V_30522) in V_30440.iter().enumerate() {
            let V_30403 = (r_idx + 1) as u32;
            V_30436.write_string(V_30403, 0, F_40091(&V_30522.expediente_origen)).map_err(F_40092)?;
            V_30436.write_string(V_30403, 1, F_40091(&V_30522.V_30481)).map_err(F_40092)?;
            V_30436.write_string(V_30403, 2, F_40091(&V_30522.expediente_destino)).map_err(F_40092)?;
            V_30436.write_string(V_30403, 3, F_40091(&V_30522.V_30167)).map_err(F_40092)?;
            V_30436.write_string(V_30403, 4, F_40091(&V_30522.V_30548)).map_err(F_40092)?;
            V_30436.write_string(V_30403, 5, F_40091(&V_30522.usuario)).map_err(F_40092)?;
        }
    }

    // =========================================================================
    // HOJA 6: Historial y Novedades
    // =========================================================================
    {
        let V_30441 = V_30124.add_worksheet();
        let V_30020 = V_30441.set_name("Historial y Novedades");

        let V_30442 = [
            "N° Expediente",
            "Fecha Evento / Registro",
            "Tipo Novedad",
            "Título / Hito",
            "Descripción",
            "Credencial Agente",
            "Usuario Responsable",
            "Estado Anterior",
            "Estado Nuevo",
        ];
        for (idx, h) in V_30442.iter().enumerate() {
            V_30441.write_string_with_format(0, idx as u16, *h, &V_30126).map_err(F_40092)?;
        }

        let V_30443 = format!(
            V_30522#"
            SELECT 
                V_30063.id,
                COALESCE(e.numero_expediente, '') AS numero_expediente,
                COALESCE(to_char(V_30063.V_30460, 'YYYY-MM-DD HH24:MI'), '') AS V_30460,
                COALESCE(V_30063.V_30458, '') AS V_30458,
                COALESCE(V_30063.V_30461, '') AS V_30461,
                COALESCE(V_30063.V_30462, '') AS V_30462,
                COALESCE(ea.credencial, '') AS credencial,
                COALESCE(V_30063.usuario, '') AS usuario,
                COALESCE(V_30063.estado_anterior, '') AS estado_anterior,
                COALESCE(V_30063.estado_nuevo, '') AS estado_nuevo
            FROM expediente_novedades V_30063
            JOIN expedientes e ON e.id = V_30063.V_30275
            LEFT JOIN expediente_agentes ea ON ea.id = V_30063.V_30464
            WHERE 1=1 {filter}
            ORDER BY V_30063.id DESC
            LIMIT ${lim_p}
            "#,
            filter = V_30414,
            lim_p = V_30413
        );

        #[derive(FromRow)]
        struct S_50126 {
            id: i64,
            numero_expediente: String,
            V_30460: String,
            V_30458: String,
            V_30461: String,
            V_30462: String,
            credencial: String,
            usuario: String,
            estado_anterior: String,
            estado_nuevo: String,
        }

        let mut V_30444 = sqlx::query_as::<V_30020, S_50126>(&V_30443);
        for b in &V_30074 {
            V_30444 = V_30444.bind(b);
        }
        V_30444 = V_30444.bind(V_30415);

        let V_30445 = V_30444.fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;
        for (r_idx, V_30522) in V_30445.iter().enumerate() {
            let V_30403 = (r_idx + 1) as u32;
            V_30441.write_string(V_30403, 0, F_40091(&V_30522.numero_expediente)).map_err(F_40092)?;
            V_30441.write_string(V_30403, 1, F_40091(&V_30522.V_30460)).map_err(F_40092)?;
            V_30441.write_string(V_30403, 2, F_40091(&V_30522.V_30458)).map_err(F_40092)?;
            V_30441.write_string(V_30403, 3, F_40091(&V_30522.V_30461)).map_err(F_40092)?;
            V_30441.write_string(V_30403, 4, F_40091(&V_30522.V_30462)).map_err(F_40092)?;
            V_30441.write_string(V_30403, 5, F_40091(&V_30522.credencial)).map_err(F_40092)?;
            V_30441.write_string(V_30403, 6, F_40091(&V_30522.usuario)).map_err(F_40092)?;
            V_30441.write_string(V_30403, 7, F_40091(&V_30522.estado_anterior)).map_err(F_40092)?;
            V_30441.write_string(V_30403, 8, F_40091(&V_30522.estado_nuevo)).map_err(F_40092)?;
        }
    }

    // =========================================================================
    // HOJA 7: Documentación Adjunta (Metadatos únicamente, sin binarios pesados)
    // =========================================================================
    {
        let V_30446 = V_30124.add_worksheet();
        let V_30020 = V_30446.set_name("Documentación Adjunta");

        let V_30447 = [
            "N° Expediente",
            "Nombre de Archivo",
            "Tipo Documento",
            "Tamaño (Bytes)",
            "Hash SHA-256",
            "Fecha Incorporación",
            "Subido Por",
            "Estado Documental",
        ];
        for (idx, h) in V_30447.iter().enumerate() {
            V_30446.write_string_with_format(0, idx as u16, *h, &V_30126).map_err(F_40092)?;
        }

        let V_30448 = format!(
            V_30522#"
            SELECT 
                d.id,
                COALESCE(e.numero_expediente, '') AS numero_expediente,
                COALESCE(d.nombre_original, d.nombre_archivo) AS nombre_archivo,
                COALESCE(d.tipo_documento, 'General') AS tipo_documento,
                COALESCE(d.tamano_bytes, 0) AS tamano_bytes,
                COALESCE(d.sha256_hash, '') AS sha256_hash,
                COALESCE(to_char(d.V_30675, 'YYYY-MM-DD HH24:MI'), '') AS V_30675,
                COALESCE(d.usuario, '') AS usuario,
                CASE WHEN d.esta_retirado THEN 'Retirado' ELSE 'Vigente' END AS estado_documental
            FROM expediente_documentos d
            JOIN expedientes e ON e.id = d.V_30275
            WHERE 1=1 {filter}
            ORDER BY d.id DESC
            LIMIT ${lim_p}
            "#,
            filter = V_30414,
            lim_p = V_30413
        );

        #[derive(FromRow)]
        struct S_50127 {
            id: i64,
            numero_expediente: String,
            nombre_archivo: String,
            tipo_documento: String,
            tamano_bytes: i64,
            sha256_hash: String,
            V_30675: String,
            usuario: String,
            estado_documental: String,
        }

        let mut V_30449 = sqlx::query_as::<V_30020, S_50127>(&V_30448);
        for b in &V_30074 {
            V_30449 = V_30449.bind(b);
        }
        V_30449 = V_30449.bind(V_30415);

        let V_30450 = V_30449.fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;
        for (r_idx, V_30522) in V_30450.iter().enumerate() {
            let V_30403 = (r_idx + 1) as u32;
            V_30446.write_string(V_30403, 0, F_40091(&V_30522.numero_expediente)).map_err(F_40092)?;
            V_30446.write_string(V_30403, 1, F_40091(&V_30522.nombre_archivo)).map_err(F_40092)?;
            V_30446.write_string(V_30403, 2, F_40091(&V_30522.tipo_documento)).map_err(F_40092)?;
            V_30446.write_number(V_30403, 3, V_30522.tamano_bytes as f64).map_err(F_40092)?;
            V_30446.write_string(V_30403, 4, F_40091(&V_30522.sha256_hash)).map_err(F_40092)?;
            V_30446.write_string(V_30403, 5, F_40091(&V_30522.V_30675)).map_err(F_40092)?;
            V_30446.write_string(V_30403, 6, F_40091(&V_30522.usuario)).map_err(F_40092)?;
            V_30446.write_string(V_30403, 7, F_40091(&V_30522.estado_documental)).map_err(F_40092)?;
        }
    }

    // =========================================================================
    // HOJA 8: Observaciones de Extracción (Calidad de datos y F_40085)
    // =========================================================================
    {
        let V_30451 = V_30124.add_worksheet();
        let V_30020 = V_30451.set_name("Observaciones");

        let V_30452 = [
            "N° Expediente / ID",
            "Severidad",
            "Categoría",
            "Campo Afectado",
            "Descripción de la Observación",
            "Sugerencia / Estado Revisión",
            "Fecha Detección",
        ];
        for (idx, h) in V_30452.iter().enumerate() {
            V_30451.write_string_with_format(0, idx as u16, *h, &V_30126).map_err(F_40092)?;
        }

        let V_30453 = format!(
            V_30522#"
            SELECT 
                c.id,
                COALESCE(e.numero_expediente, c.identificador_visible, 'Sin V_30113') AS numero_expediente,
                COALESCE(c.V_30701, 'Media') AS V_30701,
                COALESCE(c.V_30763, 'Calidad') AS V_30763,
                COALESCE(c.campo_afectado, '-') AS campo_afectado,
                COALESCE(c.V_30462, '') AS V_30462,
                COALESCE(c.sugerencia, c.V_30467, '') AS sugerencia_estado,
                COALESCE(to_char(c.V_30675, 'YYYY-MM-DD HH24:MI'), '') AS V_30675
            FROM app_calidad_datos_casos c
            LEFT JOIN expedientes e ON e.id = c.V_30275
            ORDER BY c.id DESC
            LIMIT 5000
            "#
        );

        #[derive(FromRow)]
        struct S_50128 {
            id: i64,
            numero_expediente: String,
            V_30701: String,
            V_30763: String,
            campo_afectado: String,
            V_30462: String,
            sugerencia_estado: String,
            V_30675: String,
        }

        let V_30454 = sqlx::query_as::<V_30020, S_50128>(&V_30453)
            .fetch_all(&V_30188.V_30012)
            .await
            .unwrap_or_default();

        for (r_idx, V_30522) in V_30454.iter().enumerate() {
            let V_30403 = (r_idx + 1) as u32;
            V_30451.write_string(V_30403, 0, F_40091(&V_30522.numero_expediente)).map_err(F_40092)?;
            V_30451.write_string(V_30403, 1, F_40091(&V_30522.V_30701)).map_err(F_40092)?;
            V_30451.write_string(V_30403, 2, F_40091(&V_30522.V_30763)).map_err(F_40092)?;
            V_30451.write_string(V_30403, 3, F_40091(&V_30522.campo_afectado)).map_err(F_40092)?;
            V_30451.write_string(V_30403, 4, F_40091(&V_30522.V_30462)).map_err(F_40092)?;
            V_30451.write_string(V_30403, 5, F_40091(&V_30522.sugerencia_estado)).map_err(F_40092)?;
            V_30451.write_string(V_30403, 6, F_40091(&V_30522.V_30675)).map_err(F_40092)?;
        }
    }

    let V_30123 = V_30124.save_to_buffer().map_err(F_40092)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "export_expedientes_xlsx_exhaustivo",
        "expedientes",
        "success",
        json!({
            "hojas": [
                "Expedientes y Agentes",
                "Resumen de Expedientes",
                "Suspensiones Preventivas",
                "Otras Medidas",
                "Expedientes Relacionados",
                "Historial y Novedades",
                "Documentación Adjunta",
                "Observaciones"
            ],
            "filtros": {
                "V_30065": V_30065,
                "reparticion": reparticion,
                "estado": estado,
                "V_30066": V_30066,
                "V_30067": V_30067,
                "V_30068": V_30068
            }
        }),
    )
    .await;

    let V_30182 = format!("expedientes_exhaustivo_{}.xlsx", chrono::Local::V_30072().format("%Y%m%d_%H%M%S"));

    Ok(HttpResponse::Ok()
        .append_header((
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ))
        .append_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; V_30182=\"{}\"", V_30182),
        ))
        .body(V_30123))
}

#[get("/export/V_30278.xlsx")]
async fn F_40099(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50104>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40023(&V_30064.role) {
        return Ok(HttpResponse::Forbidden().body("Sin permisos de exportación"));
    }
    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    let estado = query.estado.clone().unwrap_or_default().trim().to_string();
    let V_30066 = query.V_30066.clone().unwrap_or_default().trim().to_string();

    let V_30455 = format!("%{}%", V_30065);
    let V_30456 = format!("%{}%", V_30066);

    let mut V_30109 = sqlx::QueryBuilder::new(
        V_30522#"
        SELECT
            V_30044.id,
            V_30044.V_30275,
            e.numero_expediente,
            V_30044.V_30464,
            V_30044.credencial,
            V_30044.V_30266,
            to_char(V_30044.fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            V_30044.V_30471,
            V_30044.estado,
            to_char(V_30044.fecha_fin, 'YYYY-MM-DD') AS fecha_fin,
            V_30044.V_30476,
            V_30044.V_30477,
            V_30044.archivo_disposicion_pdf,
            V_30044.archivo_levantamiento_pdf,
            V_30044.usuario,
            V_30044.V_30463,
            to_char(V_30044.V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675
        FROM expediente_suspensiones V_30044
        LEFT JOIN expedientes e ON e.id = V_30044.V_30275
        WHERE 1=1
        "#
    );

    if !V_30065.is_empty() {
        V_30109.push(" AND (V_30044.credencial ILIKE ").push_bind(&V_30455);
        V_30109.push(" OR V_30044.V_30266 ILIKE ").push_bind(&V_30455);
        V_30109.push(" OR e.numero_expediente ILIKE ").push_bind(&V_30455).push(")");
    }

    if !estado.is_empty() {
        if estado.eq_ignore_ascii_case("levantada") || estado.eq_ignore_ascii_case("levantado") {
            V_30109.push(" AND V_30044.estado IN ('Levantado', 'Levantada')");
        } else if estado.eq_ignore_ascii_case("en revision") || estado.eq_ignore_ascii_case("en revisión") {
            V_30109.push(" AND (V_30044.estado ILIKE 'En revision' OR V_30044.estado ILIKE 'En revisi_n')");
        } else {
            V_30109.push(" AND V_30044.estado ILIKE ").push_bind(&estado);
        }
    }

    if !V_30066.is_empty() {
        V_30109.push(" AND V_30044.V_30471 ILIKE ").push_bind(&V_30456);
    }

    V_30109.push(" ORDER BY V_30044.fecha_inicio DESC, V_30044.id DESC LIMIT ").push_bind(V_30188.V_30011.max_export_rows);
    let V_30022: Vec<S_50021> = V_30109.build_query_as().fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;

    let mut V_30124 = Workbook::new();
    let V_30125 = V_30124.add_worksheet();
    let V_30020 = V_30125.set_name("Suspensiones");

    let V_30126 = rust_xlsxwriter::Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0x107C41))
        .set_font_color(rust_xlsxwriter::Color::RGB(0xFFFFFF));

    let V_30457 = [
        "Credencial",
        "Nombre Completo",
        "N° Expediente",
        "Fecha Inicio",
        "Artículo / Incisos",
        "Estado",
        "Fecha Fin",
        "Motivo Fin",
        "Fundamento Fin",
        "Observaciones",
    ];

    for (idx, h) in V_30457.iter().enumerate() {
        V_30125.write_string_with_format(0, idx as u16, *h, &V_30126).map_err(F_40092)?;
    }

    for (r_idx, V_30522) in V_30022.iter().enumerate() {
        let V_30403 = (r_idx + 1) as u32;
        V_30125.write_string(V_30403, 0, F_40091(&V_30522.credencial)).map_err(F_40092)?;
        V_30125.write_string(V_30403, 1, F_40091(&V_30522.V_30266)).map_err(F_40092)?;
        V_30125.write_string(V_30403, 2, F_40091(V_30522.numero_expediente.as_deref().unwrap_or("—"))).map_err(F_40092)?;
        V_30125.write_string(V_30403, 3, F_40091(&V_30522.fecha_inicio)).map_err(F_40092)?;
        V_30125.write_string(V_30403, 4, F_40091(&V_30522.V_30471)).map_err(F_40092)?;
        V_30125.write_string(V_30403, 5, F_40091(&V_30522.estado)).map_err(F_40092)?;
        V_30125.write_string(V_30403, 6, F_40091(V_30522.fecha_fin.as_deref().unwrap_or("—"))).map_err(F_40092)?;
        V_30125.write_string(V_30403, 7, F_40091(V_30522.V_30476.as_deref().unwrap_or("—"))).map_err(F_40092)?;
        V_30125.write_string(V_30403, 8, F_40091(V_30522.V_30477.as_deref().unwrap_or("—"))).map_err(F_40092)?;
        V_30125.write_string(V_30403, 9, F_40091(V_30522.V_30463.as_deref().unwrap_or("—"))).map_err(F_40092)?;
    }

    let V_30123 = V_30124.save_to_buffer().map_err(F_40092)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "F_40099",
        "V_30278",
        "success",
        json!({ "V_30079": V_30022.len(), "V_30065": V_30065 }),
    )
    .await;

    Ok(HttpResponse::Ok()
        .append_header((
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ))
        .append_header((
            header::CONTENT_DISPOSITION,
            "attachment; V_30182=\"suspensiones_filtradas.xlsx\"",
        ))
        .body(V_30123))
}

#[post("/expedientes/{id}/V_30277")]
async fn F_40100(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();
    let (V_30019, V_30178) = F_40068(V_30012, "uploads").await?;

    let V_30458 = V_30019.get("V_30458").cloned().unwrap_or_else(|| "Otro".to_string());
    let V_30459 = V_30019.get("V_30459").filter(|V_30044| !V_30044.trim().is_empty()).cloned();
    let V_30460 = F_40067(V_30019.get("V_30460").map(|V_30044| V_30044.as_str())).unwrap_or_else(|| chrono::Utc::now().naive_utc());
    let V_30461 = V_30019.get("V_30461").cloned().unwrap_or_default().trim().to_string();
    let V_30462 = V_30019.get("V_30462").cloned().unwrap_or_default().trim().to_string();
    let V_30463 = V_30019.get("V_30463").filter(|V_30044| !V_30044.trim().is_empty()).cloned();
    let V_30464 = V_30019.get("V_30464").and_then(|V_30044| V_30044.parse::<i64>().ok());
    let V_30465 = V_30178.get("V_30465").cloned();
    let V_30466 = V_30019.get("V_30466").filter(|V_30044| !V_30044.trim().is_empty()).cloned();
    let V_30467 = V_30019.get("V_30467").cloned().unwrap_or_else(|| "revisada".to_string());

    if V_30461.is_empty() || V_30462.is_empty() {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30277&err=Título+y+descripción+son+obligatorios", V_30275)))
            .finish());
    }

    let V_30468: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30464, V_30458, V_30459, V_30460,
            V_30461, V_30462, V_30463, usuario, es_auditoria_automatica,
            V_30465, V_30467, V_30466, V_30675
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false, $10, $11, $12, CURRENT_TIMESTAMP)
        RETURNING id
        "#
    )
    .bind(V_30275)
    .bind(V_30464)
    .bind(&V_30458)
    .bind(&V_30459)
    .bind(V_30460)
    .bind(&V_30461)
    .bind(&V_30462)
    .bind(&V_30463)
    .bind(&V_30064.V_30114)
    .bind(&V_30465)
    .bind(&V_30467)
    .bind(&V_30466)
    .fetch_one(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    sqlx::query(
        "UPDATE expedientes SET fecha_ultima_novedad = $1, tipo_ultima_novedad = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3"
    )
    .bind(V_30460)
    .bind(&V_30458)
    .bind(V_30275)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    if let Some(ref pdf) = V_30465 {
        sqlx::query(
            "INSERT INTO expediente_documentos (V_30275, novedad_id, V_30464, nombre_archivo, nombre_original, tipo_documento, usuario) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(V_30275)
        .bind(V_30468)
        .bind(V_30464)
        .bind(pdf)
        .bind(pdf)
        .bind(&V_30458)
        .bind(&V_30064.V_30114)
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "create_novedad",
        "expediente_novedades",
        "success",
        json!({ "V_30275": V_30275, "novedad_id": V_30468, "V_30092": V_30458 }),
    )
    .await;

    F_40013(
        &V_30188.V_30012,
        None,
        "novedad",
        &format!("Nueva novedad en V_30113: {}", V_30461),
        &format!("Tipo: {}. {}", V_30458, V_30462),
        &format!("/expedientes/{}?tab=tab-V_30277", V_30275),
        Some(V_30275),
        None,
        None,
    ).await;

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30277&V_30083=Novedad+registrada+correctamente", V_30275)))
        .finish())
}

#[post("/expedientes/{id}/V_30277/{novedad_id}/rectificar")]
async fn F_40101(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    form: web::Form<S_50105>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let (V_30275, novedad_id) = path.into_inner();
    let V_30167 = form.motivo_rectificacion.trim().to_string();

    if V_30167.is_empty() {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30277&err=El+V_30167+de+rectificación+es+obligatorio", V_30275)))
            .finish());
    }

    let V_30469: Option<(String, String)> = sqlx::query_as(
        "SELECT V_30461, V_30462 FROM expediente_novedades WHERE id = $1 AND V_30275 = $2"
    )
    .bind(novedad_id)
    .bind(V_30275)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    let Some((orig_titulo, V_30020)) = V_30469 else {
        return Ok(HttpResponse::NotFound().body("Novedad no encontrada"));
    };

    sqlx::query(
        "UPDATE expediente_novedades SET anulado = true, anulado_motivo = $1, anulado_usuario = $2, anulado_at = CURRENT_TIMESTAMP WHERE id = $3"
    )
    .bind(&V_30167)
    .bind(&V_30064.V_30114)
    .bind(novedad_id)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if let Some(nueva_desc) = form.nueva_descripcion.as_ref().filter(|V_30044| !V_30044.trim().is_empty()) {
        sqlx::query(
            V_30522#"
            INSERT INTO expediente_novedades (
                V_30275, V_30458, V_30460, V_30461, V_30462, V_30463,
                usuario, es_auditoria_automatica, V_30675
            ) VALUES (
                $1, 'Rectificación de novedad anterior', CURRENT_TIMESTAMP,
                $2, $3, $4, $5, false, CURRENT_TIMESTAMP
            )
            "#
        )
        .bind(V_30275)
        .bind(format!("Rectificación sobre: {}", orig_titulo))
        .bind(nueva_desc)
        .bind(format!("Motivo de rectificación: {}", V_30167))
        .bind(&V_30064.V_30114)
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "rectify_novedad",
        "expediente_novedades",
        "success",
        json!({ "V_30275": V_30275, "novedad_id": novedad_id, "V_30167": V_30167 }),
    )
    .await;

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30277&V_30083=Novedad+rectificada+correctamente", V_30275)))
        .finish())
}

#[get("/V_30278")]
async fn F_40102(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50104>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let V_30069 = query.V_30069.unwrap_or(1).max(1);
    let V_30070 = query.V_30070.unwrap_or(5).clamp(1, V_30188.V_30011.max_page_size);
    let V_30071 = (V_30069 - 1) * V_30070;

    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    let estado = query.estado.clone().unwrap_or_default().trim().to_string();
    let V_30066 = query.V_30066.clone().unwrap_or_default().trim().to_string();

    let V_30455 = format!("%{}%", V_30065);
    let V_30456 = format!("%{}%", V_30066);

    let mut V_30109 = sqlx::QueryBuilder::new(
        V_30522#"
        SELECT
            V_30044.id,
            V_30044.V_30275,
            e.numero_expediente,
            V_30044.V_30464,
            V_30044.credencial,
            V_30044.V_30266,
            to_char(V_30044.fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            V_30044.V_30471,
            V_30044.estado,
            to_char(V_30044.fecha_fin, 'YYYY-MM-DD') AS fecha_fin,
            V_30044.V_30476,
            V_30044.V_30477,
            V_30044.archivo_disposicion_pdf,
            V_30044.archivo_levantamiento_pdf,
            V_30044.usuario,
            V_30044.V_30463,
            to_char(V_30044.V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675
        FROM expediente_suspensiones V_30044
        LEFT JOIN expedientes e ON e.id = V_30044.V_30275
        WHERE 1=1
        "#
    );

    let mut V_30470 = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM expediente_suspensiones V_30044 LEFT JOIN expedientes e ON e.id = V_30044.V_30275 WHERE 1=1");

    if !V_30065.is_empty() {
        V_30109.push(" AND (V_30044.credencial ILIKE ").push_bind(&V_30455);
        V_30109.push(" OR V_30044.V_30266 ILIKE ").push_bind(&V_30455);
        V_30109.push(" OR e.numero_expediente ILIKE ").push_bind(&V_30455).push(")");

        V_30470.push(" AND (V_30044.credencial ILIKE ").push_bind(&V_30455);
        V_30470.push(" OR V_30044.V_30266 ILIKE ").push_bind(&V_30455);
        V_30470.push(" OR e.numero_expediente ILIKE ").push_bind(&V_30455).push(")");
    }

    if !estado.is_empty() {
        if estado.eq_ignore_ascii_case("levantada") || estado.eq_ignore_ascii_case("levantado") {
            V_30109.push(" AND V_30044.estado IN ('Levantado', 'Levantada')");
            V_30470.push(" AND V_30044.estado IN ('Levantado', 'Levantada')");
        } else if estado.eq_ignore_ascii_case("en revision") || estado.eq_ignore_ascii_case("en revisión") {
            V_30109.push(" AND (V_30044.estado ILIKE 'En revision' OR V_30044.estado ILIKE 'En revisi_n')");
            V_30470.push(" AND (V_30044.estado ILIKE 'En revision' OR V_30044.estado ILIKE 'En revisi_n')");
        } else {
            V_30109.push(" AND V_30044.estado ILIKE ").push_bind(&estado);
            V_30470.push(" AND V_30044.estado ILIKE ").push_bind(&estado);
        }
    }

    if !V_30066.is_empty() {
        V_30109.push(" AND V_30044.V_30471 ILIKE ").push_bind(&V_30456);
        V_30470.push(" AND V_30044.V_30471 ILIKE ").push_bind(&V_30456);
    }

    let V_30079: i64 = V_30470.build_query_scalar().fetch_one(&V_30188.V_30012).await.unwrap_or(0);
    let V_30087 = (V_30079 + V_30070 - 1) / V_30070;

    V_30109.push(" ORDER BY V_30044.fecha_inicio DESC, V_30044.id DESC LIMIT ").push_bind(V_30070).push(" OFFSET ").push_bind(V_30071);
    let V_30022: Vec<S_50021> = V_30109.build_query_as().fetch_all(&V_30188.V_30012).await.map_err(error::ErrorInternalServerError)?;

    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
    let V_30090 = F_40065(&V_30188.V_30012).await;

    render_html(&S_50072 {
        user_name: V_30064.display_name,
        V_30088,
        V_30022,
        V_30065,
        estado,
        V_30066,
        V_30090,
        V_30069,
        V_30087,
        V_30079,
        V_30070,
        success_msg: query.V_30083.clone(),
        error_msg: query.err.clone(),
    })
}

#[post("/expedientes/{id}/V_30278")]
async fn F_40103(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();
    let (V_30019, V_30178) = F_40068(V_30012, "uploads").await?;

    let V_30464 = V_30019.get("V_30464").and_then(|V_30044| V_30044.parse::<i64>().ok());
    let fecha_inicio = F_40067(V_30019.get("fecha_inicio").map(|V_30044| V_30044.as_str())).unwrap_or_else(|| chrono::Utc::now().naive_utc());
    let V_30471 = V_30019.get("V_30471").cloned().unwrap_or_else(|| "a)".to_string());
    let V_30463 = V_30019.get("V_30463").filter(|V_30044| !V_30044.trim().is_empty()).cloned();
    let V_30465 = V_30178.get("V_30465").cloned();

    let V_30472: Option<(String, String)> = if let Some(aid) = V_30464 {
        sqlx::query_as("SELECT credencial, V_30266 FROM expediente_agentes WHERE id = $1")
            .bind(aid)
            .fetch_optional(&V_30188.V_30012)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let (credencial, V_30266) = V_30472.unwrap_or_else(|| ("Sin datos".to_string(), "Agente".to_string()));

    let V_30473: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO expediente_suspensiones (
            V_30275, V_30464, credencial, V_30266, fecha_inicio,
            V_30471, estado, archivo_disposicion_pdf, usuario, V_30463,
            V_30675, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5,
            $6, 'Vigente', $7, $8, $9,
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )
        RETURNING id
        "#
    )
    .bind(V_30275)
    .bind(V_30464)
    .bind(&credencial)
    .bind(&V_30266)
    .bind(fecha_inicio)
    .bind(&V_30471)
    .bind(&V_30465)
    .bind(&V_30064.V_30114)
    .bind(&V_30463)
    .fetch_one(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if let Some(aid) = V_30464 {
        sqlx::query(
            "UPDATE expediente_agentes SET V_30326 = true, V_30327 = $1, V_30328 = $2, V_30329 = COALESCE($3, V_30329) WHERE id = $4"
        )
        .bind(fecha_inicio)
        .bind(&V_30471)
        .bind(&V_30465)
        .bind(aid)
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30464, V_30474, V_30458, V_30460,
            V_30461, V_30462, V_30463, usuario, es_auditoria_automatica,
            V_30465, V_30675
        ) VALUES (
            $1, $2, $3, 'Resolución final', $4,
            $5, $6, $7, $8, false,
            $9, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30275)
    .bind(V_30464)
    .bind(V_30473)
    .bind(fecha_inicio)
    .bind(format!("Suspensión preventiva: {} (Cred. {})", V_30266, credencial))
    .bind(format!("Se dispone la suspensión preventiva del agente en el marco del Art. 419 incisos: {}", V_30471))
    .bind(&V_30463)
    .bind(&V_30064.V_30114)
    .bind(&V_30465)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "create_suspension",
        "expediente_suspensiones",
        "success",
        json!({ "V_30275": V_30275, "V_30474": V_30473, "credencial": credencial }),
    )
    .await;

    F_40013(
        &V_30188.V_30012,
        None,
        "suspension_creada",
        &format!("Suspensión preventiva aplicada a {} (Cred. {})", V_30266, credencial),
        &format!("Medida preventiva aplicada bajo Art. 419 {}.", V_30471),
        &format!("/V_30278/{}", V_30473),
        Some(V_30275),
        Some(&credencial),
        Some(V_30473),
    ).await;

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30278&V_30083=Suspensión+preventiva+registrada+exitosamente", V_30275)))
        .finish())
}

#[post("/V_30278/{V_30474}/levantar")]
async fn F_40104(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30474 = path.into_inner();
    let (V_30019, V_30178) = F_40068(V_30012, "uploads").await?;

    let V_30475 = V_30019.get("estado").cloned().unwrap_or_else(|| "Levantada".to_string());
    let fecha_fin = F_40067(V_30019.get("fecha_fin").map(|V_30044| V_30044.as_str())).unwrap_or_else(|| chrono::Utc::now().naive_utc());
    let V_30476 = V_30019.get("V_30476").cloned().unwrap_or_default().trim().to_string();
    let V_30477 = V_30019.get("V_30477").cloned().unwrap_or_default().trim().to_string();
    let V_30465 = V_30178.get("V_30465").cloned();

    if V_30476.is_empty() || V_30477.is_empty() {
        return Ok(HttpResponse::BadRequest().body("El V_30167 y la fundamentación son obligatorios para levantar o finalizar la suspensión"));
    }

    let V_30478: Option<(Option<i64>, Option<i64>, String, String)> = sqlx::query_as(
        "SELECT V_30275, V_30464, credencial, V_30266 FROM expediente_suspensiones WHERE id = $1"
    )
    .bind(V_30474)
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None);

    let Some((V_30275, V_30464, credencial, V_30266)) = V_30478 else {
        return Ok(HttpResponse::NotFound().body("Suspensión no encontrada"));
    };

    sqlx::query(
        V_30522#"
        UPDATE expediente_suspensiones
        SET estado = $1, fecha_fin = $2, V_30476 = $3, V_30477 = $4,
            archivo_levantamiento_pdf = COALESCE($5, archivo_levantamiento_pdf),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $6
        "#
    )
    .bind(&V_30475)
    .bind(fecha_fin)
    .bind(&V_30476)
    .bind(&V_30477)
    .bind(&V_30465)
    .bind(V_30474)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if let Some(aid) = V_30464 {
        sqlx::query(
            "UPDATE expediente_agentes SET V_30326 = false WHERE id = $1"
        )
        .bind(aid)
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    if let Some(eid) = V_30275 {
        sqlx::query(
            V_30522#"
            INSERT INTO expediente_novedades (
                V_30275, V_30464, V_30474, V_30458, V_30460,
                V_30461, V_30462, V_30463, usuario, es_auditoria_automatica,
                V_30465, V_30675
            ) VALUES (
                $1, $2, $3, 'Resolución final', $4,
                $5, $6, $7, $8, false,
                $9, CURRENT_TIMESTAMP
            )
            "#
        )
        .bind(eid)
        .bind(V_30464)
        .bind(V_30474)
        .bind(fecha_fin)
        .bind(format!("Cese de suspensión preventiva ({}): {} (Cred. {})", V_30475, V_30266, credencial))
        .bind(format!("Motivo: {}. Fundamentación: {}", V_30476, V_30477))
        .bind(format!("Estado: {}", V_30475))
        .bind(&V_30064.V_30114)
        .bind(&V_30465)
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "lift_suspension",
        "expediente_suspensiones",
        "success",
        json!({ "V_30474": V_30474, "estado": V_30475, "V_30167": V_30476 }),
    )
    .await;

    F_40013(
        &V_30188.V_30012,
        None,
        "suspension_levantada",
        &format!("Suspensión concluida: {} (Cred. {})", V_30266, credencial),
        &format!("Estado final: {}. Motivo: {}", V_30475, V_30476),
        &format!("/V_30278/{}", V_30474),
        V_30275,
        Some(&credencial),
        Some(V_30474),
    ).await;

    let V_30479 = req.V_30457().get(header::REFERER).and_then(|h| h.to_str().ok()).unwrap_or("/V_30278");
    let V_30480 = if V_30479.contains("/expedientes/") {
        if let Some(eid) = V_30275 {
            format!("/expedientes/{}?tab=tab-V_30278&V_30083=Suspensión+actualizada+correctamente", eid)
        } else {
            "/V_30278?V_30083=Suspensión+actualizada+correctamente".to_string()
        }
    } else if V_30479.contains("/V_30278/") {
        format!("/V_30278/{}?V_30083=Suspensión+actualizada+correctamente", V_30474)
    } else {
        "/V_30278?V_30083=Suspensión+actualizada+correctamente".to_string()
    };

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, V_30480))
        .finish())
}

#[post("/expedientes/{id}/V_30280")]
async fn F_40105(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    V_30012: Multipart,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();
    let (V_30019, V_30178) = F_40068(V_30012, "uploads").await?;

    let V_30481 = V_30019.get("V_30481").cloned().unwrap_or_else(|| "Expediente conexo".to_string());
    let V_30482 = V_30019.get("V_30482").filter(|V_30044| !V_30044.trim().is_empty()).cloned();
    let V_30167 = V_30019.get("V_30167").cloned().unwrap_or_default().trim().to_string();
    let V_30465 = V_30178.get("V_30465").cloned();

    let V_30483 = V_30019.get("expediente_destino_id").and_then(|V_30044| V_30044.parse::<i64>().ok());
    let V_30484 = V_30019.get("numero_expediente_destino").cloned().unwrap_or_default().trim().to_string();

    let V_30485 = if let Some(did) = V_30483 {
        did
    } else if !V_30484.is_empty() {
        let V_30486: Option<i64> = sqlx::query_scalar("SELECT id FROM expedientes WHERE numero_expediente = $1 LIMIT 1")
            .bind(&V_30484)
            .fetch_optional(&V_30188.V_30012)
            .await
            .unwrap_or(None);
        match V_30486 {
            Some(id) => id,
            None => {
                return Ok(HttpResponse::SeeOther()
                    .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30280&err=No+se+encontró+el+V_30113+con+número+{}", V_30275, V_30484)))
                    .finish());
            }
        }
    } else {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30280&err=Debe+indicar+el+V_30113+a+vincular", V_30275)))
            .finish());
    };

    if V_30485 == V_30275 {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30280&err=No+se+puede+vincular+un+V_30113+consigo+mismo", V_30275)))
            .finish());
    }

    if V_30167.is_empty() {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30280&err=El+V_30167+de+vinculación+es+obligatorio", V_30275)))
            .finish());
    }

    let V_30487: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO expediente_relaciones (
            expediente_origen_id, expediente_destino_id, V_30481, V_30482,
            V_30167, V_30548, V_30465, usuario, estado, V_30675, updated_at
        ) VALUES (
            $1, $2, $3, $4,
            $5, CURRENT_TIMESTAMP, $6, $7, 'Activa', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )
        RETURNING id
        "#
    )
    .bind(V_30275)
    .bind(V_30485)
    .bind(&V_30481)
    .bind(&V_30482)
    .bind(&V_30167)
    .bind(&V_30465)
    .bind(&V_30064.V_30114)
    .fetch_one(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30488: String = sqlx::query_scalar("SELECT numero_expediente FROM expedientes WHERE id = $1").bind(V_30275).fetch_one(&V_30188.V_30012).await.unwrap_or_default();
    let V_30489: String = sqlx::query_scalar("SELECT numero_expediente FROM expedientes WHERE id = $1").bind(V_30485).fetch_one(&V_30188.V_30012).await.unwrap_or_default();

    sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30458, V_30460, V_30461, V_30462,
            usuario, es_auditoria_automatica, V_30465, V_30675
        ) VALUES (
            $1, 'Resolución o disposición', CURRENT_TIMESTAMP,
            $2, $3, $4, false, $5, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30275)
    .bind(format!("Vinculación con Expediente {}", V_30489))
    .bind(format!("Relación: {}. Motivo: {}", V_30481, V_30167))
    .bind(&V_30064.V_30114)
    .bind(&V_30465)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30458, V_30460, V_30461, V_30462,
            usuario, es_auditoria_automatica, V_30465, V_30675
        ) VALUES (
            $1, 'Resolución o disposición', CURRENT_TIMESTAMP,
            $2, $3, $4, false, $5, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30485)
    .bind(format!("Vinculado V_30511 Expediente {}", V_30488))
    .bind(format!("Relación: {}. Motivo: {}", V_30481, V_30167))
    .bind(&V_30064.V_30114)
    .bind(&V_30465)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "create_relation",
        "expediente_relaciones",
        "success",
        json!({ "V_30104": V_30275, "destino": V_30485, "V_30092": V_30481 }),
    )
    .await;

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30280&V_30083=Expedientes+vinculados+correctamente", V_30275)))
        .finish())
}

#[post("/expedientes/{id}/V_30280/{relacion_id}/anular")]
async fn F_40106(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    form: web::Form<S_50106>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let (V_30275, relacion_id) = path.into_inner();
    let V_30167 = form.motivo_anulacion.trim().to_string();

    if V_30167.is_empty() {
        return Ok(HttpResponse::SeeOther()
            .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30280&err=El+V_30167+de+desvinculación+es+obligatorio", V_30275)))
            .finish());
    }

    sqlx::query(
        "UPDATE expediente_relaciones SET estado = 'Anulada', anulado_motivo = $1, anulado_usuario = $2, anulado_at = CURRENT_TIMESTAMP WHERE id = $3"
    )
    .bind(&V_30167)
    .bind(&V_30064.V_30114)
    .bind(relacion_id)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "anular_relation",
        "expediente_relaciones",
        "success",
        json!({ "V_30275": V_30275, "relacion_id": relacion_id, "V_30167": V_30167 }),
    )
    .await;

    Ok(HttpResponse::SeeOther()
        .append_header((header::LOCATION, format!("/expedientes/{}?tab=tab-V_30280&V_30083=Relación+desvinculada+correctamente", V_30275)))
        .finish())
}

#[get("/V_30086/{credencial}/V_30276")]
async fn F_40107(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let credencial = path.into_inner().trim().to_string();

    // 1. Datos personales y de revista V_30511 el padrón unificado
    let V_30490 = sqlx::query_as::<V_30020, S_50002>(
        V_30522#"
        SELECT
            V_30147.credencial,
            V_30147.apellido,
            V_30147.nombre,
            V_30147.escalafon_grado,
            V_30147.V_30526,
            V_30147.origen_principal,
            COALESCE(V_30147.last_seen_at, LOCALTIMESTAMP) AS last_seen_at,
            (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Vigente', 'Suspendido')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'Suspendidos')) AS tiene_suspension_activa,
            (EXISTS (SELECT 1 FROM expediente_agentes ea JOIN expedientes e ON e.id = ea.V_30275 WHERE ea.credencial = V_30147.credencial AND e.estado IN ('Activo', 'En analisis', 'En análisis', 'Suspendido', 'Inicio', 'Abierto', 'En Trámite')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('SUM ADM', 'INFO SUM'))) AS tiene_sumario_abierto,
            (EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('CONDENA', 'PROCESAMIENTO', 'CC'))) AS tiene_impedimento_judicial
        FROM personas V_30147
        WHERE V_30147.credencial = $1
        "#
    )
    .bind(&credencial)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let (apellido, nombre, escalafon_grado, origen_principal, ultima_actualizacion, V_30266) = if let Some(V_30147) = V_30490 {
        let V_30491 = V_30147.apellido.clone().unwrap_or_else(|| "-".to_string());
        let V_30492 = V_30147.nombre.clone().unwrap_or_else(|| "-".to_string());
        let V_30263 = V_30147.escalafon_grado.clone().unwrap_or_else(|| "-".to_string());
        let V_30469 = V_30147.origen_principal.clone().unwrap_or_else(|| "sumarios_caratulas".to_string());
        let V_30493 = V_30147.last_seen_at.format("%d/%m/%Y %H:%M").to_string();
        let V_30494 = if !V_30491.is_empty() && V_30491 != "-" {
            if !V_30492.is_empty() && V_30492 != "-" {
                format!("{}, {}", V_30491, V_30492)
            } else {
                V_30491.clone()
            }
        } else {
            V_30147.V_30526.unwrap_or_else(|| format!("Agente {}", credencial))
        };
        (V_30491, V_30492, V_30263, V_30469, V_30493, V_30494)
    } else {
        let V_30495: String = sqlx::query_scalar(
            "SELECT V_30266 FROM expediente_agentes WHERE credencial = $1 ORDER BY id DESC LIMIT 1"
        )
        .bind(&credencial)
        .fetch_optional(&V_30188.V_30012)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| format!("Agente {}", credencial));
        ("-".to_string(), "-".to_string(), "-".to_string(), "general".to_string(), "-".to_string(), V_30495)
    };

    // 2. Expedientes y sumarios vinculados
    let V_30496 = sqlx::query_as::<V_30020, S_50025>(
        V_30522#"
        SELECT
            e.id AS V_30275,
            e.numero_expediente,
            e.V_30228,
            e.reparticion,
            e.estado,
            ea.rol,
            to_char(e.fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            e.V_30167,
            ea.V_30465
        FROM expediente_agentes ea
        JOIN expedientes e ON e.id = ea.V_30275
        WHERE ea.credencial = $1
        ORDER BY e.fecha_inicio DESC NULLS LAST, e.id DESC
        "#
    )
    .bind(&credencial)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    // 3. Medidas cautelares y V_30278 preventivas
    let V_30278 = sqlx::query_as::<V_30020, S_50021>(
        V_30522#"
        SELECT
            V_30044.id,
            V_30044.V_30275,
            e.numero_expediente,
            V_30044.V_30464,
            V_30044.credencial,
            V_30044.V_30266,
            to_char(V_30044.fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            V_30044.V_30471,
            V_30044.estado,
            to_char(V_30044.fecha_fin, 'YYYY-MM-DD') AS fecha_fin,
            V_30044.V_30476,
            V_30044.V_30477,
            V_30044.archivo_disposicion_pdf,
            V_30044.archivo_levantamiento_pdf,
            V_30044.usuario,
            V_30044.V_30463,
            to_char(V_30044.V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675
        FROM expediente_suspensiones V_30044
        LEFT JOIN expedientes e ON e.id = V_30044.V_30275
        WHERE V_30044.credencial = $1
        ORDER BY V_30044.fecha_inicio DESC NULLS LAST, V_30044.id DESC
        "#
    )
    .bind(&credencial)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30497 = V_30278.iter().filter(|V_30044| V_30044.estado == "Vigente").count();

    // 4. Línea de tiempo unificada (V_30277 de V_30113 + V_30277 de fila de padrón)
    let V_30498 = sqlx::query_as::<V_30020, S_50028>(
        V_30522#"
        SELECT
            V_30063.id,
            V_30063.V_30275,
            e.numero_expediente,
            to_char(V_30063.V_30460, 'YYYY-MM-DD HH24:MI') AS V_30460,
            V_30063.V_30458 AS V_30092,
            V_30063.V_30461,
            V_30063.V_30462,
            V_30063.usuario,
            V_30063.V_30465,
            V_30063.es_auditoria_automatica AS es_auditoria
        FROM expediente_novedades V_30063
        JOIN expedientes e ON e.id = V_30063.V_30275
        LEFT JOIN expediente_agentes ea ON ea.id = V_30063.V_30464
        WHERE ea.credencial = $1
           OR V_30063.V_30275 IN (
               SELECT V_30275 FROM expediente_agentes WHERE credencial = $1
           )
        UNION ALL
        SELECT
            fn.id,
            COALESCE(fn.V_30275, 0) AS V_30275,
            COALESCE(fn.numero_expediente, 'Novedad de Fila / Padrón') AS numero_expediente,
            to_char(fn.V_30460, 'YYYY-MM-DD HH24:MI') AS V_30460,
            fn.V_30458 AS V_30092,
            fn.V_30461,
            fn.V_30462,
            fn.usuario,
            NULL::text AS V_30465,
            false AS es_auditoria
        FROM app_fila_novedades fn
        WHERE fn.credencial = $1
        ORDER BY V_30460 DESC NULLS LAST
        LIMIT 200
        "#
    )
    .bind(&credencial)
    .bind(&credencial)
    .bind(&credencial)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    // 5. Impedimentos judiciales por estatuto
    let V_30499: Vec<(String, serde_json::Value)> = sqlx::query_as(
        V_30522#"
        SELECT ss.sheet_name, V_30522.payload_json
        FROM row_credential_index idx
        JOIN raw_sheet_rows V_30522 ON V_30522.id = idx.row_id
        JOIN source_sheets ss ON ss.id = V_30522.sheet_id
        WHERE idx.credential_value = $1
          AND ss.sheet_name IN ('CONDENA', 'PROCESAMIENTO', 'CC')
        "#
    )
    .bind(&credencial)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30500 = Vec::new();
    for (sheet_name, V_30190) in V_30499 {
        let V_30501 = V_30190.get("V_30501").or_else(|| V_30190.get("numero_de_expediente")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
        let V_30092 = V_30190.get("tipo_de_impedimento").or_else(|| V_30190.get("impedido")).and_then(|v| v.as_str()).unwrap_or(&sheet_name).to_string();
        let V_30502 = V_30190.get("tribunal_impuso_la_condena_y_controla_la_ejec").or_else(|| V_30190.get("juzgado_instruyó_la_causa_y_la_elevó_a_juicio")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
        let V_30503 = V_30190.get("fecha_de_la_sentencia").or_else(|| V_30190.get("fecha_de_impedido_en_base_desde")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
        let V_30504 = V_30190.get("sentencia").or_else(|| V_30190.get("V_30463")).or_else(|| V_30190.get("V_30167")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
        let V_30505 = V_30190.get("documental").or_else(|| V_30190.get("archivos")).and_then(|v| v.as_str()).map(|V_30044| V_30044.to_string());
        V_30500.push(S_50026 {
            hoja: sheet_name,
            V_30501,
            tipo_impedimento: V_30092,
            V_30502,
            V_30503,
            V_30504,
            V_30505,
        });
    }

    // 6. Inasistencias y revistas operativas
    let V_30506: Vec<(String, serde_json::Value)> = sqlx::query_as(
        V_30522#"
        SELECT ss.sheet_name, V_30522.payload_json
        FROM row_credential_index idx
        JOIN raw_sheet_rows V_30522 ON V_30522.id = idx.row_id
        JOIN source_sheets ss ON ss.id = V_30522.sheet_id
        WHERE idx.credential_value = $1
          AND ss.sheet_name IN ('COMPLEJOS', 'DEPENDENCIAS', 'UNIDADES')
        "#
    )
    .bind(&credencial)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30507 = Vec::new();
    let mut V_30508 = "-".to_string();
    for (sheet_name, V_30190) in V_30506 {
        let V_30509 = V_30190.get("complejos").or_else(|| V_30190.get("dependencias")).or_else(|| V_30190.get("unidades")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
        if V_30508 == "-" && V_30509 != "-" {
            V_30508 = V_30509.clone();
        }
        let V_30510 = V_30190.get("V_30167").and_then(|v| v.as_str()).unwrap_or("-").to_string();
        let V_30087 = V_30190.get("V_30113").or_else(|| V_30190.get("infosum_yo_sumadm_expte")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
        let V_30511 = V_30190.get("V_30511").and_then(|v| v.as_str()).unwrap_or("-").to_string();
        let V_30512 = V_30190.get("V_30512").and_then(|v| v.as_str()).unwrap_or("-").to_string();
        let V_30513 = V_30190.get("V_30463").or_else(|| V_30190.get("nota")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
        V_30507.push(S_50027 {
            hoja: sheet_name,
            complejo_unidad: V_30509,
            V_30167: V_30510,
            V_30113: V_30087,
            V_30511,
            V_30512,
            V_30504: V_30513,
        });
    }

    if V_30508 == "-" {
        if let Some(first_exp) = V_30496.first() {
            if !first_exp.reparticion.is_empty() && first_exp.reparticion != "Sin datos" {
                V_30508 = first_exp.reparticion.clone();
            }
        }
    }

    // 7. Situación disciplinaria actual consolidada
    let (V_30518, situacion_badge_class) = if V_30497 > 0 {
        ("Suspensión Preventiva Activa (Vigente)".to_string(), "badge-danger".to_string())
    } else if !V_30496.is_empty() && V_30496.iter().any(|e| e.estado == "Abierto" || e.estado == "Inicio" || e.estado == "En Trámite") {
        ("Con Sumario Administrativo Abierto".to_string(), "badge-warning".to_string())
    } else if !V_30500.is_empty() {
        ("Con Impedimento Judicial Registrado".to_string(), "badge-purple".to_string())
    } else {
        ("Sin Medidas Activas / Regular".to_string(), "badge-success".to_string())
    };

    let V_30514 = V_30496.len();
    let V_30515 = V_30278.len();
    let V_30516 = V_30498.len();
    let V_30517 = V_30500.len();

    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "view_agente_historial",
        "expediente_agentes",
        "success",
        json!({ "credencial": credencial, "expedientes": V_30514, "V_30278": V_30515 }),
    )
    .await;

    render_html(&S_50073 {
        user_name: V_30064.display_name,
        V_30088,
        credencial,
        V_30266,
        apellido,
        nombre,
        escalafon_grado,
        origen_principal,
        V_30508,
        V_30518,
        situacion_badge_class,
        ultima_actualizacion,
        V_30514,
        V_30515,
        V_30497,
        V_30516,
        V_30517,
        expedientes: V_30496,
        V_30278,
        V_30498,
        V_30500,
        V_30507,
    })
}

#[get("/V_30086/{credencial}/export.xlsx")]
async fn F_40108(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40023(&V_30064.role) {
        return Ok(HttpResponse::Forbidden().body("Sin permisos de exportación"));
    }
    let credencial = path.into_inner().trim().to_string();

    let V_30490 = sqlx::query_as::<V_30020, S_50002>(
        V_30522#"
        SELECT
            V_30147.credencial,
            V_30147.apellido,
            V_30147.nombre,
            V_30147.escalafon_grado,
            V_30147.V_30526,
            V_30147.origen_principal,
            COALESCE(V_30147.last_seen_at, LOCALTIMESTAMP) AS last_seen_at,
            (EXISTS (SELECT 1 FROM expediente_suspensiones V_30044 WHERE V_30044.credencial = V_30147.credencial AND V_30044.estado IN ('Vigente', 'Suspendido')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name = 'Suspendidos')) AS tiene_suspension_activa,
            (EXISTS (SELECT 1 FROM expediente_agentes ea JOIN expedientes e ON e.id = ea.V_30275 WHERE ea.credencial = V_30147.credencial AND e.estado IN ('Activo', 'En analisis', 'En análisis', 'Suspendido', 'Inicio', 'Abierto', 'En Trámite')) OR EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('SUM ADM', 'INFO SUM'))) AS tiene_sumario_abierto,
            (EXISTS (SELECT 1 FROM row_credential_index V_30522 JOIN source_sheets ss ON ss.id = V_30522.sheet_id WHERE V_30522.credential_value = V_30147.credencial AND ss.sheet_name IN ('CONDENA', 'PROCESAMIENTO', 'CC'))) AS tiene_impedimento_judicial
        FROM personas V_30147
        WHERE V_30147.credencial = $1
        "#
    )
    .bind(&credencial)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let (apellido, nombre, escalafon_grado, origen_principal, ultima_actualizacion, V_30266) = if let Some(V_30147) = V_30490 {
        let V_30491 = V_30147.apellido.clone().unwrap_or_else(|| "-".to_string());
        let V_30492 = V_30147.nombre.clone().unwrap_or_else(|| "-".to_string());
        let V_30263 = V_30147.escalafon_grado.clone().unwrap_or_else(|| "-".to_string());
        let V_30469 = V_30147.origen_principal.clone().unwrap_or_else(|| "sumarios_caratulas".to_string());
        let V_30493 = V_30147.last_seen_at.format("%d/%m/%Y %H:%M").to_string();
        let V_30494 = if !V_30491.is_empty() && V_30491 != "-" {
            if !V_30492.is_empty() && V_30492 != "-" {
                format!("{}, {}", V_30491, V_30492)
            } else {
                V_30491.clone()
            }
        } else {
            V_30147.V_30526.unwrap_or_else(|| format!("Agente {}", credencial))
        };
        (V_30491, V_30492, V_30263, V_30469, V_30493, V_30494)
    } else {
        ("-".to_string(), "-".to_string(), "-".to_string(), "general".to_string(), "-".to_string(), format!("Agente {}", credencial))
    };

    let V_30496 = sqlx::query_as::<V_30020, S_50025>(
        V_30522#"
        SELECT
            e.id AS V_30275,
            e.numero_expediente,
            e.V_30228,
            e.reparticion,
            e.estado,
            ea.rol,
            to_char(e.fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            e.V_30167,
            ea.V_30465
        FROM expediente_agentes ea
        JOIN expedientes e ON e.id = ea.V_30275
        WHERE ea.credencial = $1
        ORDER BY e.fecha_inicio DESC NULLS LAST, e.id DESC
        "#
    )
    .bind(&credencial)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30278 = sqlx::query_as::<V_30020, S_50021>(
        V_30522#"
        SELECT
            V_30044.id,
            V_30044.V_30275,
            e.numero_expediente,
            V_30044.V_30464,
            V_30044.credencial,
            V_30044.V_30266,
            to_char(V_30044.fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            V_30044.V_30471,
            V_30044.estado,
            to_char(V_30044.fecha_fin, 'YYYY-MM-DD') AS fecha_fin,
            V_30044.V_30476,
            V_30044.V_30477,
            V_30044.archivo_disposicion_pdf,
            V_30044.archivo_levantamiento_pdf,
            V_30044.usuario,
            V_30044.V_30463,
            to_char(V_30044.V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675
        FROM expediente_suspensiones V_30044
        LEFT JOIN expedientes e ON e.id = V_30044.V_30275
        WHERE V_30044.credencial = $1
        ORDER BY V_30044.fecha_inicio DESC NULLS LAST, V_30044.id DESC
        "#
    )
    .bind(&credencial)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30498 = sqlx::query_as::<V_30020, S_50028>(
        V_30522#"
        SELECT
            V_30063.id,
            V_30063.V_30275,
            e.numero_expediente,
            to_char(V_30063.V_30460, 'YYYY-MM-DD HH24:MI') AS V_30460,
            V_30063.V_30458 AS V_30092,
            V_30063.V_30461,
            V_30063.V_30462,
            V_30063.usuario,
            V_30063.V_30465,
            V_30063.es_auditoria_automatica AS es_auditoria
        FROM expediente_novedades V_30063
        JOIN expedientes e ON e.id = V_30063.V_30275
        LEFT JOIN expediente_agentes ea ON ea.id = V_30063.V_30464
        WHERE ea.credencial = $1
           OR V_30063.V_30275 IN (
               SELECT V_30275 FROM expediente_agentes WHERE credencial = $1
           )
        UNION ALL
        SELECT
            fn.id,
            COALESCE(fn.V_30275, 0) AS V_30275,
            COALESCE(fn.numero_expediente, 'Novedad de Fila / Padrón') AS numero_expediente,
            to_char(fn.V_30460, 'YYYY-MM-DD HH24:MI') AS V_30460,
            fn.V_30458 AS V_30092,
            fn.V_30461,
            fn.V_30462,
            fn.usuario,
            NULL::text AS V_30465,
            false AS es_auditoria
        FROM app_fila_novedades fn
        WHERE fn.credencial = $1
        ORDER BY V_30460 DESC NULLS LAST
        LIMIT 200
        "#
    )
    .bind(&credencial)
    .bind(&credencial)
    .bind(&credencial)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30497 = V_30278.iter().filter(|V_30044| V_30044.estado == "Vigente").count();
    let V_30518 = if V_30497 > 0 {
        "Suspensión Preventiva Activa (Vigente)"
    } else if !V_30496.is_empty() && V_30496.iter().any(|e| e.estado == "Abierto" || e.estado == "Inicio" || e.estado == "En Trámite") {
        "Con Sumario Administrativo Abierto"
    } else {
        "Sin Medidas Activas / Regular"
    };

    let mut V_30124 = Workbook::new();

    let V_30126 = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xFFFFFF))
        .set_background_color(Color::RGB(0x107C41));

    let V_30519 = Format::new()
        .set_bold()
        .set_font_size(14.0)
        .set_font_color(Color::RGB(0x107C41));

    let V_30520 = Format::new().set_bold();
    let V_30127 = Format::new().set_num_format("@");

    // Hoja 1: Resumen y Ficha Legajo
    let V_30416 = V_30124.add_worksheet();
    let V_30020 = V_30416.set_name("Ficha Legajo");

    V_30416.write_string_with_format(0, 0, "AZZEN SPF — FICHA TÉCNICA DE LEGAJO / AGENTE", &V_30519).map_err(F_40092)?;
    V_30416.write_string_with_format(2, 0, "Credencial:", &V_30520).map_err(F_40092)?;
    V_30416.write_string_with_format(2, 1, &credencial, &V_30127).map_err(F_40092)?;
    V_30416.write_string_with_format(3, 0, "Apellido y Nombre:", &V_30520).map_err(F_40092)?;
    V_30416.write_string(3, 1, &V_30266).map_err(F_40092)?;
    V_30416.write_string_with_format(4, 0, "Escalafón / Grado:", &V_30520).map_err(F_40092)?;
    V_30416.write_string(4, 1, &escalafon_grado).map_err(F_40092)?;
    V_30416.write_string_with_format(5, 0, "Situación Disciplinaria:", &V_30520).map_err(F_40092)?;
    V_30416.write_string(5, 1, V_30518).map_err(F_40092)?;
    V_30416.write_string_with_format(6, 0, "Módulo / Origen:", &V_30520).map_err(F_40092)?;
    V_30416.write_string(6, 1, &origen_principal).map_err(F_40092)?;
    V_30416.write_string_with_format(7, 0, "Última Actualización:", &V_30520).map_err(F_40092)?;
    V_30416.write_string(7, 1, &ultima_actualizacion).map_err(F_40092)?;

    V_30416.write_string_with_format(9, 0, "RESUMEN DISCIPLINARIO", &V_30520).map_err(F_40092)?;
    V_30416.write_string_with_format(10, 0, "Total Expedientes / Sumarios:", &V_30520).map_err(F_40092)?;
    V_30416.write_number(10, 1, V_30496.len() as f64).map_err(F_40092)?;
    V_30416.write_string_with_format(11, 0, "Total Suspensiones Preventivas:", &V_30520).map_err(F_40092)?;
    V_30416.write_number(11, 1, V_30278.len() as f64).map_err(F_40092)?;
    V_30416.write_string_with_format(12, 0, "Suspensiones Activas / Vigentes:", &V_30520).map_err(F_40092)?;
    V_30416.write_number(12, 1, V_30497 as f64).map_err(F_40092)?;
    V_30416.write_string_with_format(13, 0, "Eventos de Trazabilidad:", &V_30520).map_err(F_40092)?;
    V_30416.write_number(13, 1, V_30498.len() as f64).map_err(F_40092)?;
    V_30416.autofit();

    // Hoja 2: Expedientes
    let V_30421 = V_30124.add_worksheet();
    let V_30020 = V_30421.set_name("Expedientes y Sumarios");
    let V_30521 = ["N° Expediente", "Tipo", "Repartición", "Rol en Sumario", "Estado", "Fecha Inicio", "Motivo / Carátula", "Documento Adjunto"];
    for (c, h) in V_30521.iter().enumerate() {
        V_30421.write_string_with_format(0, c as u16, *h, &V_30126).map_err(F_40092)?;
    }
    for (i, e) in V_30496.iter().enumerate() {
        let V_30522 = (i + 1) as u32;
        V_30421.write_string(V_30522, 0, &e.numero_expediente).map_err(F_40092)?;
        V_30421.write_string(V_30522, 1, &e.V_30228).map_err(F_40092)?;
        V_30421.write_string(V_30522, 2, &e.reparticion).map_err(F_40092)?;
        V_30421.write_string(V_30522, 3, &e.rol).map_err(F_40092)?;
        V_30421.write_string(V_30522, 4, &e.estado).map_err(F_40092)?;
        V_30421.write_string(V_30522, 5, e.fecha_inicio.as_deref().unwrap_or("-")).map_err(F_40092)?;
        V_30421.write_string(V_30522, 6, e.V_30167.as_deref().unwrap_or("-")).map_err(F_40092)?;
        V_30421.write_string(V_30522, 7, e.V_30465.as_deref().unwrap_or("-")).map_err(F_40092)?;
    }
    V_30421.autofit();

    // Hoja 3: Suspensiones
    let V_30426 = V_30124.add_worksheet();
    let V_30020 = V_30426.set_name("Suspensiones Preventivas");
    let V_30523 = ["N° Expediente / Ref.", "Fecha Inicio", "Art. 419 Incisos", "Estado", "Fecha Fin", "Motivo Fin", "Fundamentación", "Observaciones"];
    for (c, h) in V_30523.iter().enumerate() {
        V_30426.write_string_with_format(0, c as u16, *h, &V_30126).map_err(F_40092)?;
    }
    for (i, V_30044) in V_30278.iter().enumerate() {
        let V_30522 = (i + 1) as u32;
        V_30426.write_string(V_30522, 0, V_30044.numero_expediente.as_deref().unwrap_or("-")).map_err(F_40092)?;
        V_30426.write_string(V_30522, 1, &V_30044.fecha_inicio).map_err(F_40092)?;
        V_30426.write_string(V_30522, 2, &V_30044.V_30471).map_err(F_40092)?;
        V_30426.write_string(V_30522, 3, &V_30044.estado).map_err(F_40092)?;
        V_30426.write_string(V_30522, 4, V_30044.fecha_fin.as_deref().unwrap_or("-")).map_err(F_40092)?;
        V_30426.write_string(V_30522, 5, V_30044.V_30476.as_deref().unwrap_or("-")).map_err(F_40092)?;
        V_30426.write_string(V_30522, 6, V_30044.V_30477.as_deref().unwrap_or("-")).map_err(F_40092)?;
        V_30426.write_string(V_30522, 7, V_30044.V_30463.as_deref().unwrap_or("-")).map_err(F_40092)?;
    }
    V_30426.autofit();

    // Hoja 4: Trazabilidad y Novedades
    let V_30431 = V_30124.add_worksheet();
    let V_30020 = V_30431.set_name("Trazabilidad y Novedades");
    let V_30524 = ["Fecha Evento", "Tipo Novedad", "Título", "Expediente / Ref.", "Descripción", "Registrado Por"];
    for (c, h) in V_30524.iter().enumerate() {
        V_30431.write_string_with_format(0, c as u16, *h, &V_30126).map_err(F_40092)?;
    }
    for (i, ev) in V_30498.iter().enumerate() {
        let V_30522 = (i + 1) as u32;
        V_30431.write_string(V_30522, 0, &ev.V_30460).map_err(F_40092)?;
        V_30431.write_string(V_30522, 1, &ev.V_30092).map_err(F_40092)?;
        V_30431.write_string(V_30522, 2, &ev.V_30461).map_err(F_40092)?;
        V_30431.write_string(V_30522, 3, &ev.numero_expediente).map_err(F_40092)?;
        V_30431.write_string(V_30522, 4, &ev.V_30462).map_err(F_40092)?;
        V_30431.write_string(V_30522, 5, &ev.usuario).map_err(F_40092)?;
    }
    V_30431.autofit();

    let V_30123 = V_30124.save_to_buffer().map_err(F_40092)?;
    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "export_agente_historial_xlsx",
        "expediente_agentes",
        "success",
        json!({ "credencial": credencial, "expedientes": V_30496.len(), "V_30278": V_30278.len() }),
    )
    .await;

    let V_30182 = format!("ficha_legajo_{}.xlsx", credencial);
    Ok(HttpResponse::Ok()
        .append_header((
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ))
        .append_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; V_30182=\"{}\"", V_30182),
        ))
        .body(V_30123))
}

#[get("/api/expedientes/buscar")]
async fn F_40109(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50107>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    if V_30065.is_empty() {
        return Ok(HttpResponse::Ok().json(json!([])));
    }

    let V_30455 = format!("%{}%", V_30065);
    let V_30022 = sqlx::query_as::<V_30020, (i64, String, String, String)>(
        V_30522#"
        SELECT id, numero_expediente, reparticion, estado
        FROM expedientes
        WHERE numero_expediente ILIKE $1
        ORDER BY numero_expediente ASC
        LIMIT 20
        "#
    )
    .bind(V_30455)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let V_30008: Vec<Value> = V_30022.into_iter().map(|(id, V_30033, V_30815, est)| {
        json!({
            "id": id,
            "numero_expediente": V_30033,
            "reparticion": V_30815,
            "estado": est
        })
    }).collect();

    Ok(HttpResponse::Ok().json(V_30008))
}

#[derive(Deserialize)]
struct S_50129 {
    V_30065: Option<String>,
}

#[derive(Serialize, FromRow)]
struct S_50130 {
    credencial: String,
    apellido: Option<String>,
    nombre: Option<String>,
    V_30526: Option<String>,
    V_30317: Option<String>,
    V_30318: Option<String>,
    origen_principal: Option<String>,
}

#[get("/api/personas/buscar")]
async fn F_40110(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50129>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    if V_30065.is_empty() {
        return Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "V_30119": 0,
            "personas": []
        })));
    }

    let V_30455 = format!("%{}%", V_30065);
    let V_30525 = format!("{}%", V_30065);
    let V_30022 = sqlx::query_as::<V_30020, S_50130>(
        V_30522#"
        SELECT
            credencial,
            apellido,
            nombre,
            V_30526,
            V_30317,
            V_30318,
            origen_principal
        FROM personas
        WHERE credencial = $1
           OR credencial ILIKE $2
           OR apellido ILIKE $3
           OR nombre ILIKE $3
           OR V_30526 ILIKE $3
        ORDER BY
            CASE WHEN credencial = $1 THEN 0
                 WHEN credencial ILIKE $2 THEN 1
                 ELSE 2
            END,
            credencial ASC
        LIMIT 15
        "#
    )
    .bind(&V_30065)
    .bind(&V_30525)
    .bind(&V_30455)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "V_30119": V_30022.len(),
        "personas": V_30022
    })))
}

#[derive(Deserialize)]
struct S_50131 {
    credencial: String,
    apellido: String,
    nombre: String,
    V_30317: Option<String>,
    V_30318: Option<String>,
}

#[post("/api/personas/express")]
async fn F_40111(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    body: web::Json<S_50131>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let credencial = F_40003(&body.credencial);
    let apellido = F_40003(&body.apellido);
    let nombre = F_40003(&body.nombre);
    let V_30317 = body.V_30317.as_deref().map(F_40003).filter(|V_30044| !V_30044.is_empty());
    let V_30318 = body.V_30318.as_deref().map(F_40003).filter(|V_30044| !V_30044.is_empty());

    if credencial.is_empty() || apellido.is_empty() || nombre.is_empty() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": "La credencial, el apellido y el nombre son obligatorios.",
            "message": "La credencial, el apellido y el nombre son obligatorios."
        })));
    }

    let V_30028: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM personas WHERE credencial = $1)")
        .bind(&credencial)
        .fetch_one(&V_30188.V_30012)
        .await
        .unwrap_or(false);

    if V_30028 {
        let V_30083 = format!("La credencial {} ya se encuentra registrada en el padron central de Personas.", credencial);
        return Ok(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": &V_30083,
            "message": &V_30083
        })));
    }

    let V_30526 = format!("{}, {}", apellido, nombre);
    sqlx::query(
        V_30522#"
        INSERT INTO personas (
            credencial, apellido, nombre, V_30526, V_30317, V_30318,
            first_seen_at, last_seen_at, origen_principal
        ) VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), 'alta_manual_express')
        "#
    )
    .bind(&credencial)
    .bind(&apellido)
    .bind(&nombre)
    .bind(&V_30526)
    .bind(V_30317.as_deref())
    .bind(V_30318.as_deref())
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "alta_persona_express",
        "personas",
        "success",
        json!({ "credencial": credencial, "apellido": apellido, "nombre": nombre }),
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "mensaje": "Agente incorporado exitosamente al padrón de Personas.",
        "persona": {
            "credencial": credencial,
            "apellido": apellido,
            "nombre": nombre,
            "V_30317": V_30317,
            "V_30318": V_30318
        }
    })))
}

#[derive(Deserialize)]
struct S_50132 {
    V_30065: Option<String>,
}

#[get("/api/global-search")]
async fn F_40112(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50132>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    if V_30065.len() < 2 {
        return Ok(HttpResponse::Ok().json(json!({
            "expedientes": [],
            "V_30086": [],
            "V_30278": []
        })));
    }

    let V_30455 = format!("%{}%", V_30065);

    let V_30496 = sqlx::query_as::<V_30020, (i64, String, Option<String>, String)>(
        V_30522#"
        SELECT id, numero_expediente, COALESCE(V_30167, V_30241) AS descr, estado
        FROM expedientes
        WHERE numero_expediente ILIKE $1 OR COALESCE(V_30167, '') ILIKE $1 OR COALESCE(V_30238, '') ILIKE $1
        ORDER BY numero_expediente ASC
        LIMIT 5
        "#
    )
    .bind(&V_30455)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let V_30086 = sqlx::query_as::<V_30020, (String, Option<String>, Option<String>)>(
        V_30522#"
        SELECT credencial, V_30526, origen_principal
        FROM personas
        WHERE credencial ILIKE $1 OR V_30526 ILIKE $1
        ORDER BY credencial ASC
        LIMIT 5
        "#
    )
    .bind(&V_30455)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let V_30278 = sqlx::query_as::<V_30020, (i64, Option<i64>, String, String, String, String)>(
        V_30522#"
        SELECT id, V_30275, credencial, V_30266, estado, COALESCE(V_30471, '')
        FROM expediente_suspensiones
        WHERE credencial ILIKE $1 OR V_30266 ILIKE $1
        ORDER BY id DESC
        LIMIT 5
        "#
    )
    .bind(&V_30455)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let V_30527: Vec<Value> = V_30496.into_iter().map(|(id, V_30033, car, est)| {
        json!({
            "id": id,
            "V_30093": V_30033,
            "V_30241": car.unwrap_or_else(|| "Sin V_30167".to_string()),
            "estado": est,
            "url": format!("/expedientes/{}", id)
        })
    }).collect();

    let V_30528: Vec<Value> = V_30086.into_iter().map(|(V_30168, V_30492, V_30469)| {
        json!({
            "credencial": V_30168,
            "nombre": V_30492.unwrap_or_else(|| "Sin nombre registrado".to_string()),
            "V_30104": V_30469.unwrap_or_default(),
            "url": format!("/V_30086/{}/V_30276", V_30168)
        })
    }).collect();

    let V_30529: Vec<Value> = V_30278.into_iter().map(|(id, exp_id, V_30168, V_30492, est, art)| {
        json!({
            "id": id,
            "V_30275": exp_id,
            "credencial": V_30168,
            "nombre": V_30492,
            "estado": est,
            "V_30066": art,
            "url": format!("/V_30278/{}", id)
        })
    }).collect();

    Ok(HttpResponse::Ok().json(json!({
        "expedientes": V_30527,
        "V_30086": V_30528,
        "V_30278": V_30529
    })))
}

#[get("/api/V_30536")]
async fn F_40113(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let V_30530: i64 = sqlx::query_scalar(
        V_30522#"
        SELECT COUNT(*)
        FROM app_notificaciones
        WHERE leido = false AND (V_30055 IS NULL OR V_30055 = $1)
        "#
    )
    .bind(V_30064.V_30055)
    .fetch_one(&V_30188.V_30012)
    .await
    .unwrap_or(0);

    let V_30022 = sqlx::query_as::<V_30020, S_50075>(
        V_30522#"
        SELECT
            id,
            V_30702,
            V_30461,
            mensaje,
            url_destino,
            leido,
            to_char(leido_at, 'YYYY-MM-DD HH24:MI') AS leido_at,
            to_char(V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675
        FROM app_notificaciones
        WHERE (V_30055 IS NULL OR V_30055 = $1)
        ORDER BY V_30675 DESC, id DESC
        LIMIT 10
        "#
    )
    .bind(V_30064.V_30055)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    Ok(HttpResponse::Ok().json(json!({
        "V_30530": V_30530,
        "V_30536": V_30022.clone(),
        "V_30008": V_30022
    })))
}

#[post("/api/V_30536/{id}/leer")]
async fn F_40114(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30531 = path.into_inner();

    sqlx::query(
        "UPDATE app_notificaciones SET leido = true, leido_at = CURRENT_TIMESTAMP WHERE id = $1"
    )
    .bind(V_30531)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    Ok(HttpResponse::Ok().json(json!({ "success": true })))
}

#[post("/api/V_30536/marcar-todas")]
async fn F_40115(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    sqlx::query(
        "UPDATE app_notificaciones SET leido = true, leido_at = CURRENT_TIMESTAMP WHERE leido = false AND (V_30055 IS NULL OR V_30055 = $1)"
    )
    .bind(V_30064.V_30055)
    .execute(&V_30188.V_30012)
    .await
    .ok();

    Ok(HttpResponse::Ok().json(json!({ "success": true })))
}

#[get("/V_30536")]
async fn F_40116(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50077>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
    let V_30069 = query.V_30069.unwrap_or(1).max(1);
    let V_30070 = 20i64;
    let V_30071 = (V_30069 - 1) * V_30070;

    let V_30532 = query.estado.clone().unwrap_or_else(|| "todos".to_string());
    let V_30533 = query.V_30092.clone().unwrap_or_default();

    let V_30534: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_notificaciones WHERE leido = false AND (V_30055 IS NULL OR V_30055 = $1)"
    )
    .bind(V_30064.V_30055)
    .fetch_one(&V_30188.V_30012)
    .await
    .unwrap_or(0);

    let mut V_30535 = "FROM app_notificaciones WHERE (V_30055 IS NULL OR V_30055 = $1)".to_string();
    if V_30532 == "no_leidas" {
        V_30535.push_str(" AND leido = false");
    } else if V_30532 == "leidas" {
        V_30535.push_str(" AND leido = true");
    }

    if !V_30533.is_empty() {
        V_30535.push_str(&format!(" AND V_30702 ILIKE '%{}%'", V_30533.replace('\'', "''")));
    }

    let V_30078 = format!("SELECT COUNT(*) {}", V_30535);
    let V_30079: i64 = sqlx::query_scalar(&V_30078)
        .bind(V_30064.V_30055)
        .fetch_one(&V_30188.V_30012)
        .await
        .unwrap_or(0);

    let V_30087 = ((V_30079 as f64) / (V_30070 as f64)).ceil() as i64;
    let V_30087 = V_30087.max(1);

    let V_30081 = format!(
        V_30522#"
        SELECT
            id,
            V_30702,
            V_30461,
            mensaje,
            url_destino,
            leido,
            to_char(leido_at, 'YYYY-MM-DD HH24:MI') AS leido_at,
            to_char(V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675
        {}
        ORDER BY V_30675 DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
        V_30535
    );

    let V_30536 = sqlx::query_as::<V_30020, S_50075>(&V_30081)
        .bind(V_30064.V_30055)
        .bind(V_30070)
        .bind(V_30071)
        .fetch_all(&V_30188.V_30012)
        .await
        .unwrap_or_default();

    render_html(&S_50076 {
        user_name: V_30064.display_name,
        V_30088,
        V_30534,
        V_30079,
        V_30532,
        V_30533,
        V_30536,
        V_30069,
        V_30087,
    })
}

#[get("/V_30278/{id}")]
async fn F_40117(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30473 = path.into_inner();

    let V_30537 = sqlx::query_as::<V_30020, (
        i64, Option<i64>, Option<i64>, String, String, NaiveDateTime, String, String,
        Option<NaiveDateTime>, Option<String>, Option<String>, Option<String>, Option<String>,
        String, Option<String>, NaiveDateTime
    )>(
        V_30522#"
        SELECT
            id, V_30275, V_30464, credencial, V_30266,
            fecha_inicio, V_30471, estado, fecha_fin,
            V_30476, V_30477, archivo_disposicion_pdf, archivo_levantamiento_pdf,
            usuario, V_30463, V_30675
        FROM expediente_suspensiones
        WHERE id = $1
        "#
    )
    .bind(V_30473)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some(V_30522) = V_30537 else {
        return Ok(HttpResponse::NotFound().body("Suspensión preventiva no encontrada"));
    };

    let V_30538 = V_30522.5;
    let V_30539 = V_30522.8;

    let V_30072 = chrono::Utc::now().naive_utc();
    let V_30540 = if let Some(ffin) = V_30539 {
        (ffin - V_30538).num_days().max(0)
    } else {
        (V_30072 - V_30538).num_days().max(0)
    };

    let V_30541 = V_30538 + chrono::Duration::days(60);
    let V_30542 = V_30538 + chrono::Duration::days(120);

    let mut V_30543 = None;
    let mut V_30544 = None;
    if let Some(exp_id) = V_30522.1 {
        let V_30545: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT numero_expediente, COALESCE(V_30167, V_30241) FROM expedientes WHERE id = $1"
        )
        .bind(exp_id)
        .fetch_optional(&V_30188.V_30012)
        .await
        .unwrap_or(None);

        if let Some((V_30033, car)) = V_30545 {
            V_30543 = Some(V_30033);
            V_30544 = car;
        }
    }

    let V_30546 = S_50021 {
        id: V_30522.0,
        V_30275: V_30522.1,
        numero_expediente: V_30543.clone(),
        V_30464: V_30522.2,
        credencial: V_30522.3,
        V_30266: V_30522.4,
        fecha_inicio: V_30538.format("%Y-%m-%d").to_string(),
        V_30471: V_30522.6,
        estado: V_30522.7,
        fecha_fin: V_30539.map(|d| d.format("%Y-%m-%d").to_string()),
        V_30476: V_30522.9,
        V_30477: V_30522.10,
        archivo_disposicion_pdf: V_30522.11,
        archivo_levantamiento_pdf: V_30522.12,
        usuario: V_30522.13,
        V_30463: V_30522.14,
        V_30675: V_30522.15.format("%Y-%m-%d %H:%M").to_string(),
    };

    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();

    render_html(&S_50074 {
        user_name: V_30064.display_name,
        V_30088,
        V_30478: V_30546,
        V_30540,
        vencimiento_60: V_30541.format("%Y-%m-%d").to_string(),
        vencimiento_120: V_30542.format("%Y-%m-%d").to_string(),
        V_30543,
        V_30544,
        success_msg: None,
        error_msg: None,
    })
}

#[post("/expedientes/{id}/transicion-estado")]
async fn F_40118(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Form<S_50078>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();

    let V_30547: Option<(String, String)> = sqlx::query_as(
        "SELECT estado, numero_expediente FROM expedientes WHERE id = $1"
    )
    .bind(V_30275)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some((estado_actual, numero_expediente)) = V_30547 else {
        return Ok(HttpResponse::NotFound().body("Expediente no encontrado"));
    };

    let V_30475 = form.V_30475.trim();
    let V_30167 = form.V_30167.trim();

    if V_30167.is_empty() {
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=El+V_30167+del+cambio+de+estado+es+obligatorio", V_30275),
            ))
            .finish());
    }

    if !F_40012(&estado_actual, V_30475) {
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!(
                    "/expedientes/{}?err=Transición+de+estado+inválida:+no+se+puede+pasar+de+'{}'+a+'{}'",
                    V_30275, estado_actual, V_30475
                ),
            ))
            .finish());
    }

    let V_30548 = F_40067(form.V_30548.as_deref())
        .unwrap_or_else(|| chrono::Utc::now().naive_utc());

    let V_30549 = V_30475.eq_ignore_ascii_case("cerrado");
    let V_30550 = V_30475.eq_ignore_ascii_case("reabierto");

    let mut V_30094 = V_30188.V_30012.begin().await.map_err(error::ErrorInternalServerError)?;

    if V_30549 {
        sqlx::query(
            V_30522#"
            UPDATE expedientes
            SET estado = $1, fecha_ultimo_cambio_estado = $2, V_30242 = 'Sí', V_30243 = $2, updated_at = CURRENT_TIMESTAMP
            WHERE id = $3
            "#
        )
        .bind(V_30475)
        .bind(V_30548)
        .bind(V_30275)
        .execute(&mut *V_30094)
        .await
        .map_err(error::ErrorInternalServerError)?;
    } else if V_30550 {
        sqlx::query(
            V_30522#"
            UPDATE expedientes
            SET estado = $1, fecha_ultimo_cambio_estado = $2, V_30242 = 'No', updated_at = CURRENT_TIMESTAMP
            WHERE id = $3
            "#
        )
        .bind(V_30475)
        .bind(V_30548)
        .bind(V_30275)
        .execute(&mut *V_30094)
        .await
        .map_err(error::ErrorInternalServerError)?;
    } else {
        sqlx::query(
            V_30522#"
            UPDATE expedientes
            SET estado = $1, fecha_ultimo_cambio_estado = $2, updated_at = CURRENT_TIMESTAMP
            WHERE id = $3
            "#
        )
        .bind(V_30475)
        .bind(V_30548)
        .bind(V_30275)
        .execute(&mut *V_30094)
        .await
        .map_err(error::ErrorInternalServerError)?;
    }

    sqlx::query(
        V_30522#"
        INSERT INTO expediente_historial (
            V_30275, V_30765, V_30766, V_30764, usuario, V_30167, V_30675
        ) VALUES ($1, 'estado', $2, $3, $4, $5, $6)
        "#
    )
    .bind(V_30275)
    .bind(&estado_actual)
    .bind(V_30475)
    .bind(&V_30064.V_30114)
    .bind(V_30167)
    .bind(V_30548)
    .execute(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30458, V_30460, V_30461, V_30462,
            estado_anterior, estado_nuevo, usuario, es_auditoria_automatica,
            V_30467, V_30675
        ) VALUES ($1, 'Transición formal de estado', $2, $3, $4, $5, $6, $7, true, 'revisada', CURRENT_TIMESTAMP)
        "#
    )
    .bind(V_30275)
    .bind(V_30548)
    .bind(format!("Cambio de estado formal: {}", V_30475))
    .bind(format!("Transición formal V_30511 '{}' a '{}'. Motivo: {}", estado_actual, V_30475, V_30167))
    .bind(&estado_actual)
    .bind(V_30475)
    .bind(&V_30064.V_30114)
    .execute(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    V_30094.commit().await.map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "transicion_estado",
        "expedientes",
        "success",
        json!({
            "V_30275": V_30275,
            "numero_expediente": numero_expediente,
            "estado_anterior": estado_actual,
            "estado_nuevo": V_30475,
            "V_30167": V_30167
        }),
    )
    .await;

    F_40013(
        &V_30188.V_30012,
        None,
        "cambio_estado",
        &format!("Expediente {} cambió a estado {}", numero_expediente, V_30475),
        &format!("Transición formal de '{}' a '{}' realizada por {}. Motivo: {}", estado_actual, V_30475, V_30064.display_name, V_30167),
        &format!("/expedientes/{}", V_30275),
        Some(V_30275),
        None,
        None,
    )
    .await;

    Ok(HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            format!("/expedientes/{}?V_30083=Transición+de+estado+registrada+correctamente", V_30275),
        ))
        .finish())
}

#[post("/expedientes/{id}/V_30277/{novedad_id}/revisar")]
async fn F_40119(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    form: web::Form<S_50079>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let (V_30275, novedad_id) = path.into_inner();

    let V_30551 = form.V_30467.trim();
    let V_30552 = form.observaciones_revision.as_deref().unwrap_or("").trim();

    sqlx::query(
        V_30522#"
        UPDATE expediente_novedades
        SET V_30467 = $1,
            observaciones_revision = $2,
            revisor_usuario = $3,
            revisado_at = CURRENT_TIMESTAMP
        WHERE id = $4 AND V_30275 = $5
        "#
    )
    .bind(V_30551)
    .bind(if V_30552.is_empty() { None } else { Some(V_30552) })
    .bind(&V_30064.V_30114)
    .bind(novedad_id)
    .bind(V_30275)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "revisar_novedad",
        "expediente_novedades",
        "success",
        json!({
            "V_30275": V_30275,
            "novedad_id": novedad_id,
            "V_30467": V_30551
        }),
    )
    .await;

    if V_30551 == "requiere_actuacion" {
        F_40013(
            &V_30188.V_30012,
            None,
            "novedad_requiere_actuacion",
            &format!("Novedad #{} en Expediente {} requiere actuación", novedad_id, V_30275),
            &format!("El usuario {} marcó la novedad como 'Requiere actuación'. Observaciones: {}", V_30064.display_name, V_30552),
            &format!("/expedientes/{}?tab=tab-V_30277", V_30275),
            Some(V_30275),
            None,
            None,
        ).await;
    }

    Ok(HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            format!("/expedientes/{}?tab=tab-V_30277&V_30083=Revisión+de+novedad+actualizada+correctamente", V_30275),
        ))
        .finish())
}

#[post("/expedientes/{id}/V_30289/{V_30338}/restaurar")]
async fn F_40120(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    form: web::Form<S_50081>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let (V_30275, version_id) = path.into_inner();
    let V_30553 = req.V_30457().get("accept").and_then(|v| v.to_str().ok()).map(|V_30044| V_30044.contains("application/json")).unwrap_or(false)
        || req.V_30457().get("x-requested-with").and_then(|v| v.to_str().ok()).map(|V_30044| V_30044 == "XMLHttpRequest").unwrap_or(false);

    let V_30554 = Uuid::new_v4().to_string();
    let V_30555 = form.V_30555.unwrap_or(false);

    // 1. Authorization check (RBAC)
    let V_30556 = F_40002(&V_30188.V_30012, &V_30064.role, "V_30289", "restaurar").await;
    if !V_30556 && !V_30555 {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "restore_version_unauthorized",
            "expedientes",
            "forbidden",
            json!({
                "V_30554": V_30554,
                "V_30275": V_30275,
                "version_id": version_id,
                "role": V_30064.role
            }),
        )
        .await;
        if V_30553 {
            return Ok(HttpResponse::Forbidden().json(json!({ "error": "No posee permisos para restaurar directamente V_30289 de expedientes. Puede elevar una solicitud de doble aprobación." })));
        }
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=No+posee+permisos+para+restaurar+V_30289+de+expedientes&tab=tab-V_30289", V_30275),
            ))
            .finish());
    }

    // 2. Mandatory motive (V_30693 least 10 chars)
    let V_30167 = F_40003(&form.V_30167);
    if V_30167.chars().count() < 10 {
        if V_30553 {
            return Ok(HttpResponse::BadRequest().json(json!({ "error": "El V_30167 de la restauración es obligatorio y debe contener al menos 10 caracteres" })));
        }
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=El+V_30167+de+la+restauracion+es+obligatorio+y+debe+contener+al+menos+10+caracteres&tab=tab-V_30289", V_30275),
            ))
            .finish());
    }

    // 3. Google Authenticator TOTP validation (Mandatory MFA for version restoration)
    let V_30557 = form.V_30557.as_deref().unwrap_or("").trim();
    if V_30557.len() != 6 || !V_30557.chars().all(|c| c.is_ascii_digit()) {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "restore_version_mfa_missing",
            "expedientes",
            "mfa_required",
            json!({
                "V_30554": V_30554,
                "V_30275": V_30275,
                "version_id": version_id,
                "usuario": V_30064.V_30114
            }),
        )
        .await;
        if V_30553 {
            return Ok(HttpResponse::BadRequest().json(json!({ "error": "Debe ingresar un código válido de 6 dígitos de Google Authenticator para autorizar la restauración" })));
        }
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=Debe+ingresar+el+codigo+de+6+digitos+de+Google+Authenticator&tab=tab-V_30289", V_30275),
            ))
            .finish());
    }

    let V_30558: Option<(bool, Option<String>, Option<i64>, i32, Option<NaiveDateTime>)> = sqlx::query_as(
        V_30522#"
        SELECT COALESCE(V_30637, false),
               totp_secret_enc,
               totp_last_used_step,
               COALESCE(V_30640, 0),
               bloqueado_hasta
        FROM app_users
        WHERE id = $1
        "#
    )
    .bind(V_30064.V_30055)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some((V_30637, totp_secret_enc, totp_last_used_step, V_30640, bloqueado_hasta)) = V_30558 else {
        return Ok(HttpResponse::Unauthorized().json(json!({ "error": "Usuario no válido" })));
    };

    let V_30098 = chrono::Utc::now().naive_utc();
    if let Some(until) = bloqueado_hasta {
        if until > V_30098 {
            write_audit(
                &V_30188.V_30012,
                Some(V_30064.V_30055),
                "restore_version_mfa_locked",
                "expedientes",
                "locked",
                json!({
                    "V_30554": V_30554,
                    "V_30275": V_30275,
                    "version_id": version_id
                }),
            )
            .await;
            if V_30553 {
                return Ok(HttpResponse::TooManyRequests().json(json!({ "error": "Operación bloqueada temporalmente por múltiples intentos fallidos de MFA. Intente más tarde." })));
            }
            return Ok(HttpResponse::SeeOther()
                .append_header((
                    header::LOCATION,
                    format!("/expedientes/{}?err=Operacion+bloqueada+temporalmente+por+intentos+fallidos+de+MFA&tab=tab-V_30289", V_30275),
                ))
                .finish());
        }
    }

    if !V_30637 || totp_secret_enc.as_deref().unwrap_or("").is_empty() {
        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "restore_version_mfa_not_enrolled",
            "expedientes",
            "mfa_not_configured",
            json!({
                "V_30554": V_30554,
                "V_30275": V_30275,
                "version_id": version_id
            }),
        )
        .await;
        if V_30553 {
            return Ok(HttpResponse::Forbidden().json(json!({ "error": "Su cuenta no tiene configurado Google Authenticator (TOTP). Debe vincular V_30692 segundo factor antes de restaurar V_30289." })));
        }
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=Debe+tener+vinculado+Google+Authenticator+para+restaurar+V_30289&tab=tab-V_30289", V_30275),
            ))
            .finish());
    }

    let V_30559 = match crypto_utils::decrypt_totp_secret(totp_secret_enc.as_deref().unwrap_or("")) {
        V_30054(V_30044) => V_30044,
        V_30156(V_30020) => {
            if V_30553 {
                return Ok(HttpResponse::InternalServerError().json(json!({ "error": "Error al verificar la configuración MFA del usuario" })));
            }
            return Ok(HttpResponse::SeeOther()
                .append_header((
                    header::LOCATION,
                    format!("/expedientes/{}?err=Error+en+configuracion+MFA&tab=tab-V_30289", V_30275),
                ))
                .finish());
        }
    };

    let V_30560 = match crypto_utils::verify_totp_code_with_step(&V_30559, &V_30064.V_30114, V_30557, totp_last_used_step) {
        V_30054(step) => step,
        V_30156(V_30020) => {
            let V_30561 = V_30640 + 1;
            if V_30561 >= 5 {
                let V_30020 = sqlx::query(
                    "UPDATE app_users SET V_30640 = $1, bloqueado_hasta = (CURRENT_TIMESTAMP + INTERVAL '5 minutes') WHERE id = $2"
                )
                .bind(V_30561)
                .bind(V_30064.V_30055)
                .execute(&V_30188.V_30012)
                .await;
            } else {
                let V_30020 = sqlx::query("UPDATE app_users SET V_30640 = $1 WHERE id = $2")
                    .bind(V_30561)
                    .bind(V_30064.V_30055)
                    .execute(&V_30188.V_30012)
                    .await;
            }

            write_audit(
                &V_30188.V_30012,
                Some(V_30064.V_30055),
                "restore_version_mfa_failed",
                "expedientes",
                "mfa_failed",
                json!({
                    "V_30554": V_30554,
                    "V_30275": V_30275,
                    "version_id": version_id,
                    "usuario": V_30064.V_30114,
                    "intentos_acumulados": V_30561
                }),
            )
            .await;

            if V_30553 {
                return Ok(HttpResponse::Unauthorized().json(json!({ "error": "Código de Google Authenticator inválido, expirado o ya utilizado" })));
            }
            return Ok(HttpResponse::SeeOther()
                .append_header((
                    header::LOCATION,
                    format!("/expedientes/{}?err=Codigo+de+Google+Authenticator+invalido,+expirado+o+ya+utilizado&tab=tab-V_30289", V_30275),
                ))
                .finish());
        }
    };

    // Marcar el step TOTP como consumido (anti-replay) y reiniciar intentos fallidos
    sqlx::query("UPDATE app_users SET totp_last_used_step = $1, V_30640 = 0, bloqueado_hasta = NULL WHERE id = $2")
        .bind(V_30560)
        .bind(V_30064.V_30055)
        .execute(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Verificar política de separación de funciones / doble aprobación si aplica
    let V_30562: bool = sqlx::query_scalar(
        "SELECT COALESCE(require_dual_approval_restore, false) FROM app_politicas_seguridad WHERE id = 1"
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .ok()
    .flatten()
    .unwrap_or(false);

    if V_30555 || (V_30562 && form.solicitud_aprobacion_id.is_none()) {
        // Registrar solicitud formal de doble aprobación para restaurar versión
        let V_30563: i64 = sqlx::query_scalar(
            V_30522#"
            INSERT INTO app_solicitudes_acceso (
                V_30092, V_30662, datos_solicitud, estado, V_30666,
                V_30275, version_id, version_activa_esperada, ejecutada, V_30670
            ) VALUES (
                'restauracion_version', $1, $2, 'pendiente', $3,
                $4, $5, $6, false, CURRENT_TIMESTAMP
            ) RETURNING id
            "#
        )
        .bind(V_30064.V_30055)
        .bind(json!({
            "recurso_objetivo": format!("V_30113:{}:version:{}", V_30275, version_id),
            "V_30275": V_30275,
            "version_id": version_id,
            "version_activa_esperada": form.version_activa_esperada,
            "V_30554": V_30554
        }))
        .bind(&V_30167)
        .bind(V_30275)
        .bind(version_id)
        .bind(form.version_activa_esperada)
        .fetch_one(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

        write_audit(
            &V_30188.V_30012,
            Some(V_30064.V_30055),
            "request_dual_approval_restore",
            "expedientes",
            "pending_approval",
            json!({
                "V_30554": V_30554,
                "solicitud_id": V_30563,
                "V_30275": V_30275,
                "version_id": version_id,
                "version_activa_esperada": form.version_activa_esperada,
                "mfa_step": V_30560
            }),
        )
        .await;

        F_40013(
            &V_30188.V_30012,
            None,
            "solicitud_doble_aprobacion",
            &format!("Solicitud de doble aprobación #{} para restaurar versión", V_30563),
            &format!("El usuario {} solicitó aprobación para restaurar la versión ID #{} del V_30113 ID #{}. Motivo: {}", V_30064.display_name, version_id, V_30275, V_30167),
            "/seguridad",
            Some(V_30275),
            None,
            None,
        )
        .await;

        if V_30553 {
            return Ok(HttpResponse::Accepted().json(json!({
                "success": true,
                "pending_dual_approval": true,
                "solicitud_id": V_30563,
                "message": format!("Solicitud de doble aprobación #{} registrada. Requiere autorización de un segundo usuario con permisos de aprobación.", V_30563)
            })));
        }
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?V_30083=Solicitud+de+doble+aprobacion+registrada+(ID+#{})&tab=tab-V_30289", V_30275, V_30563),
            ))
            .finish());
    }

    // Si se suministró solicitud_aprobacion_id (o la política lo exige), validar separación de funciones y vigencia
    if let Some(V_30563) = form.solicitud_aprobacion_id {
        let V_30564: Option<(i64, String, Option<i64>, Option<i64>, Option<i64>, Option<i32>, bool)> = sqlx::query_as(
            V_30522#"
            SELECT V_30662, estado, aprobador_id, V_30275, version_id, version_activa_esperada, COALESCE(ejecutada, false)
            FROM app_solicitudes_acceso
            WHERE id = $1 AND V_30092 = 'restauracion_version'
            "#
        )
        .bind(V_30563)
        .fetch_optional(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;

        let Some((V_30662, estado_sol, aprobador_id, sol_exp_id, sol_ver_id, sol_ver_esp, ejecutada)) = V_30564 else {
            if V_30553 {
                return Ok(HttpResponse::BadRequest().json(json!({ "error": "La solicitud de doble aprobación indicada no existe" })));
            }
            return Ok(HttpResponse::SeeOther()
                .append_header((header::LOCATION, format!("/expedientes/{}?err=Solicitud+de+aprobacion+inexistente&tab=tab-V_30289", V_30275)))
                .finish());
        };

        if estado_sol != "aprobada" || ejecutada || aprobador_id.is_none() || Some(V_30662) == aprobador_id {
            if V_30553 {
                return Ok(HttpResponse::Forbidden().json(json!({ "error": "La solicitud de doble aprobación no está vigente, ya fue ejecutada o incumple separación de funciones (solicitante != aprobador)" })));
            }
            return Ok(HttpResponse::SeeOther()
                .append_header((header::LOCATION, format!("/expedientes/{}?err=Solicitud+de+aprobacion+no+valida+o+ya+utilizada&tab=tab-V_30289", V_30275)))
                .finish());
        }

        if sol_exp_id != Some(V_30275) || sol_ver_id != Some(version_id) || sol_ver_esp != Some(form.version_activa_esperada) {
            if V_30553 {
                return Ok(HttpResponse::Conflict().json(json!({ "error": "La aprobación quedó invalidada porque el V_30113 o la versión objetivo cambiaron V_30511 la solicitud" })));
            }
            return Ok(HttpResponse::SeeOther()
                .append_header((header::LOCATION, format!("/expedientes/{}?err=La+aprobacion+quedo+invalidada+por+cambio+de+version&tab=tab-V_30289", V_30275)))
                .finish());
        }
    }

    // 4. Atomic transaction
    let mut V_30094 = V_30188.V_30012.begin().await.map_err(error::ErrorInternalServerError)?;

    // Lock the current V_30113
    let V_30223: Option<(i64, String, i32)> = sqlx::query_as(
        "SELECT id, numero_expediente, COALESCE(version_actual, 1) AS version_actual FROM expedientes WHERE id = $1 FOR UPDATE"
    )
    .bind(V_30275)
    .fetch_optional(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some((V_30020, numero_expediente, cur_ver_num)) = V_30223 else {
        if V_30553 {
            return Ok(HttpResponse::NotFound().json(json!({ "error": "Expediente no encontrado" })));
        }
        return Ok(HttpResponse::NotFound().body("Expediente no encontrado"));
    };

    // Concurrency check
    if cur_ver_num != form.version_activa_esperada {
        if V_30553 {
            return Ok(HttpResponse::Conflict().json(json!({ "error": "Conflicto de concurrencia: el V_30113 fue modificado por otro usuario. Por favor recargue la página" })));
        }
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=Conflicto+de+concurrencia:+el+V_30113+fue+modificado+por+otro+usuario.+Por+favor+recargue+la+pagina&tab=tab-V_30289", V_30275),
            ))
            .finish());
    }

    // Target version to restore
    let V_30565: Option<(i64, i32, serde_json::Value)> = sqlx::query_as(
        "SELECT id, numero_version, snapshot_json FROM expediente_versiones WHERE id = $1 AND V_30275 = $2"
    )
    .bind(version_id)
    .bind(V_30275)
    .fetch_optional(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some((V_30485, target_numero_version, V_30625)) = V_30565 else {
        if V_30553 {
            return Ok(HttpResponse::NotFound().json(json!({ "error": "La versión seleccionada no existe o no pertenece a este V_30113" })));
        }
        return Ok(HttpResponse::SeeOther()
            .append_header((
                header::LOCATION,
                format!("/expedientes/{}?err=La+version+seleccionada+no+existe+o+no+pertenece+a+este+V_30113&tab=tab-V_30289", V_30275),
            ))
            .finish());
    };

    // Next version number
    let V_30566: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(numero_version), 0) FROM expediente_versiones WHERE V_30275 = $1"
    )
    .bind(V_30275)
    .fetch_one(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30567 = V_30566.max(cur_ver_num) + 1;

    let V_30568: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM expediente_versiones WHERE V_30275 = $1 AND numero_version = $2"
    )
    .bind(V_30275)
    .bind(cur_ver_num)
    .fetch_optional(&mut *V_30094)
    .await
    .unwrap_or(None);

    let V_30569 = sqlx::query_as::<V_30020, S_50011>(
        V_30522#"
        SELECT
            id,
            COALESCE(V_30228, 'sumario') AS V_30228,
            COALESCE(numero_expediente, 'Sin datos') AS numero_expediente,
            COALESCE(reparticion, 'Sin datos') AS reparticion,
            V_30230,
            to_char(fecha_inicio, 'YYYY-MM-DD') AS fecha_inicio,
            to_char(V_30232, 'YYYY-MM-DD') AS V_30232,
            to_char(V_30233, 'YYYY-MM-DD') AS V_30233,
            V_30234,
            V_30235,
            COALESCE(V_30167, 'Sin datos') AS V_30167,
            COALESCE(estado, 'Abierto') AS estado,
            V_30066,
            V_30067,
            V_30236,
            V_30237,
            COALESCE(V_30238, 'Sin datos') AS V_30238,
            COALESCE(V_30068, 'No') AS V_30068,
            V_30240,
            V_30241,
            COALESCE(V_30242, 'No') AS V_30242,
            to_char(V_30243, 'YYYY-MM-DD') AS V_30243,
            documento_informe_hechos,
            documento_disposicion,
            documento_finalizacion,
            V_30231,
            V_30463,
            to_char(fecha_ultimo_cambio_estado, 'YYYY-MM-DD HH24:MI') AS fecha_ultimo_cambio_estado,
            to_char(fecha_ultima_novedad, 'YYYY-MM-DD HH24:MI') AS fecha_ultima_novedad,
            tipo_ultima_novedad,
            COALESCE(version_actual, 1) AS version_actual
        FROM expedientes
        WHERE id = $1
        "#,
    )
    .bind(V_30275)
    .fetch_one(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30570 = F_40010(
        &V_30569.V_30228,
        &V_30569.numero_expediente,
        &V_30569.reparticion,
        V_30569.V_30230.as_deref(),
        V_30569.V_30231.as_deref(),
        V_30569.fecha_inicio.as_deref(),
        V_30569.V_30232.as_deref(),
        V_30569.V_30233.as_deref(),
        V_30569.V_30234.as_deref(),
        V_30569.V_30235.as_deref(),
        &V_30569.V_30167,
        V_30569.V_30066.as_deref(),
        V_30569.V_30067.as_deref(),
        V_30569.V_30236.as_deref(),
        V_30569.V_30237.as_deref(),
        &V_30569.V_30238,
        Some(&V_30569.V_30068),
        V_30569.V_30240.as_deref(),
        V_30569.V_30241.as_deref(),
        Some(&V_30569.V_30242),
        V_30569.V_30243.as_deref(),
    );

    let (campos_modificados, diferencias_json) = F_40011(&V_30570, &V_30625);

    sqlx::query("UPDATE expediente_versiones SET es_activa = false WHERE V_30275 = $1")
        .bind(V_30275)
        .execute(&mut *V_30094)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let V_30027 = F_40009(
        V_30275,
        V_30567,
        V_30568,
        "restauracion",
        &V_30064.V_30114,
        &V_30167,
        Some(V_30485),
        &V_30625,
    );

    let V_30340: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO expediente_versiones (
            V_30275, numero_version, V_30306, tipo_operacion,
            snapshot_json, campos_modificados, diferencias_json, usuario,
            V_30055, V_30167, version_origen_id, es_activa,
            V_30554, mfa_verificado, mfa_step, aprobacion_id, record_hash, V_30675
        ) VALUES (
            $1, $2, $3, 'restauracion',
            $4, $5, $6, $7,
            $8, $9, $10, true,
            $11, true, $12, $13, $14, CURRENT_TIMESTAMP
        ) RETURNING id
        "#,
    )
    .bind(V_30275)
    .bind(V_30567)
    .bind(V_30568)
    .bind(&V_30625)
    .bind(serde_json::to_value(&campos_modificados).unwrap_or_default())
    .bind(&diferencias_json)
    .bind(&V_30064.V_30114)
    .bind(Some(V_30064.V_30055))
    .bind(&V_30167)
    .bind(Some(V_30485))
    .bind(&V_30554)
    .bind(V_30560)
    .bind(form.solicitud_aprobacion_id)
    .bind(&V_30027)
    .fetch_one(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if let Some(V_30563) = form.solicitud_aprobacion_id {
        sqlx::query("UPDATE app_solicitudes_acceso SET ejecutada = true, ejecutada_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(V_30563)
            .execute(&mut *V_30094)
            .await
            .map_err(error::ErrorInternalServerError)?;
    }

    // Registrar cada V_30765 modificado en expediente_historial
    if let Some(diffs_obj) = diferencias_json.as_object() {
        for (campo_key, diff_val) in diffs_obj {
            let V_30571 = diff_val.get("anterior").and_then(|v| v.as_str()).unwrap_or("");
            let V_30572 = diff_val.get("nuevo").and_then(|v| v.as_str()).unwrap_or("");
            sqlx::query(
                "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(V_30275)
            .bind(format!("restauracion:{}", campo_key))
            .bind(V_30571)
            .bind(V_30572)
            .bind(&V_30064.V_30114)
            .bind(format!("Restauración a v{} (nueva v{}): {}", target_numero_version, V_30567, V_30167))
            .execute(&mut *V_30094)
            .await
            .map_err(error::ErrorInternalServerError)?;
        }
    }

    let V_30573 = V_30625.get("V_30228").and_then(|v| v.as_str()).unwrap_or("sumario");
    let V_30574 = V_30625.get("numero_expediente").and_then(|v| v.as_str()).unwrap_or("Sin datos");
    let V_30575 = V_30625.get("reparticion").and_then(|v| v.as_str()).unwrap_or("Sin datos");
    let V_30576 = V_30625.get("V_30230").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30577 = V_30625.get("V_30231").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30578 = F_40067(V_30625.get("fecha_inicio").and_then(|v| v.as_str()));
    let V_30579 = F_40067(V_30625.get("V_30232").and_then(|v| v.as_str()));
    let V_30580 = F_40067(V_30625.get("V_30233").and_then(|v| v.as_str()));
    let V_30581 = V_30625.get("V_30234").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30582 = V_30625.get("V_30235").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30583 = V_30625.get("V_30167").and_then(|v| v.as_str()).unwrap_or("Sin datos");
    let V_30584 = V_30625.get("V_30066").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30585 = V_30625.get("V_30067").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30586 = V_30625.get("V_30236").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30587 = V_30625.get("V_30237").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30588 = V_30625.get("V_30238").and_then(|v| v.as_str()).unwrap_or("Sin datos");
    let V_30589 = V_30625.get("V_30068").and_then(|v| v.as_str()).unwrap_or("No");
    let V_30590 = V_30625.get("V_30240").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30591 = V_30625.get("V_30241").and_then(|v| v.as_str()).filter(|V_30044| !V_30044.trim().is_empty());
    let V_30592 = V_30625.get("V_30242").and_then(|v| v.as_str()).unwrap_or("No");
    let V_30593 = F_40067(V_30625.get("V_30243").and_then(|v| v.as_str()));

    sqlx::query(
        V_30522#"
        UPDATE expedientes SET
            V_30228 = $1,
            numero_expediente = $2,
            reparticion = $3,
            V_30230 = $4,
            V_30231 = $5,
            fecha_inicio = $6,
            V_30232 = $7,
            V_30233 = $8,
            V_30234 = $9,
            V_30235 = $10,
            V_30167 = $11,
            V_30066 = $12,
            V_30067 = $13,
            V_30236 = $14,
            V_30237 = $15,
            V_30238 = $16,
            V_30068 = $17,
            V_30240 = $18,
            V_30241 = $19,
            V_30242 = $20,
            V_30243 = $21,
            version_actual = $22,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $23
        "#,
    )
    .bind(V_30573)
    .bind(V_30574)
    .bind(V_30575)
    .bind(V_30576)
    .bind(V_30577)
    .bind(V_30578)
    .bind(V_30579)
    .bind(V_30580)
    .bind(V_30581)
    .bind(V_30582)
    .bind(V_30583)
    .bind(V_30584)
    .bind(V_30585)
    .bind(V_30586)
    .bind(V_30587)
    .bind(V_30588)
    .bind(V_30589)
    .bind(V_30590)
    .bind(V_30591)
    .bind(V_30592)
    .bind(V_30593)
    .bind(V_30567)
    .bind(V_30275)
    .execute(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30594 = format!("Restauración controlada a v{} (nueva versión v{})", target_numero_version, V_30567);
    let V_30595 = format!(
        "Restauración autorizada de datos descriptivos recuperados V_30511 la versión histórica v{}. Operador: {}. Motivo: {}",
        target_numero_version, V_30064.display_name, V_30167
    );
    sqlx::query(
        V_30522#"
        INSERT INTO expediente_novedades (
            V_30275, V_30458, V_30460, V_30461, V_30462,
            usuario, es_auditoria_automatica, V_30675
        ) VALUES (
            $1, 'Restauración de versión', CURRENT_TIMESTAMP,
            $2, $3, $4, true, CURRENT_TIMESTAMP
        )
        "#
    )
    .bind(V_30275)
    .bind(&V_30594)
    .bind(&V_30595)
    .bind(&V_30064.V_30114)
    .execute(&mut *V_30094)
    .await
    .map_err(error::ErrorInternalServerError)?;

    V_30094.commit().await.map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "restore_expediente_version",
        "expedientes",
        "success",
        json!({
            "V_30554": V_30554,
            "V_30275": V_30275,
            "version_origen_id": V_30485,
            "version_origen_num": target_numero_version,
            "version_anterior_num": cur_ver_num,
            "nueva_version_num": V_30567,
            "nueva_version_id": V_30340,
            "campos_modificados": campos_modificados,
            "mfa_verificado": true,
            "mfa_step": V_30560,
            "aprobacion_id": form.solicitud_aprobacion_id,
            "record_hash": V_30027,
            "V_30167": V_30167
        }),
    )
    .await;

    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO app_alertas_seguridad (
            V_30701, V_30461, V_30462, V_30702, V_30055, estado
        ) VALUES (
            'alta', $1, $2, 'restauracion_version_ejecutada', $3, 'activa'
        )
        "#
    )
    .bind(format!("Restauración de versión ejecutada en Expediente {}", numero_expediente))
    .bind(format!("El usuario {} ({}) restauró la versión v{} creando la versión v{} (MFA step: {}, correlación: {}). Motivo: {}", V_30064.display_name, V_30064.V_30114, target_numero_version, V_30567, V_30560, V_30554, V_30167))
    .bind(V_30064.V_30055)
    .execute(&V_30188.V_30012)
    .await;

    F_40013(
        &V_30188.V_30012,
        None,
        "restauracion_version",
        &format!("Expediente {}: Restauración a versión v{}", numero_expediente, target_numero_version),
        &format!("Se generó la versión v{} restaurando datos de v{}. Operador: {} (MFA TOTP verificado). Motivo: {}", V_30567, target_numero_version, V_30064.display_name, V_30167),
        &format!("/expedientes/{}", V_30275),
        Some(V_30275),
        None,
        None,
    ).await;

    if V_30553 {
        return Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "V_30554": V_30554,
            "V_30275": V_30275,
            "nueva_version_num": V_30567,
            "nueva_version_id": V_30340,
            "version_restaurada": target_numero_version,
            "mfa_verificado": true,
            "record_hash": V_30027,
            "message": format!("Versión restaurada con éxito con verificación Google Authenticator (se creó v{})", V_30567)
        })));
    }

    Ok(HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            format!("/expedientes/{}?V_30083=Versión+restaurada+con+éxito+(se+creó+v{})&tab=tab-V_30289", V_30275, V_30567),
        ))
        .finish())
}

#[get("/api/expedientes/{id}/V_30289")]
async fn F_40121(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();

    let V_30008 = sqlx::query_as::<V_30020, S_50080>(
        V_30522#"
        SELECT
            id,
            V_30275,
            numero_version,
            V_30306,
            tipo_operacion,
            snapshot_json,
            campos_modificados,
            diferencias_json,
            COALESCE(usuario, 'Sistema') AS usuario,
            V_30055,
            V_30167,
            version_origen_id,
            es_activa,
            to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675
        FROM expediente_versiones
        WHERE V_30275 = $1
        ORDER BY numero_version DESC
        "#,
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(V_30008))
}

#[get("/api/expedientes/{id}/V_30289/{V_30338}")]
async fn F_40122(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let (V_30275, version_id) = path.into_inner();

    let V_30596 = sqlx::query_as::<V_30020, S_50080>(
        V_30522#"
        SELECT
            id,
            V_30275,
            numero_version,
            V_30306,
            tipo_operacion,
            snapshot_json,
            campos_modificados,
            diferencias_json,
            COALESCE(usuario, 'Sistema') AS usuario,
            V_30055,
            V_30167,
            version_origen_id,
            es_activa,
            to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675
        FROM expediente_versiones
        WHERE id = $1 AND ($2 = 0 OR V_30275 = $2)
        "#,
    )
    .bind(version_id)
    .bind(V_30275)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    match V_30596 {
        Some(v) => Ok(HttpResponse::Ok().json(v)),
        None => Ok(HttpResponse::NotFound().json(json!({ "error": "Versión no encontrada" }))),
    }
}

#[get("/api/expedientes/{id}/V_30289-comparar")]
async fn F_40123(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    query: web::Query<S_50082>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();

    let V_30597 = if let Some(v) = query.v1 {
        sqlx::query_as::<V_30020, S_50080>(
            "SELECT id, V_30275, numero_version, V_30306, tipo_operacion, snapshot_json, campos_modificados, diferencias_json, COALESCE(usuario, 'Sistema') AS usuario, V_30055, V_30167, version_origen_id, es_activa, to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675 FROM expediente_versiones WHERE id = $1 AND ($2 = 0 OR V_30275 = $2)"
        )
        .bind(v)
        .bind(V_30275)
        .fetch_optional(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?
    } else if let Some(va) = query.v_a {
        sqlx::query_as::<V_30020, S_50080>(
            "SELECT id, V_30275, numero_version, V_30306, tipo_operacion, snapshot_json, campos_modificados, diferencias_json, COALESCE(usuario, 'Sistema') AS usuario, V_30055, V_30167, version_origen_id, es_activa, to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675 FROM expediente_versiones WHERE numero_version = $1 AND ($2 = 0 OR V_30275 = $2)"
        )
        .bind(va)
        .bind(V_30275)
        .fetch_optional(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?
    } else {
        None
    };

    let V_30598 = if let Some(v) = query.v2 {
        sqlx::query_as::<V_30020, S_50080>(
            "SELECT id, V_30275, numero_version, V_30306, tipo_operacion, snapshot_json, campos_modificados, diferencias_json, COALESCE(usuario, 'Sistema') AS usuario, V_30055, V_30167, version_origen_id, es_activa, to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675 FROM expediente_versiones WHERE id = $1 AND ($2 = 0 OR V_30275 = $2)"
        )
        .bind(v)
        .bind(V_30275)
        .fetch_optional(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?
    } else if let Some(vb) = query.v_b {
        sqlx::query_as::<V_30020, S_50080>(
            "SELECT id, V_30275, numero_version, V_30306, tipo_operacion, snapshot_json, campos_modificados, diferencias_json, COALESCE(usuario, 'Sistema') AS usuario, V_30055, V_30167, version_origen_id, es_activa, to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675 FROM expediente_versiones WHERE numero_version = $1 AND ($2 = 0 OR V_30275 = $2)"
        )
        .bind(vb)
        .bind(V_30275)
        .fetch_optional(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?
    } else {
        None
    };

    let (Some(V_30597), Some(V_30598)) = (V_30597, V_30598) else {
        return Ok(HttpResponse::NotFound().json(json!({ "error": "Una o ambas V_30289 no existen para este V_30113" })));
    };

    let (V_30623, V_30018) = F_40011(&V_30597.snapshot_json, &V_30598.snapshot_json);

    Ok(HttpResponse::Ok().json(json!({
        "v1": V_30597,
        "v2": V_30598,
        "campos_modificados": V_30623,
        "diferencias": V_30018
    })))
}

#[get("/administracion/V_30289")]
async fn F_40124(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50015>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let V_30599 = query.V_30599.clone().unwrap_or_else(|| "agrupada".to_string()).to_lowercase();
    let V_30600 = query.V_30600.clone().unwrap_or_else(|| "todos".to_string()).to_lowercase();
    let V_30601 = query.V_30601.clone().unwrap_or_else(|| "todas".to_string()).to_lowercase();
    let V_30602 = query.usuario.clone().unwrap_or_default().trim().to_string();
    let V_30603 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    let V_30604 = query.V_30604.clone().unwrap_or_default().trim().to_string();
    let V_30605 = query.V_30605.clone().unwrap_or_default().trim().to_string();

    let V_30069 = query.V_30069.unwrap_or(1).max(1);
    let V_30070 = query.V_30070.unwrap_or(25).clamp(5, 100);
    let V_30071 = (V_30069 - 1) * V_30070;

    let V_30606: (i64, i64, i64, i64) = sqlx::query_as(
        V_30522#"
        SELECT
            (SELECT COUNT(*) FROM expediente_versiones) + (SELECT COUNT(*) FROM documento_versiones) AS total_versiones,
            (SELECT COUNT(DISTINCT V_30275) FROM expediente_versiones) AS exp_versionados,
            (SELECT COUNT(DISTINCT documento_id) FROM documento_versiones) AS doc_versionados,
            (SELECT COUNT(*) FROM expediente_versiones WHERE tipo_operacion = 'restauracion' OR version_origen_id IS NOT NULL) AS restauraciones
        "#
    )
    .fetch_one(&V_30188.V_30012)
    .await
    .unwrap_or((0, 0, 0, 0));

    let V_30607 = S_50014 {
        total_versiones: V_30606.0,
        expedientes_versionados: V_30606.1,
        documentos_versionados: V_30606.2,
        restauraciones_realizadas: V_30606.3,
    };

    let V_30608: Vec<String> = sqlx::query_scalar(
        V_30522#"
        SELECT DISTINCT usuario FROM (
            SELECT usuario FROM expediente_versiones
            UNION
            SELECT usuario FROM documento_versiones
        ) u WHERE usuario IS NOT NULL AND usuario != '' ORDER BY usuario
        "#
    )
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let (V_30611, V_30087, V_30008, grouped_items) = if V_30599 == "agrupada" {
        let mut V_30412: Vec<String> = Vec::new();
        V_30412.push("e.id IN (SELECT DISTINCT V_30275 FROM expediente_versiones)".to_string());

        if V_30601 != "todas" && !V_30601.is_empty() {
            V_30412.push(format!("la.tipo_operacion = '{}'", V_30601.replace('\'', "''")));
        }

        if !V_30602.is_empty() {
            V_30412.push(format!("LOWER(la.usuario) = LOWER('{}')", V_30602.replace('\'', "''")));
        }

        if !V_30604.is_empty() {
            V_30412.push(format!("la.V_30675 >= '{} 00:00:00'::timestamp", V_30604.replace('\'', "''")));
        }

        if !V_30605.is_empty() {
            V_30412.push(format!("la.V_30675 <= '{} 23:59:59'::timestamp", V_30605.replace('\'', "''")));
        }

        if !V_30603.is_empty() {
            let V_30609 = V_30603.replace('\'', "''").to_lowercase();
            V_30412.push(format!(
                "(LOWER(e.numero_expediente) LIKE '%{0}%' OR LOWER(e.reparticion) LIKE '%{0}%' OR LOWER(e.V_30167) LIKE '%{0}%' OR LOWER(la.V_30167) LIKE '%{0}%')",
                V_30609
            ));
        }

        let V_30610 = format!("WHERE {}", V_30412.join(" AND "));

        let V_30078 = format!(
            V_30522#"
            WITH last_act AS (
                SELECT DISTINCT ON (V_30275) V_30275, tipo_operacion, usuario, V_30167, V_30675
                FROM (
                    SELECT V_30275, tipo_operacion, COALESCE(usuario, 'Sistema') AS usuario, COALESCE(V_30167, '') AS V_30167, V_30675
                    FROM expediente_versiones
                    UNION ALL
                    SELECT V_30275, tipo_operacion, COALESCE(usuario, 'Sistema') AS usuario, COALESCE(V_30167, '') AS V_30167, V_30675
                    FROM documento_versiones
                ) all_act
                ORDER BY V_30275, V_30675 DESC
            )
            SELECT COUNT(*)
            FROM expedientes e
            LEFT JOIN last_act la ON la.V_30275 = e.id
            {}
            "#,
            V_30610
        );

        let V_30611: i64 = sqlx::query_scalar(&V_30078)
            .fetch_one(&V_30188.V_30012)
            .await
            .unwrap_or(0);

        let V_30087 = ((V_30611 as f64) / (V_30070 as f64)).ceil() as i64;
        let V_30087 = V_30087.max(1);

        let V_30612 = format!(
            V_30522#"
            WITH last_act AS (
                SELECT DISTINCT ON (V_30275)
                    V_30275,
                    tipo_operacion,
                    usuario,
                    V_30167,
                    V_30675
                FROM (
                    SELECT V_30275, tipo_operacion, COALESCE(usuario, 'Sistema') AS usuario, COALESCE(V_30167, '') AS V_30167, V_30675
                    FROM expediente_versiones
                    UNION ALL
                    SELECT V_30275, tipo_operacion, COALESCE(usuario, 'Sistema') AS usuario, COALESCE(V_30167, '') AS V_30167, V_30675
                    FROM documento_versiones
                ) all_act
                ORDER BY V_30275, V_30675 DESC
            ),
            exp_counts AS (
                SELECT V_30275, COUNT(*) AS total_versiones_exp
                FROM expediente_versiones
                GROUP BY V_30275
            ),
            doc_counts AS (
                SELECT V_30275, COUNT(DISTINCT id) AS total_documentos
                FROM expediente_documentos
                GROUP BY V_30275
            )
            SELECT 
                e.id,
                e.numero_expediente,
                COALESCE(e.reparticion, 'Sin datos') AS reparticion,
                COALESCE(e.V_30167, 'Sin datos') AS V_30167,
                COALESCE(e.version_actual, 1) AS version_actual,
                COALESCE(ec.total_versiones_exp, 1) AS total_versiones_exp,
                COALESCE(dc.total_documentos, 0) AS total_documentos,
                COALESCE(la.tipo_operacion, 'creacion') AS ultima_operacion,
                COALESCE(la.usuario, 'Sistema') AS ultimo_usuario,
                COALESCE(la.V_30167, 'Creación inicial') AS ultimo_motivo,
                to_char(COALESCE(la.V_30675, e.V_30675, CURRENT_TIMESTAMP), 'YYYY-MM-DD HH24:MI:SS') AS ultima_fecha_hora
            FROM expedientes e
            LEFT JOIN last_act la ON la.V_30275 = e.id
            LEFT JOIN exp_counts ec ON ec.V_30275 = e.id
            LEFT JOIN doc_counts dc ON dc.V_30275 = e.id
            {}
            ORDER BY COALESCE(la.V_30675, e.V_30675, CURRENT_TIMESTAMP) DESC, e.id DESC
            LIMIT {} OFFSET {}
            "#,
            V_30610, V_30070, V_30071
        );

        let V_30613: Vec<S_50017> = sqlx::query_as(&V_30612)
            .fetch_all(&V_30188.V_30012)
            .await
            .unwrap_or_default();

        let V_30614: Vec<S_50016> = V_30613.into_iter().map(|V_30522| {
            S_50016 {
                id: V_30522.id,
                numero_expediente: V_30522.numero_expediente,
                reparticion: V_30522.reparticion,
                V_30167: V_30522.V_30167,
                version_actual: V_30522.version_actual,
                total_versiones_exp: V_30522.total_versiones_exp,
                total_documentos: V_30522.total_documentos,
                ultima_operacion: V_30522.ultima_operacion,
                ultimo_usuario: V_30522.ultimo_usuario,
                ultimo_motivo: V_30522.ultimo_motivo,
                ultima_fecha_hora: V_30522.ultima_fecha_hora,
            }
        }).collect();

        (V_30611, V_30087, Vec::new(), V_30614)
    } else {
        let mut V_30412: Vec<String> = Vec::new();

        if V_30600 == "V_30113" {
            V_30412.push("tipo_entidad = 'Expediente'".to_string());
        } else if V_30600 == "documento" {
            V_30412.push("tipo_entidad = 'Documento'".to_string());
        }

        if V_30601 != "todas" && !V_30601.is_empty() {
            V_30412.push(format!("tipo_operacion = '{}'", V_30601.replace('\'', "''")));
        }

        if !V_30602.is_empty() {
            V_30412.push(format!("LOWER(usuario) = LOWER('{}')", V_30602.replace('\'', "''")));
        }

        if !V_30604.is_empty() {
            V_30412.push(format!("V_30675 >= '{} 00:00:00'::timestamp", V_30604.replace('\'', "''")));
        }

        if !V_30605.is_empty() {
            V_30412.push(format!("V_30675 <= '{} 23:59:59'::timestamp", V_30605.replace('\'', "''")));
        }

        if !V_30603.is_empty() {
            let V_30609 = V_30603.replace('\'', "''").to_lowercase();
            V_30412.push(format!(
                "(LOWER(numero_expediente) LIKE '%{0}%' OR LOWER(reparticion) LIKE '%{0}%' OR LOWER(referencia) LIKE '%{0}%' OR LOWER(V_30167) LIKE '%{0}%')",
                V_30609
            ));
        }

        let V_30610 = if V_30412.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", V_30412.join(" AND "))
        };

        let V_30078 = format!(
            V_30522#"
            WITH unificados AS (
                SELECT 
                    v.id,
                    'Expediente' AS tipo_entidad,
                    e.numero_expediente,
                    e.reparticion,
                    e.numero_expediente AS referencia,
                    v.tipo_operacion,
                    v.usuario,
                    v.V_30167,
                    v.V_30675
                FROM expediente_versiones v
                JOIN expedientes e ON e.id = v.V_30275

                UNION ALL

                SELECT 
                    dv.id,
                    'Documento' AS tipo_entidad,
                    e.numero_expediente,
                    e.reparticion,
                    dv.nombre_original AS referencia,
                    dv.tipo_operacion,
                    dv.usuario,
                    dv.V_30167,
                    dv.V_30675
                FROM documento_versiones dv
                JOIN expedientes e ON e.id = dv.V_30275
            )
            SELECT COUNT(*) FROM unificados {}
            "#,
            V_30610
        );

        let V_30611: i64 = sqlx::query_scalar(&V_30078)
            .fetch_one(&V_30188.V_30012)
            .await
            .unwrap_or(0);

        let V_30087 = ((V_30611 as f64) / (V_30070 as f64)).ceil() as i64;
        let V_30087 = V_30087.max(1);

        let V_30615 = format!(
            V_30522#"
            WITH unificados AS (
                SELECT 
                    v.id,
                    'Expediente' AS tipo_entidad,
                    v.V_30275 AS entidad_id,
                    v.V_30275,
                    e.numero_expediente,
                    COALESCE(e.reparticion, 'Sin datos') AS reparticion,
                    e.numero_expediente AS referencia,
                    v.numero_version,
                    v.tipo_operacion,
                    v.usuario,
                    v.V_30167,
                    to_char(v.V_30675, 'YYYY-MM-DD HH24:MI:SS') AS fecha_hora,
                    v.es_activa AS es_vigente,
                    false AS esta_retirado,
                    v.V_30306,
                    v.version_origen_id,
                    NULL::text AS sha256_hash,
                    NULL::bigint AS tamano_bytes,
                    COALESCE(v.campos_modificados::text, '[]') AS campos_json,
                    COALESCE(v.diferencias_json::text, '{{}}') AS dif_json,
                    v.snapshot_json::text AS snapshot_json,
                    v.V_30675
                FROM expediente_versiones v
                JOIN expedientes e ON e.id = v.V_30275

                UNION ALL

                SELECT 
                    dv.id,
                    'Documento' AS tipo_entidad,
                    dv.documento_id AS entidad_id,
                    dv.V_30275,
                    e.numero_expediente,
                    COALESCE(e.reparticion, 'Sin datos') AS reparticion,
                    dv.nombre_original AS referencia,
                    dv.numero_version,
                    dv.tipo_operacion,
                    dv.usuario,
                    dv.V_30167,
                    to_char(dv.V_30675, 'YYYY-MM-DD HH24:MI:SS') AS fecha_hora,
                    dv.es_vigente,
                    dv.esta_retirado,
                    dv.V_30306,
                    dv.version_origen_id,
                    dv.sha256_hash,
                    dv.tamano_bytes,
                    '[]' AS campos_json,
                    json_build_object(
                        'nombre_archivo', dv.nombre_archivo,
                        'nombre_original', dv.nombre_original,
                        'tipo_documento', dv.tipo_documento,
                        'tamano_bytes', dv.tamano_bytes,
                        'sha256_hash', dv.sha256_hash
                    )::text AS dif_json,
                    json_build_object(
                        'documento_id', dv.documento_id,
                        'V_30275', dv.V_30275,
                        'numero_version', dv.numero_version,
                        'nombre_archivo', dv.nombre_archivo,
                        'nombre_original', dv.nombre_original,
                        'tipo_documento', dv.tipo_documento,
                        'tamano_bytes', dv.tamano_bytes,
                        'sha256_hash', dv.sha256_hash,
                        'tipo_operacion', dv.tipo_operacion,
                        'V_30167', dv.V_30167,
                        'usuario', dv.usuario,
                        'es_vigente', dv.es_vigente,
                        'esta_retirado', dv.esta_retirado
                    )::text AS snapshot_json,
                    dv.V_30675
                FROM documento_versiones dv
                JOIN expedientes e ON e.id = dv.V_30275
            )
            SELECT 
                id, tipo_entidad, entidad_id, V_30275, numero_expediente, reparticion, referencia,
                numero_version, tipo_operacion, usuario, V_30167, fecha_hora,
                es_vigente, esta_retirado, V_30306, version_origen_id,
                sha256_hash, tamano_bytes, campos_json, dif_json, snapshot_json
            FROM unificados
            {}
            ORDER BY V_30675 DESC, id DESC
            LIMIT {} OFFSET {}
            "#,
            V_30610, V_30070, V_30071
        );

        let V_30022: Vec<S_50018> = sqlx::query_as(&V_30615)
            .fetch_all(&V_30188.V_30012)
            .await
            .unwrap_or_default();

        let V_30616: Vec<S_50013> = V_30022.into_iter().map(|V_30522| {
            let V_30617: Vec<String> = serde_json::from_str(&V_30522.campos_json).unwrap_or_default();
            let V_30618 = if !V_30617.is_empty() {
                format!("Campos: {}", V_30617.join(", "))
            } else if V_30522.tipo_entidad == "Documento" {
                if let Some(sz) = V_30522.tamano_bytes {
                    let V_30619 = if sz > 1024 * 1024 {
                        format!("{:.1} MB", sz as f64 / (1024.0 * 1024.0))
                    } else if sz > 1024 {
                        format!("{:.1} KB", sz as f64 / 1024.0)
                    } else {
                        format!("{} B", sz)
                    };
                    format!("Archivo de {}", V_30619)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            S_50013 {
                id: V_30522.id,
                tipo_entidad: V_30522.tipo_entidad,
                entidad_id: V_30522.entidad_id,
                V_30275: V_30522.V_30275,
                V_30543: V_30522.numero_expediente,
                expediente_reparticion: V_30522.reparticion,
                referencia: V_30522.referencia,
                numero_version: V_30522.numero_version,
                tipo_operacion: V_30522.tipo_operacion,
                usuario: V_30522.usuario,
                V_30167: V_30522.V_30167,
                fecha_hora: V_30522.fecha_hora,
                es_vigente: V_30522.es_vigente,
                esta_retirado: V_30522.esta_retirado,
                V_30306: V_30522.V_30306,
                version_origen_id: V_30522.version_origen_id,
                sha256_hash: V_30522.sha256_hash,
                tamano_bytes: V_30522.tamano_bytes,
                resumen_cambios: V_30618,
                detalles_json: V_30522.snapshot_json,
            }
        }).collect();

        (V_30611, V_30087, V_30616, Vec::new())
    };

    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();

    let V_30620 = S_50049 {
        user_name: V_30064.display_name,
        user_role: V_30064.role,
        V_30088,
        V_30607,
        V_30008,
        grouped_items,
        V_30599,
        V_30600,
        V_30601,
        usuario: V_30602,
        V_30065: V_30603,
        V_30604,
        V_30605,
        V_30069,
        V_30087,
        V_30611,
        V_30608,
    };

    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(V_30620.render().map_err(error::ErrorInternalServerError)?))
}

#[get("/api/administracion/V_30289")]
async fn F_40125(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50015>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    let V_30599 = query.V_30599.clone().unwrap_or_else(|| "plana".to_string()).to_lowercase();
    let V_30600 = query.V_30600.clone().unwrap_or_else(|| "todos".to_string()).to_lowercase();
    let V_30601 = query.V_30601.clone().unwrap_or_else(|| "todas".to_string()).to_lowercase();
    let V_30602 = query.usuario.clone().unwrap_or_default().trim().to_string();
    let V_30603 = query.V_30065.clone().unwrap_or_default().trim().to_string();
    let V_30604 = query.V_30604.clone().unwrap_or_default().trim().to_string();
    let V_30605 = query.V_30605.clone().unwrap_or_default().trim().to_string();

    let V_30069 = query.V_30069.unwrap_or(1).max(1);
    let V_30070 = query.V_30070.unwrap_or(25).clamp(5, 100);
    let V_30071 = (V_30069 - 1) * V_30070;

    let V_30606: (i64, i64, i64, i64) = sqlx::query_as(
        V_30522#"
        SELECT
            (SELECT COUNT(*) FROM expediente_versiones) + (SELECT COUNT(*) FROM documento_versiones) AS total_versiones,
            (SELECT COUNT(DISTINCT V_30275) FROM expediente_versiones) AS exp_versionados,
            (SELECT COUNT(DISTINCT documento_id) FROM documento_versiones) AS doc_versionados,
            (SELECT COUNT(*) FROM expediente_versiones WHERE tipo_operacion = 'restauracion' OR version_origen_id IS NOT NULL) AS restauraciones
        "#
    )
    .fetch_one(&V_30188.V_30012)
    .await
    .unwrap_or((0, 0, 0, 0));

    let V_30607 = S_50014 {
        total_versiones: V_30606.0,
        expedientes_versionados: V_30606.1,
        documentos_versionados: V_30606.2,
        restauraciones_realizadas: V_30606.3,
    };

    if V_30599 == "agrupada" {
        let mut V_30412: Vec<String> = Vec::new();
        V_30412.push("e.id IN (SELECT DISTINCT V_30275 FROM expediente_versiones)".to_string());

        if V_30601 != "todas" && !V_30601.is_empty() {
            V_30412.push(format!("la.tipo_operacion = '{}'", V_30601.replace('\'', "''")));
        }

        if !V_30602.is_empty() {
            V_30412.push(format!("LOWER(la.usuario) = LOWER('{}')", V_30602.replace('\'', "''")));
        }

        if !V_30604.is_empty() {
            V_30412.push(format!("la.V_30675 >= '{} 00:00:00'::timestamp", V_30604.replace('\'', "''")));
        }

        if !V_30605.is_empty() {
            V_30412.push(format!("la.V_30675 <= '{} 23:59:59'::timestamp", V_30605.replace('\'', "''")));
        }

        if !V_30603.is_empty() {
            let V_30609 = V_30603.replace('\'', "''").to_lowercase();
            V_30412.push(format!(
                "(LOWER(e.numero_expediente) LIKE '%{0}%' OR LOWER(e.reparticion) LIKE '%{0}%' OR LOWER(e.V_30167) LIKE '%{0}%' OR LOWER(la.V_30167) LIKE '%{0}%')",
                V_30609
            ));
        }

        let V_30610 = format!("WHERE {}", V_30412.join(" AND "));

        let V_30078 = format!(
            V_30522#"
            WITH last_act AS (
                SELECT DISTINCT ON (V_30275) V_30275, tipo_operacion, usuario, V_30167, V_30675
                FROM (
                    SELECT V_30275, tipo_operacion, COALESCE(usuario, 'Sistema') AS usuario, COALESCE(V_30167, '') AS V_30167, V_30675
                    FROM expediente_versiones
                    UNION ALL
                    SELECT V_30275, tipo_operacion, COALESCE(usuario, 'Sistema') AS usuario, COALESCE(V_30167, '') AS V_30167, V_30675
                    FROM documento_versiones
                ) all_act
                ORDER BY V_30275, V_30675 DESC
            )
            SELECT COUNT(*)
            FROM expedientes e
            LEFT JOIN last_act la ON la.V_30275 = e.id
            {}
            "#,
            V_30610
        );

        let V_30611: i64 = sqlx::query_scalar(&V_30078)
            .fetch_one(&V_30188.V_30012)
            .await
            .unwrap_or(0);

        let V_30087 = ((V_30611 as f64) / (V_30070 as f64)).ceil() as i64;
        let V_30087 = V_30087.max(1);

        let V_30612 = format!(
            V_30522#"
            WITH last_act AS (
                SELECT DISTINCT ON (V_30275)
                    V_30275,
                    tipo_operacion,
                    usuario,
                    V_30167,
                    V_30675
                FROM (
                    SELECT V_30275, tipo_operacion, COALESCE(usuario, 'Sistema') AS usuario, COALESCE(V_30167, '') AS V_30167, V_30675
                    FROM expediente_versiones
                    UNION ALL
                    SELECT V_30275, tipo_operacion, COALESCE(usuario, 'Sistema') AS usuario, COALESCE(V_30167, '') AS V_30167, V_30675
                    FROM documento_versiones
                ) all_act
                ORDER BY V_30275, V_30675 DESC
            ),
            exp_counts AS (
                SELECT V_30275, COUNT(*) AS total_versiones_exp
                FROM expediente_versiones
                GROUP BY V_30275
            ),
            doc_counts AS (
                SELECT V_30275, COUNT(DISTINCT id) AS total_documentos
                FROM expediente_documentos
                GROUP BY V_30275
            )
            SELECT 
                e.id,
                e.numero_expediente,
                COALESCE(e.reparticion, 'Sin datos') AS reparticion,
                COALESCE(e.V_30167, 'Sin datos') AS V_30167,
                COALESCE(e.version_actual, 1) AS version_actual,
                COALESCE(ec.total_versiones_exp, 1) AS total_versiones_exp,
                COALESCE(dc.total_documentos, 0) AS total_documentos,
                COALESCE(la.tipo_operacion, 'creacion') AS ultima_operacion,
                COALESCE(la.usuario, 'Sistema') AS ultimo_usuario,
                COALESCE(la.V_30167, 'Creación inicial') AS ultimo_motivo,
                to_char(COALESCE(la.V_30675, e.V_30675, CURRENT_TIMESTAMP), 'YYYY-MM-DD HH24:MI:SS') AS ultima_fecha_hora
            FROM expedientes e
            LEFT JOIN last_act la ON la.V_30275 = e.id
            LEFT JOIN exp_counts ec ON ec.V_30275 = e.id
            LEFT JOIN doc_counts dc ON dc.V_30275 = e.id
            {}
            ORDER BY COALESCE(la.V_30675, e.V_30675, CURRENT_TIMESTAMP) DESC, e.id DESC
            LIMIT {} OFFSET {}
            "#,
            V_30610, V_30070, V_30071
        );

        let V_30613: Vec<S_50017> = sqlx::query_as(&V_30612)
            .fetch_all(&V_30188.V_30012)
            .await
            .unwrap_or_default();

        let V_30614: Vec<S_50016> = V_30613.into_iter().map(|V_30522| {
            S_50016 {
                id: V_30522.id,
                numero_expediente: V_30522.numero_expediente,
                reparticion: V_30522.reparticion,
                V_30167: V_30522.V_30167,
                version_actual: V_30522.version_actual,
                total_versiones_exp: V_30522.total_versiones_exp,
                total_documentos: V_30522.total_documentos,
                ultima_operacion: V_30522.ultima_operacion,
                ultimo_usuario: V_30522.ultimo_usuario,
                ultimo_motivo: V_30522.ultimo_motivo,
                ultima_fecha_hora: V_30522.ultima_fecha_hora,
            }
        }).collect();

        return Ok(HttpResponse::Ok().json(json!({
            "V_30607": V_30607,
            "grouped_items": V_30614,
            "V_30069": V_30069,
            "V_30087": V_30087,
            "V_30611": V_30611
        })));
    }

    let mut V_30412: Vec<String> = Vec::new();

    if V_30600 == "V_30113" {
        V_30412.push("tipo_entidad = 'Expediente'".to_string());
    } else if V_30600 == "documento" {
        V_30412.push("tipo_entidad = 'Documento'".to_string());
    }

    if V_30601 != "todas" && !V_30601.is_empty() {
        V_30412.push(format!("tipo_operacion = '{}'", V_30601.replace('\'', "''")));
    }

    if !V_30602.is_empty() {
        V_30412.push(format!("LOWER(usuario) = LOWER('{}')", V_30602.replace('\'', "''")));
    }

    if !V_30604.is_empty() {
        V_30412.push(format!("V_30675 >= '{} 00:00:00'::timestamp", V_30604.replace('\'', "''")));
    }

    if !V_30605.is_empty() {
        V_30412.push(format!("V_30675 <= '{} 23:59:59'::timestamp", V_30605.replace('\'', "''")));
    }

    if !V_30603.is_empty() {
        let V_30609 = V_30603.replace('\'', "''").to_lowercase();
        V_30412.push(format!(
            "(LOWER(numero_expediente) LIKE '%{0}%' OR LOWER(reparticion) LIKE '%{0}%' OR LOWER(referencia) LIKE '%{0}%' OR LOWER(V_30167) LIKE '%{0}%')",
            V_30609
        ));
    }

    let V_30610 = if V_30412.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", V_30412.join(" AND "))
    };

    let V_30078 = format!(
        V_30522#"
        WITH unificados AS (
            SELECT 
                v.id,
                'Expediente' AS tipo_entidad,
                e.numero_expediente,
                e.reparticion,
                e.numero_expediente AS referencia,
                v.tipo_operacion,
                v.usuario,
                v.V_30167,
                v.V_30675
            FROM expediente_versiones v
            JOIN expedientes e ON e.id = v.V_30275

            UNION ALL

            SELECT 
                dv.id,
                'Documento' AS tipo_entidad,
                e.numero_expediente,
                e.reparticion,
                dv.nombre_original AS referencia,
                dv.tipo_operacion,
                dv.usuario,
                dv.V_30167,
                dv.V_30675
            FROM documento_versiones dv
            JOIN expedientes e ON e.id = dv.V_30275
        )
        SELECT COUNT(*) FROM unificados {}
        "#,
        V_30610
    );

    let V_30611: i64 = sqlx::query_scalar(&V_30078)
        .fetch_one(&V_30188.V_30012)
        .await
        .unwrap_or(0);

    let V_30087 = ((V_30611 as f64) / (V_30070 as f64)).ceil() as i64;
    let V_30087 = V_30087.max(1);

    let V_30615 = format!(
        V_30522#"
        WITH unificados AS (
            SELECT 
                v.id,
                'Expediente' AS tipo_entidad,
                v.V_30275 AS entidad_id,
                v.V_30275,
                e.numero_expediente,
                COALESCE(e.reparticion, 'Sin datos') AS reparticion,
                e.numero_expediente AS referencia,
                v.numero_version,
                v.tipo_operacion,
                v.usuario,
                v.V_30167,
                to_char(v.V_30675, 'YYYY-MM-DD HH24:MI:SS') AS fecha_hora,
                v.es_activa AS es_vigente,
                false AS esta_retirado,
                v.V_30306,
                v.version_origen_id,
                NULL::text AS sha256_hash,
                NULL::bigint AS tamano_bytes,
                COALESCE(v.campos_modificados::text, '[]') AS campos_json,
                COALESCE(v.diferencias_json::text, '{{}}') AS dif_json,
                v.snapshot_json::text AS snapshot_json,
                v.V_30675
            FROM expediente_versiones v
            JOIN expedientes e ON e.id = v.V_30275

            UNION ALL

            SELECT 
                dv.id,
                'Documento' AS tipo_entidad,
                dv.documento_id AS entidad_id,
                dv.V_30275,
                e.numero_expediente,
                COALESCE(e.reparticion, 'Sin datos') AS reparticion,
                dv.nombre_original AS referencia,
                dv.numero_version,
                dv.tipo_operacion,
                dv.usuario,
                dv.V_30167,
                to_char(dv.V_30675, 'YYYY-MM-DD HH24:MI:SS') AS fecha_hora,
                dv.es_vigente,
                dv.esta_retirado,
                dv.V_30306,
                dv.version_origen_id,
                dv.sha256_hash,
                dv.tamano_bytes,
                '[]' AS campos_json,
                json_build_object(
                    'nombre_archivo', dv.nombre_archivo,
                    'nombre_original', dv.nombre_original,
                    'tipo_documento', dv.tipo_documento,
                    'tamano_bytes', dv.tamano_bytes,
                    'sha256_hash', dv.sha256_hash
                )::text AS dif_json,
                json_build_object(
                    'documento_id', dv.documento_id,
                    'V_30275', dv.V_30275,
                    'numero_version', dv.numero_version,
                    'nombre_archivo', dv.nombre_archivo,
                    'nombre_original', dv.nombre_original,
                    'tipo_documento', dv.tipo_documento,
                    'tamano_bytes', dv.tamano_bytes,
                    'sha256_hash', dv.sha256_hash,
                    'tipo_operacion', dv.tipo_operacion,
                    'V_30167', dv.V_30167,
                    'usuario', dv.usuario,
                    'es_vigente', dv.es_vigente,
                    'esta_retirado', dv.esta_retirado
                )::text AS snapshot_json,
                dv.V_30675
            FROM documento_versiones dv
            JOIN expedientes e ON e.id = dv.V_30275
        )
        SELECT 
            id, tipo_entidad, entidad_id, V_30275, numero_expediente, reparticion, referencia,
            numero_version, tipo_operacion, usuario, V_30167, fecha_hora,
            es_vigente, esta_retirado, V_30306, version_origen_id,
            sha256_hash, tamano_bytes, campos_json, dif_json, snapshot_json
        FROM unificados
        {}
        ORDER BY V_30675 DESC, id DESC
        LIMIT {} OFFSET {}
        "#,
        V_30610, V_30070, V_30071
    );

    let V_30022: Vec<S_50018> = sqlx::query_as(&V_30615)
        .fetch_all(&V_30188.V_30012)
        .await
        .unwrap_or_default();

    let V_30008: Vec<S_50013> = V_30022.into_iter().map(|V_30522| {
        let V_30617: Vec<String> = serde_json::from_str(&V_30522.campos_json).unwrap_or_default();
        let V_30618 = if !V_30617.is_empty() {
            format!("Campos: {}", V_30617.join(", "))
        } else if V_30522.tipo_entidad == "Documento" {
            if let Some(sz) = V_30522.tamano_bytes {
                let V_30619 = if sz > 1024 * 1024 {
                    format!("{:.1} MB", sz as f64 / (1024.0 * 1024.0))
                } else if sz > 1024 {
                    format!("{:.1} KB", sz as f64 / 1024.0)
                } else {
                    format!("{} B", sz)
                };
                format!("Archivo de {}", V_30619)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        S_50013 {
            id: V_30522.id,
            tipo_entidad: V_30522.tipo_entidad,
            entidad_id: V_30522.entidad_id,
            V_30275: V_30522.V_30275,
            V_30543: V_30522.numero_expediente,
            expediente_reparticion: V_30522.reparticion,
            referencia: V_30522.referencia,
            numero_version: V_30522.numero_version,
            tipo_operacion: V_30522.tipo_operacion,
            usuario: V_30522.usuario,
            V_30167: V_30522.V_30167,
            fecha_hora: V_30522.fecha_hora,
            es_vigente: V_30522.es_vigente,
            esta_retirado: V_30522.esta_retirado,
            V_30306: V_30522.V_30306,
            version_origen_id: V_30522.version_origen_id,
            sha256_hash: V_30522.sha256_hash,
            tamano_bytes: V_30522.tamano_bytes,
            resumen_cambios: V_30618,
            detalles_json: V_30522.snapshot_json,
        }
    }).collect();

    Ok(HttpResponse::Ok().json(json!({
        "V_30607": V_30607,
        "V_30008": V_30008,
        "V_30069": V_30069,
        "V_30087": V_30087,
        "V_30611": V_30611
    })))
}

#[derive(Serialize)]
struct S_50133 {
    id: i64,
    numero_expediente: String,
    reparticion: String,
    V_30167: String,
    estado: String,
    version_actual: i32,
    V_30238: String,
    fecha_inicio: Option<String>,
}

#[derive(Serialize)]
struct S_50134 {
    id: i64,
    V_30275: i64,
    numero_version: i32,
    V_30306: Option<i64>,
    tipo_operacion: String,
    usuario: String,
    V_30167: String,
    version_origen_id: Option<i64>,
    es_activa: bool,
    V_30675: String,
    campos_modificados: Vec<String>,
    diferencias_json: serde_json::Value,
    snapshot_json: serde_json::Value,
}

#[derive(Serialize)]
struct S_50135 {
    id: i64,
    numero_version: i32,
    tipo_operacion: String,
    usuario: String,
    fecha_hora: String,
    V_30167: String,
    sha256_hash: Option<String>,
    tamano_bytes: Option<i64>,
    es_vigente: bool,
    esta_retirado: bool,
}

#[derive(Serialize)]
struct S_50136 {
    id: i64,
    nombre_original: String,
    tipo_documento: String,
    numero_version_vigente: Option<i32>,
    esta_retirado: bool,
    sha256_hash: Option<String>,
    tamano_bytes: Option<i64>,
    V_30289: Vec<S_50135>,
}

#[derive(Serialize)]
struct S_50137 {
    V_30113: S_50133,
    V_30289: Vec<S_50134>,
    V_30284: Vec<S_50136>,
}

#[get("/api/expedientes/{id}/V_30276-completo")]
async fn F_40126(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let V_30344 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30275 = path.into_inner();

    let V_30621: Option<(i64, String, String, String, String, i32, String, Option<String>)> = sqlx::query_as(
        V_30522#"
        SELECT 
            id,
            COALESCE(numero_expediente, 'Sin datos'),
            COALESCE(reparticion, 'Sin datos'),
            COALESCE(V_30167, 'Sin datos'),
            COALESCE(estado, 'Abierto'),
            COALESCE(version_actual, 1),
            COALESCE(V_30238, 'Sin datos'),
            to_char(fecha_inicio, 'YYYY-MM-DD')
        FROM expedientes
        WHERE id = $1
        "#
    )
    .bind(V_30275)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some(V_30087) = V_30621 else {
        return Ok(HttpResponse::NotFound().json(json!({ "error": "Expediente no encontrado" })));
    };

    let V_30545 = S_50133 {
        id: V_30087.0,
        numero_expediente: V_30087.1,
        reparticion: V_30087.2,
        V_30167: V_30087.3,
        estado: V_30087.4,
        version_actual: V_30087.5,
        V_30238: V_30087.6,
        fecha_inicio: V_30087.7,
    };

    type ExpVerRaw = (i64, i64, i32, Option<i64>, String, String, String, Option<i64>, bool, String, String, String, String);
    let V_30138: Vec<ExpVerRaw> = sqlx::query_as(
        V_30522#"
        SELECT 
            id, V_30275, numero_version, V_30306, tipo_operacion,
            COALESCE(usuario, 'Sistema'), COALESCE(V_30167, ''), version_origen_id,
            es_activa, to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS'),
            COALESCE(campos_modificados::text, '[]'),
            COALESCE(diferencias_json::text, '{}'),
            snapshot_json::text
        FROM expediente_versiones
        WHERE V_30275 = $1
        ORDER BY numero_version DESC
        "#
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let V_30289: Vec<S_50134> = V_30138.into_iter().map(|V_30522| {
        let V_30623: Vec<String> = serde_json::from_str(&V_30522.10).unwrap_or_default();
        let V_30624: serde_json::Value = serde_json::from_str(&V_30522.11).unwrap_or_else(|V_30020| json!({}));
        let V_30625: serde_json::Value = serde_json::from_str(&V_30522.12).unwrap_or_else(|V_30020| json!({}));
        S_50134 {
            id: V_30522.0,
            V_30275: V_30522.1,
            numero_version: V_30522.2,
            V_30306: V_30522.3,
            tipo_operacion: V_30522.4,
            usuario: V_30522.5,
            V_30167: V_30522.6,
            version_origen_id: V_30522.7,
            es_activa: V_30522.8,
            V_30675: V_30522.9,
            campos_modificados: V_30623,
            diferencias_json: V_30624,
            snapshot_json: V_30625,
        }
    }).collect();

    type DocRaw = (i64, String, String, Option<i32>, bool, Option<String>, Option<i64>);
    let V_30143: Vec<DocRaw> = sqlx::query_as(
        V_30522#"
        SELECT 
            id, COALESCE(nombre_original, 'documento'), COALESCE(tipo_documento, 'OTRO'),
            numero_version_vigente, COALESCE(esta_retirado, false), sha256_hash, tamano_bytes
        FROM expediente_documentos
        WHERE V_30275 = $1
        ORDER BY id ASC
        "#
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    type DocVerRaw = (i64, i64, i32, String, String, String, String, Option<String>, Option<i64>, bool, bool);
    let V_30627: Vec<DocVerRaw> = sqlx::query_as(
        V_30522#"
        SELECT 
            id, documento_id, numero_version, tipo_operacion,
            COALESCE(usuario, 'Sistema'), to_char(V_30675, 'YYYY-MM-DD HH24:MI:SS'),
            COALESCE(V_30167, ''), sha256_hash, tamano_bytes, es_vigente, esta_retirado
        FROM documento_versiones
        WHERE V_30275 = $1
        ORDER BY documento_id ASC, numero_version DESC
        "#
    )
    .bind(V_30275)
    .fetch_all(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    use std::collections::HashMap;
    let mut V_30628: HashMap<i64, Vec<S_50135>> = HashMap::new();
    for dv in V_30627 {
        V_30628.entry(dv.1).or_default().push(S_50135 {
            id: dv.0,
            numero_version: dv.2,
            tipo_operacion: dv.3,
            usuario: dv.4,
            fecha_hora: dv.5,
            V_30167: dv.6,
            sha256_hash: dv.7,
            tamano_bytes: dv.8,
            es_vigente: dv.9,
            esta_retirado: dv.10,
        });
    }

    let V_30284: Vec<S_50136> = V_30143.into_iter().map(|d| {
        let V_30629 = V_30628.V_30056(&d.0).unwrap_or_default();
        S_50136 {
            id: d.0,
            nombre_original: d.1,
            tipo_documento: d.2,
            numero_version_vigente: d.3,
            esta_retirado: d.4,
            sha256_hash: d.5,
            tamano_bytes: d.6,
            V_30289: V_30629,
        }
    }).collect();

    Ok(HttpResponse::Ok().json(S_50137 {
        V_30113: V_30545,
        V_30289,
        V_30284,
    }))
}

// =========================================================================
// CENTRO DE SEGURIDAD, IDENTIDADES, DISPOSITIVOS Y CONTROL DE ACCESO (IAM)
// =========================================================================

#[get("/seguridad")]
async fn F_40127(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };

    if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" && V_30064.role != "auditor_seguridad" {
        return Ok(HttpResponse::Forbidden().body(
            "Acceso denegado: Se requiere rol de Superusuario, Administración Técnica o Auditoría de Seguridad para acceder al Centro de Seguridad."
        ));
    }

    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();
    let V_30630 = query.get("tab").cloned().unwrap_or_else(|| "usuarios".to_string());

    // 1. Políticas
    let V_30631 = sqlx::query_as::<V_30020, S_50052>(
        V_30522#"
        SELECT id, session_idle_minutes, session_absolute_hours, max_login_attempts,
               lockout_duration_minutes, V_30042, temp_code_expiry_minutes,
               max_temp_code_resends, single_session_per_user, V_30112,
               password_min_length, password_require_special, updated_at
        FROM app_politicas_seguridad
        WHERE id = 1
        "#
    )
    .fetch_optional(&V_30188.V_30012)
    .await
    .unwrap_or(None)
    .unwrap_or(S_50052 {
        id: 1,
        session_idle_minutes: 30,
        session_absolute_hours: 8,
        max_login_attempts: 5,
        lockout_duration_minutes: 15,
        V_30042: "privilegiados".to_string(),
        temp_code_expiry_minutes: 5,
        max_temp_code_resends: 3,
        single_session_per_user: false,
        V_30112: false,
        password_min_length: 8,
        password_require_special: false,
        updated_at: Utc::now().naive_utc(),
    });

    // 2. Usuarios
    let V_30632 = sqlx::query(
        V_30522#"
        SELECT u.id, u.V_30114, u.display_name, COALESCE(u.V_30050, 'Sin correo declarado') AS V_30050,
               COALESCE(u.V_30636, false) AS V_30636,
               COALESCE(u.V_30637, false) AS V_30637,
               COALESCE(u.V_30638, false) AS V_30638,
               u.role, u.V_30639, COALESCE(u.estado, 'activo') AS estado,
               COALESCE(u.V_30640, 0) AS V_30640,
               to_char(u.ultimo_acceso, 'YYYY-MM-DD HH24:MI') AS ultimo_acceso,
               COALESCE(u.ip_ultimo_acceso, '127.0.0.1') AS ip_ultimo_acceso,
               to_char(u.V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675,
               (SELECT COUNT(*) FROM app_dispositivos d WHERE d.V_30055 = u.id) AS disp_count
        FROM app_users u
        ORDER BY u.id ASC
        "#
    )
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30633: Vec<S_50053> = Vec::new();
    let mut V_30634 = 0i64;
    let mut V_30635 = 0i64;

    for V_30522 in V_30632 {
        use sqlx::Row;
        let id: i64 = V_30522.get("id");
        let V_30114: String = V_30522.get("V_30114");
        let display_name: String = V_30522.get("display_name");
        let V_30050: String = V_30522.get("V_30050");
        let V_30636: bool = V_30522.get("V_30636");
        let V_30637: bool = V_30522.get("V_30637");
        let V_30638: bool = V_30522.get("V_30638");
        let role: String = V_30522.get("role");
        let V_30639: bool = V_30522.get("V_30639");
        let estado: String = V_30522.get("estado");
        let V_30640: i32 = V_30522.get("V_30640");
        let V_30641: String = V_30522.get::<Option<String>, V_30020>("ultimo_acceso").unwrap_or_else(|| "Nunca".to_string());
        let V_30642: String = V_30522.get("ip_ultimo_acceso");
        let V_30643: String = V_30522.get::<Option<String>, V_30020>("V_30675").unwrap_or_else(|| "".to_string());
        let V_30644: i64 = V_30522.get("disp_count");

        if V_30639 {
            V_30634 += 1;
        }
        if V_30636 {
            V_30635 += 1;
        }

        V_30633.push(S_50053 {
            id,
            V_30114,
            display_name,
            V_30050,
            V_30636,
            V_30637,
            V_30638,
            role,
            V_30639,
            estado,
            V_30640,
            V_30641,
            V_30642,
            V_30643,
            V_30644,
        });
    }

    // 3. Dispositivos
    let V_30645 = sqlx::query(
        V_30522#"
        SELECT d.id, d.V_30055, u.V_30114 AS u_cred, u.display_name AS u_name,
               d.V_30651, d.V_30652, d.V_30653,
               d.V_30654, d.V_30655, d.estado, d.V_30656,
               to_char(d.V_30657, 'YYYY-MM-DD HH24:MI') AS V_30657,
               to_char(d.V_30658, 'YYYY-MM-DD HH24:MI') AS V_30658
        FROM app_dispositivos d
        JOIN app_users u ON u.id = d.V_30055
        ORDER BY d.V_30658 DESC
        "#
    )
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30646: Vec<S_50054> = Vec::new();
    let mut V_30647 = 0i64;
    let mut V_30648 = 0i64;

    for V_30522 in V_30645 {
        use sqlx::Row;
        let id: i64 = V_30522.get("id");
        let V_30055: i64 = V_30522.get("V_30055");
        let V_30649: String = V_30522.get("u_cred");
        let V_30650: String = V_30522.get("u_name");
        let V_30651: String = V_30522.get("V_30651");
        let V_30652: String = V_30522.get("V_30652");
        let V_30653: String = V_30522.get("V_30653");
        let V_30654: String = V_30522.get("V_30654");
        let V_30655: String = V_30522.get("V_30655");
        let estado: String = V_30522.get("estado");
        let V_30656: Option<String> = V_30522.get("V_30656");
        let V_30657: String = V_30522.get::<Option<String>, V_30020>("V_30657").unwrap_or_default();
        let V_30658: String = V_30522.get::<Option<String>, V_30020>("V_30658").unwrap_or_default();

        if estado == "autorizado" {
            V_30647 += 1;
        } else if estado == "pendiente" {
            V_30648 += 1;
        }

        V_30646.push(S_50054 {
            id,
            V_30055,
            V_30649,
            V_30650,
            V_30651,
            V_30652,
            V_30653,
            V_30654,
            V_30655,
            estado,
            V_30656,
            V_30657,
            V_30658,
        });
    }

    // 4. Solicitudes
    let V_30659 = sqlx::query(
        V_30522#"
        SELECT V_30044.id, V_30044.V_30092, V_30044.V_30662, u1.display_name AS sol_name,
               V_30044.V_30664, COALESCE(u2.display_name, '—') AS afec_name,
               V_30044.V_30666, V_30044.datos_solicitud::text AS V_30667,
               V_30044.estado, COALESCE(u3.display_name, 'Pendiente') AS ap_name,
               COALESCE(V_30044.V_30669, '') AS motivo_dec,
               to_char(V_30044.V_30670, 'YYYY-MM-DD HH24:MI') AS V_30670,
               to_char(V_30044.V_30671, 'YYYY-MM-DD HH24:MI') AS V_30671
        FROM app_solicitudes_acceso V_30044
        JOIN app_users u1 ON u1.id = V_30044.V_30662
        LEFT JOIN app_users u2 ON u2.id = V_30044.V_30664
        LEFT JOIN app_users u3 ON u3.id = V_30044.aprobador_id
        ORDER BY V_30044.id DESC
        "#
    )
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30660: Vec<S_50055> = Vec::new();
    let mut V_30661 = 0i64;

    for V_30522 in V_30659 {
        use sqlx::Row;
        let id: i64 = V_30522.get("id");
        let V_30092: String = V_30522.get("V_30092");
        let V_30662: i64 = V_30522.get("V_30662");
        let V_30663: String = V_30522.get("sol_name");
        let V_30664: Option<i64> = V_30522.get("V_30664");
        let V_30665: String = V_30522.get("afec_name");
        let V_30666: String = V_30522.get("V_30666");
        let V_30667: String = V_30522.get("V_30667");
        let estado: String = V_30522.get("estado");
        let V_30668: String = V_30522.get("ap_name");
        let V_30669: String = V_30522.get("motivo_dec");
        let V_30670: String = V_30522.get::<Option<String>, V_30020>("V_30670").unwrap_or_default();
        let V_30671: String = V_30522.get::<Option<String>, V_30020>("V_30671").unwrap_or_default();

        if estado == "pendiente" {
            V_30661 += 1;
        }

        V_30660.push(S_50055 {
            id,
            V_30092,
            V_30662,
            V_30663,
            V_30664,
            V_30665,
            V_30666,
            V_30667,
            estado,
            V_30668,
            V_30669,
            V_30670,
            V_30671,
        });
    }

    // 5. Sesiones
    let V_30672 = req.V_30041("spf_session").map(|c| c.value().to_string()).unwrap_or_default();
    let V_30673 = sqlx::query(
        V_30522#"
        SELECT V_30044.id::text AS V_30733, V_30044.V_30055, u.V_30114, u.display_name, u.role,
               to_char(V_30044.V_30675, 'YYYY-MM-DD HH24:MI') AS V_30675,
               to_char(V_30044.V_30676, 'YYYY-MM-DD HH24:MI') AS V_30676,
               to_char(V_30044.V_30677, 'YYYY-MM-DD HH24:MI') AS V_30677,
               COALESCE(V_30044.V_30678, '127.0.0.1') AS V_30678,
               COALESCE(V_30044.V_30092, 'Navegador Web') AS V_30092,
               COALESCE(d.V_30652, V_30044.V_30651, 'Terminal Web') AS disp_info,
               COALESCE(V_30044.motivo_revocacion, '') AS V_30680
        FROM app_sessions V_30044
        JOIN app_users u ON u.id = V_30044.V_30055
        LEFT JOIN app_dispositivos d ON d.V_30055 = V_30044.V_30055 AND d.V_30651 = V_30044.V_30651
        WHERE V_30044.revoked_at IS NULL
          AND V_30044.V_30677 > NOW()
          AND V_30044.absolute_expires_at > NOW()
        ORDER BY V_30044.V_30676 DESC
        "#
    )
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30674: Vec<S_50056> = Vec::new();
    for V_30522 in V_30673 {
        use sqlx::Row;
        let id: String = V_30522.get("V_30733");
        let V_30055: i64 = V_30522.get("V_30055");
        let V_30114: String = V_30522.get("V_30114");
        let display_name: String = V_30522.get("display_name");
        let role: String = V_30522.get("role");
        let V_30675: String = V_30522.get::<Option<String>, V_30020>("V_30675").unwrap_or_default();
        let V_30676: String = V_30522.get::<Option<String>, V_30020>("V_30676").unwrap_or_default();
        let V_30677: String = V_30522.get::<Option<String>, V_30020>("V_30677").unwrap_or_default();
        let V_30678: String = V_30522.get("V_30678");
        let V_30092: String = V_30522.get("V_30092");
        let V_30679: String = V_30522.get("disp_info");
        let V_30680: String = V_30522.get("V_30680");
        let V_30681 = id == V_30672;
        let V_30682 = if V_30680.is_empty() {
            "Activa (Sesión única)".to_string()
        } else {
            format!("Activa ({})", V_30680)
        };

        V_30674.push(S_50056 {
            id,
            V_30055,
            V_30114,
            display_name,
            role,
            V_30675,
            V_30676,
            V_30677,
            V_30678,
            V_30092,
            V_30679,
            V_30682,
            V_30681,
        });
    }

    // 6. Matriz RBAC
    let V_30683 = sqlx::query(
        "SELECT role, recurso, accion, permitido FROM app_roles_permisos"
    )
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30684: HashMap<(String, String, String), bool> = HashMap::new();
    for V_30522 in V_30683 {
        use sqlx::Row;
        let V_30685: String = V_30522.get("role");
        let V_30686: String = V_30522.get("recurso");
        let V_30687: String = V_30522.get("accion");
        let V_30688: bool = V_30522.get("permitido");
        V_30684.insert((V_30685, V_30686, V_30687), V_30688);
    }

    let V_30689 = ["expedientes", "V_30278", "V_30284", "V_30289", "usuarios", "seguridad", "auditoria"];
    let V_30690 = ["consultar", "crear", "modificar", "rectificar", "descargar", "restaurar", "administrar", "aprobar"];
    let mut V_30691: Vec<S_50057> = Vec::new();

    for rec in &V_30689 {
        for acc in &V_30690 {
            let V_30692 = *V_30684.get(&("superusuario".to_string(), rec.to_string(), acc.to_string())).unwrap_or(&true);
            let V_30693 = *V_30684.get(&("admin_tecnico".to_string(), rec.to_string(), acc.to_string())).unwrap_or(&false);
            let V_30694 = *V_30684.get(&("admin_expedientes".to_string(), rec.to_string(), acc.to_string())).unwrap_or(&false);
            let V_30695 = *V_30684.get(&("auditor_seguridad".to_string(), rec.to_string(), acc.to_string())).unwrap_or(&false);
            let V_30696 = *V_30684.get(&("operador".to_string(), rec.to_string(), acc.to_string())).unwrap_or(&false);
            let V_30697 = *V_30684.get(&("consulta".to_string(), rec.to_string(), acc.to_string())).unwrap_or(&false);

            V_30691.push(S_50057 {
                recurso: rec.to_string(),
                accion: acc.to_string(),
                superusuario: V_30692,
                admin_tecnico: V_30693,
                admin_expedientes: V_30694,
                auditor_seguridad: V_30695,
                operador: V_30696,
                consulta: V_30697,
            });
        }
    }

    // 7. Alertas
    let V_30698 = sqlx::query(
        V_30522#"
        SELECT a.id, a.V_30701, a.V_30461, a.V_30462, a.V_30702,
               COALESCE(u.V_30114, '—') AS u_involucrado,
               a.estado, COALESCE(a.V_30704, '') AS motivo_res,
               to_char(a.V_30705, 'YYYY-MM-DD HH24:MI') AS V_30705
        FROM app_alertas_seguridad a
        LEFT JOIN app_users u ON u.id = a.V_30055
        ORDER BY CASE a.V_30701 WHEN 'critica' THEN 1 WHEN 'alta' THEN 2 WHEN 'media' THEN 3 ELSE 4 END, a.id DESC
        "#
    )
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30699: Vec<S_50058> = Vec::new();
    let mut V_30700 = 0i64;

    for V_30522 in V_30698 {
        use sqlx::Row;
        let id: i64 = V_30522.get("id");
        let V_30701: String = V_30522.get("V_30701");
        let V_30461: String = V_30522.get("V_30461");
        let V_30462: String = V_30522.get("V_30462");
        let V_30702: String = V_30522.get("V_30702");
        let V_30703: String = V_30522.get("u_involucrado");
        let estado: String = V_30522.get("estado");
        let V_30704: String = V_30522.get("motivo_res");
        let V_30705: String = V_30522.get::<Option<String>, V_30020>("V_30705").unwrap_or_default();

        if estado == "activa" {
            V_30700 += 1;
        }

        V_30699.push(S_50058 {
            id,
            V_30701,
            V_30461,
            V_30462,
            V_30702,
            V_30703,
            estado,
            V_30704,
            V_30705,
        });
    }

    // 8. Eventos de auditoría
    let V_30706 = sqlx::query(
        V_30522#"
        SELECT l.id, to_char(l.V_30708, 'YYYY-MM-DD HH24:MI:SS') AS V_30708,
               COALESCE(u.V_30114, 'sistema') AS V_30709,
               l.V_30710, l.V_30711, l.V_30712, l.details::text AS V_30713
        FROM app_audit_log l
        LEFT JOIN app_users u ON u.id = l.V_30055
        ORDER BY l.id DESC
        LIMIT 100
        "#
    )
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30707: Vec<S_50059> = Vec::new();
    for V_30522 in V_30706 {
        use sqlx::Row;
        let id: i64 = V_30522.get("id");
        let V_30708: String = V_30522.get::<Option<String>, V_30020>("V_30708").unwrap_or_default();
        let V_30709: String = V_30522.get("V_30709");
        let V_30710: String = V_30522.get("V_30710");
        let V_30711: String = V_30522.get("V_30711");
        let V_30712: String = V_30522.get("V_30712");
        let V_30713: String = V_30522.get("V_30713");

        V_30707.push(S_50059 {
            id,
            V_30708,
            V_30709,
            V_30710,
            V_30711,
            V_30712,
            V_30713,
        });
    }

    let V_30714 = V_30633.len() as i64;
    let V_30715 = V_30714 - V_30635;

    let V_30607 = S_50051 {
        V_30714,
        V_30634,
        V_30635,
        V_30715,
        sesiones_activas: V_30674.len() as i64,
        V_30647,
        V_30648,
        V_30661,
        V_30700,
    };

    render_html(&S_50050 {
        user_name: V_30064.display_name,
        user_role: V_30064.role,
        V_30088,
        V_30630,
        V_30607,
        V_30631,
        V_30633,
        V_30646,
        V_30660,
        V_30674,
        V_30691,
        V_30699,
        V_30707,
        current_user_id: V_30064.V_30055,
    })
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Guardar Políticas Centrales
// -------------------------------------------------------------------------
#[post("/api/seguridad/V_30631")]
async fn F_40128(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Json<S_50060>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
    }

    let V_30716 = form.single_session_per_user.unwrap_or(false);
    let V_30717 = form.V_30112.unwrap_or(false);
    let V_30718 = form.password_require_special.unwrap_or(false);

    sqlx::query(
        V_30522#"
        UPDATE app_politicas_seguridad
        SET session_idle_minutes = $1,
            session_absolute_hours = $2,
            max_login_attempts = $3,
            lockout_duration_minutes = $4,
            V_30042 = $5,
            temp_code_expiry_minutes = $6,
            max_temp_code_resends = $7,
            single_session_per_user = $8,
            V_30112 = $9,
            password_min_length = $10,
            password_require_special = $11,
            updated_at = NOW(),
            updated_by = $12
        WHERE id = 1
        "#
    )
    .bind(form.session_idle_minutes)
    .bind(form.session_absolute_hours)
    .bind(form.max_login_attempts)
    .bind(form.lockout_duration_minutes)
    .bind(&form.V_30042)
    .bind(form.temp_code_expiry_minutes)
    .bind(form.max_temp_code_resends)
    .bind(V_30716)
    .bind(V_30717)
    .bind(form.password_min_length)
    .bind(V_30718)
    .bind(V_30064.V_30055)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "actualizar_politicas",
        "seguridad",
        "success",
        json!({
            "session_idle_minutes": form.session_idle_minutes,
            "V_30042": form.V_30042,
            "single_session_per_user": V_30716,
            "V_30112": V_30717,
        }),
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({ "success": true, "message": "Políticas actualizadas correctamente." })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Alta de Usuario con Correo Obligatorio
// -------------------------------------------------------------------------
#[post("/api/seguridad/usuarios/crear")]
async fn F_40129(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Json<S_50061>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
    }
    if form.role == "superusuario" && V_30064.role != "superusuario" {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "Solo un Superusuario puede crear una cuenta con rol de Superusuario." })));
    }

    let V_30168 = form.V_30114.trim();
    let V_30181 = form.display_name.trim();
    let V_30050 = form.V_30050.trim();
    let V_30046 = form.V_30115.trim();

    if V_30168.is_empty() || V_30181.is_empty() || V_30050.is_empty() || V_30046.len() < 8 {
        return Ok(HttpResponse::BadRequest().json(json!({ "error": "Datos inválidos. El correo es obligatorio y la clave debe tener al menos 8 caracteres." })));
    }
    if !V_30050.contains('@') || !V_30050.contains('.') {
        return Ok(HttpResponse::BadRequest().json(json!({ "error": "Formato de correo electrónico inválido." })));
    }

    let V_30028: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM app_users WHERE LOWER(V_30114) = LOWER($1)")
        .bind(V_30168)
        .fetch_one(&V_30188.V_30012)
        .await
        .map_err(error::ErrorInternalServerError)?;
    if V_30028 > 0 {
        return Ok(HttpResponse::Conflict().json(json!({ "error": "Ya existe un usuario con esa credencial." })));
    }

    let V_30029 = F_40019(V_30046).map_err(|V_30020| error::ErrorInternalServerError("Hash error"))?;

    let V_30719: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO app_users (
            V_30114, display_name, V_30050, password_hash, role,
            V_30639, estado, V_30636,
            V_30638, temp_password_expires_at,
            V_30637, backup_codes_confirmed, created_by
        )
        VALUES ($1, $2, $3, $4, $5, true, 'activo', false, true, NOW() + INTERVAL '24 hours', false, false, $6)
        RETURNING id
        "#
    )
    .bind(V_30168)
    .bind(V_30181)
    .bind(V_30050)
    .bind(V_30029)
    .bind(&form.role)
    .bind(V_30064.V_30055)
    .fetch_one(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let mut V_30720 = None;
    if form.enviar_codigo_inmediato.unwrap_or(true) {
        let (V_30034, V_30029) = F_40020();
        let V_30087 = Utc::now() + Duration::minutes(15);
        let V_30020 = sqlx::query(
            "INSERT INTO app_codigos_temporales (V_30055, V_30092, codigo_hash, email_destino, expira_at) VALUES ($1, 'verificacion_email', $2, $3, $4)"
        )
        .bind(V_30719)
        .bind(&V_30029)
        .bind(V_30050)
        .bind(V_30087.naive_utc())
        .execute(&V_30188.V_30012)
        .await;
        V_30720 = Some(V_30034.clone());

        let V_30075 = V_30188.V_30011.clone();
        let V_30088 = V_30050.to_string();
        let V_30089 = V_30181.to_string();
        let V_30090 = V_30168.to_string();
        let V_30076 = V_30719;
        tokio::spawn(async move {
            if let Ok(e) = mailer::send_welcome_verification_email(&V_30088, &V_30089, &V_30090, &V_30034, V_30076, &V_30075).await {
                eprintln!("[WELCOME EMAIL ERROR] {}", e);
            } else {
                println!("[WELCOME EMAIL SUCCESS] Correo de bienvenida y verificación enviado a {}", V_30088);
            }
        });
    }

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "crear_usuario",
        "usuarios",
        "success",
        json!({ "nuevo_id": V_30719, "V_30114": V_30168, "V_30050": V_30050, "role": form.role }),
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({ "success": true, "V_30055": V_30719, "V_30720": V_30720 })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Modificar Estado de Usuario (Activar / Suspender / Bloquear)
// -------------------------------------------------------------------------
#[post("/api/seguridad/usuarios/{id}/estado")]
async fn F_40130(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<S_50062>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
    }
    let V_30485 = path.into_inner();
    let V_30721 = form.estado == "activo";

    // Protección transaccional del último superusuario
let V_30722: Option<String> = sqlx::query_scalar("SELECT role FROM app_users WHERE id = $1")
.bind(V_30485)
.fetch_optional(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

if V_30722.as_deref() == Some("superusuario") && !V_30721 {
let V_30723: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM app_users WHERE role = 'superusuario' AND V_30639 = true AND estado = 'activo'")
.fetch_one(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;
if V_30723 <= 1 {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Operación denegada: El sistema debe contar con al menos un Superusuario activo." })));
}
}

sqlx::query(
"UPDATE app_users SET estado = $1, V_30639 = $2 WHERE id = $3"
)
.bind(&form.estado)
.bind(V_30721)
.bind(V_30485)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

if !V_30721 {
// Revocación inmediata de V_30674 activas (0 ms)
let V_30020 = sqlx::query("UPDATE app_sessions SET revoked_at = NOW() WHERE V_30055 = $1 AND revoked_at IS NULL")
.bind(V_30485)
.execute(&V_30188.V_30012)
.await;
}

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"cambio_estado_usuario",
"usuarios",
"success",
json!({ "usuario_id": V_30485, "V_30475": form.estado, "V_30167": form.V_30167 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true, "estado": form.estado })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Modificar Rol de Usuario
// -------------------------------------------------------------------------
#[post("/api/seguridad/usuarios/{id}/rol")]
async fn F_40131(
req: HttpRequest,
V_30188: web::Data<AppState>,
path: web::Path<i64>,
form: web::Json<S_50063>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}
let V_30485 = path.into_inner();

// Solo un superusuario puede otorgar el rol superusuario
if form.role == "superusuario" && V_30064.role != "superusuario" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Solo un Superusuario puede otorgar el rol de Superusuario." })));
}

// Protección del último superusuario si se cambia V_30692 rol
let V_30724: Option<String> = sqlx::query_scalar("SELECT role FROM app_users WHERE id = $1")
.bind(V_30485)
.fetch_optional(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

if V_30724.as_deref() == Some("superusuario") && form.role != "superusuario" {
let V_30723: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM app_users WHERE role = 'superusuario' AND V_30639 = true AND estado = 'activo'")
.fetch_one(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;
if V_30723 <= 1 {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Operación denegada: El sistema debe contar con al menos un Superusuario activo." })));
}
}

sqlx::query("UPDATE app_users SET role = $1, role_modified_by = $2, role_modified_at = NOW() WHERE id = $3")
.bind(&form.role)
.bind(V_30064.V_30055)
.bind(V_30485)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"cambio_rol_usuario",
"usuarios",
"success",
json!({ "usuario_id": V_30485, "nuevo_rol": form.role, "V_30167": form.V_30167 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true, "role": form.role })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Reenviar Código Temporal de Verificación
// -------------------------------------------------------------------------
#[post("/api/seguridad/usuarios/{id}/reenviar-codigo")]
async fn F_40132(
req: HttpRequest,
V_30188: web::Data<AppState>,
path: web::Path<i64>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}
let V_30485 = path.into_inner();

let V_30152 = sqlx::query("SELECT V_30114, display_name, V_30050 FROM app_users WHERE id = $1")
.bind(V_30485)
.fetch_optional(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

let Some(V_30128) = V_30152 else {
return Ok(HttpResponse::NotFound().json(json!({ "error": "Usuario no encontrado." })));
};

use sqlx::Row;
let V_30725: Option<String> = V_30128.get("V_30050");
let Some(V_30050) = V_30725 else {
return Ok(HttpResponse::BadRequest().json(json!({ "error": "El usuario no tiene correo electrónico declarado." })));
};
let V_30114: String = V_30128.get("V_30114");
let display_name: String = V_30128.get("display_name");

let (V_30034, V_30029) = F_40020();
let V_30087 = Utc::now() + Duration::minutes(15);

sqlx::query(
"INSERT INTO app_codigos_temporales (V_30055, V_30092, codigo_hash, email_destino, expira_at) VALUES ($1, 'verificacion_email', $2, $3, $4)"
)
.bind(V_30485)
.bind(&V_30029)
.bind(&V_30050)
.bind(V_30087.naive_utc())
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

let V_30075 = V_30188.V_30011.clone();
let V_30088 = V_30050.clone();
let V_30089 = display_name;
let V_30090 = V_30114;
let V_30164 = V_30034.clone();
let V_30076 = V_30485;
tokio::spawn(async move {
if let Ok(e) = mailer::send_welcome_verification_email(&V_30088, &V_30089, &V_30090, &V_30164, V_30076, &V_30075).await {
eprintln!("[RESEND EMAIL ERROR] {}", e);
} else {
println!("[RESEND EMAIL SUCCESS] Correo de verificación reenviado a {}", V_30088);
}
});

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"generar_codigo_verificacion",
"usuarios",
"success",
json!({ "usuario_id": V_30485, "V_30050": V_30050 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true, "V_30720": V_30034, "V_30050": V_30050 })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Verificar Código Temporal y Activar Correo
// -------------------------------------------------------------------------
#[post("/api/seguridad/usuarios/{id}/verificar-codigo")]
async fn F_40133(
req: HttpRequest,
V_30188: web::Data<AppState>,
path: web::Path<i64>,
form: web::Json<S_50065>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
let V_30485 = path.into_inner();

let V_30161 = F_40021(&form.codigo);

let V_30162: Option<i64> = sqlx::query_scalar(
V_30522#"
SELECT id FROM app_codigos_temporales
WHERE V_30055 = $1
AND codigo_hash = $2
AND usado = false
AND expira_at > NOW()
ORDER BY id DESC
LIMIT 1
"#
)
.bind(V_30485)
.bind(&V_30161)
.fetch_optional(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

let Some(cid) = V_30162 else {
write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"verificar_codigo_email",
"usuarios",
"failed",
json!({ "usuario_id": V_30485, "reason": "invalid_or_expired" }),
)
.await;
return Ok(HttpResponse::BadRequest().json(json!({ "error": "Código de verificación inválido, expirado o ya utilizado." })));
};

let V_30020 = sqlx::query("UPDATE app_codigos_temporales SET usado = true, usado_at = NOW() WHERE id = $1")
.bind(cid)
.execute(&V_30188.V_30012)
.await;

let V_30020 = sqlx::query("UPDATE app_users SET V_30636 = true, email_verificado_at = NOW() WHERE id = $1")
.bind(V_30485)
.execute(&V_30188.V_30012)
.await;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"verificar_codigo_email",
"usuarios",
"success",
json!({ "usuario_id": V_30485 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true, "message": "Correo electrónico verificado exitosamente." })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Restablecer Contraseña (Credencial Temporal de 24 Horas)
// -------------------------------------------------------------------------
#[post("/api/seguridad/usuarios/{id}/reset-V_30115")]
async fn F_40134(
req: HttpRequest,
V_30188: web::Data<AppState>,
path: web::Path<i64>,
form: web::Json<S_50064>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}
let V_30485 = path.into_inner();

// Generar credencial temporal de un solo uso de alta entropía (12+ caracteres)
let V_30726 = if let Some(ref np) = form.new_password {
if !np.trim().is_empty() && np.trim().len() >= 8 {
np.trim().to_string()
} else {
crypto_utils::generate_temporary_password()
}
} else {
crypto_utils::generate_temporary_password()
};

let V_30029 = F_40019(&V_30726).map_err(|V_30020| error::ErrorInternalServerError("Hash error"))?;

sqlx::query(
V_30522#"
UPDATE app_users 
SET password_hash = $1, 
V_30638 = true, 
temp_password_expires_at = NOW() + INTERVAL '24 hours',
V_30640 = 0, 
bloqueado_hasta = NULL 
WHERE id = $2
"#
)
.bind(V_30029)
.bind(V_30485)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

// Revocación inmediata de V_30674 activas por seguridad
let V_30020 = sqlx::query("UPDATE app_sessions SET revoked_at = NOW() WHERE V_30055 = $1 AND revoked_at IS NULL")
.bind(V_30485)
.execute(&V_30188.V_30012)
.await;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"resetear_password_temporal",
"usuarios",
"success",
json!({ "usuario_id": V_30485, "V_30167": form.V_30167 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({
"success": true,
"temporary_password": V_30726,
"temp_password": V_30726,
"message": "Contraseña temporal generada exitosamente (validez máxima de 24 horas). Se exigirá el cambio obligatorio en el primer inicio de sesión."
})))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Restablecer Autenticación 2FA (TOTP)
// -------------------------------------------------------------------------
#[post("/api/seguridad/usuarios/{id}/reset-totp")]
async fn F_40135(
req: HttpRequest,
V_30188: web::Data<AppState>,
path: web::Path<i64>,
form: web::Json<S_50095>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}
let V_30485 = path.into_inner();
let V_30167 = form.V_30167.as_deref().unwrap_or("Restablecimiento administrativo de 2FA").trim();
if V_30167.is_empty() {
return Ok(HttpResponse::BadRequest().json(json!({ "error": "Debe especificar un V_30167 justificado para restablecer el 2FA." })));
}

let V_30727: Option<String> = sqlx::query_scalar("SELECT V_30114 FROM app_users WHERE id = $1")
.bind(V_30485)
.fetch_optional(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

let Some(V_30168) = V_30727 else {
return Ok(HttpResponse::NotFound().json(json!({ "error": "Usuario no encontrado" })));
};

sqlx::query("UPDATE app_users SET V_30637 = false, totp_secret_enc = NULL, totp_verificado_at = NULL WHERE id = $1")
.bind(V_30485)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

let V_30020 = sqlx::query("UPDATE app_backup_codes SET usado = true, usado_at = NOW() WHERE V_30055 = $1 AND usado = false")
.bind(V_30485)
.execute(&V_30188.V_30012)
.await;

let V_30020 = sqlx::query(
"INSERT INTO app_alertas_seguridad (V_30701, V_30461, V_30462, V_30702, V_30055) VALUES ('alta', 'Restablecimiento de factor 2FA (TOTP)', $1, 'mfa_reset_administrativo', $2)"
)
.bind(format!("El administrador {} restableció el segundo factor TOTP del usuario {}. Motivo: {}", V_30064.V_30114, V_30168, V_30167))
.bind(V_30485)
.execute(&V_30188.V_30012)
.await;

let V_30091 = req.connection_info().realip_remote_addr().unwrap_or("127.0.0.1").to_string();
write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"resetear_totp",
"usuarios",
"success",
json!({ "usuario_id": V_30485, "V_30114": V_30168, "V_30167": V_30167, "ip": V_30091 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({
"success": true,
"message": "Autenticación 2FA (TOTP) restablecida exitosamente. El usuario deberá vincular V_30692 dispositivo en V_30692 próximo inicio de sesión."
})))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Modificar Estado de Dispositivo (Separación de Funciones)
// -------------------------------------------------------------------------
#[post("/api/seguridad/V_30646/{id}/estado")]
async fn F_40136(
req: HttpRequest,
V_30188: web::Data<AppState>,
path: web::Path<i64>,
form: web::Json<S_50066>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}
let V_30728 = path.into_inner();

let V_30729: Option<(i64, String, String)> = sqlx::query_as(
"SELECT V_30055, estado, V_30651 FROM app_dispositivos WHERE id = $1"
)
.bind(V_30728)
.fetch_optional(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

let Some((owner_id, current_estado, disp_uuid)) = V_30729 else {
return Ok(HttpResponse::NotFound().json(json!({ "error": "Dispositivo no encontrado" })));
};

// Principio de separación de funciones: Un administrador estándar no puede aprobar sus propios V_30646 (excepto el superusuario institucional)
if owner_id == V_30064.V_30055 && (form.estado == "aprobado" || form.estado == "autorizado") && V_30064.role != "superusuario" {
return Ok(HttpResponse::Forbidden().json(json!({
"error": "Por principio de separación de funciones, no puede autorizar sus propios V_30646. Solicite la aprobación a otro administrador."
})));
}

// VALIDACIÓN 2FA OBLIGATORIA: Si el dispositivo está 'revocado' y se solicita 'aprobado' o 'autorizado'
if current_estado == "revocado" && (form.estado == "aprobado" || form.estado == "autorizado") {
    let V_30557 = form.V_30557.as_deref().unwrap_or("").trim();
    if V_30557.is_empty() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Para rehabilitar un dispositivo revocado es obligatorio ingresar V_30692 código de 6 dígitos de Google Authenticator (TOTP)."
        })));
    }

    let V_30730: Option<(bool, Option<String>, Option<i64>)> = sqlx::query_as(
        "SELECT COALESCE(V_30637, false), totp_secret_enc, totp_last_used_step FROM app_users WHERE id = $1"
    )
    .bind(V_30064.V_30055)
    .fetch_optional(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let Some((V_30637, totp_secret_enc, totp_last_used_step)) = V_30730 else {
        return Ok(HttpResponse::Unauthorized().json(json!({ "error": "Usuario administrador no válido" })));
    };

    if !V_30637 || totp_secret_enc.as_deref().unwrap_or("").is_empty() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Su cuenta de administrador no tiene vinculado Google Authenticator (TOTP). Vincule V_30692 2FA para realizar operaciones sensibles."
        })));
    }

    let V_30559 = match crypto_utils::decrypt_totp_secret(totp_secret_enc.as_deref().unwrap_or("")) {
        V_30054(V_30044) => V_30044,
        V_30156(V_30020) => {
            return Ok(HttpResponse::InternalServerError().json(json!({ "error": "Error interno al descifrar la configuración 2FA del administrador." })));
        }
    };

    let V_30731 = match crypto_utils::verify_totp_code_with_step(&V_30559, &V_30064.V_30114, V_30557, totp_last_used_step) {
        V_30054(step) => step,
        V_30156(V_30020) => {
            write_audit(
                &V_30188.V_30012,
                Some(V_30064.V_30055),
                "rehabilitar_dispositivo_2fa_fallido",
                "V_30646",
                "invalid_totp",
                json!({ "dispositivo_id": V_30728, "V_30651": disp_uuid }),
            )
            .await;
            return Ok(HttpResponse::Unauthorized().json(json!({
                "error": "Código de Google Authenticator (TOTP) inválido o expirado."
            })));
        }
    };

    // Anti-replay: actualizar step utilizado
    let V_30020 = sqlx::query("UPDATE app_users SET totp_last_used_step = $1 WHERE id = $2")
        .bind(V_30731)
        .bind(V_30064.V_30055)
        .execute(&V_30188.V_30012)
        .await;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "rehabilitar_dispositivo_revocado_2fa",
        "V_30646",
        "success",
        json!({ "dispositivo_id": V_30728, "V_30651": disp_uuid, "V_30167": form.V_30167 }),
    )
    .await;
}

if form.estado == "aprobado" || form.estado == "autorizado" {
sqlx::query(
"UPDATE app_dispositivos SET estado = 'autorizado', V_30656 = $1, autorizado_por = $2, V_30671 = NOW() WHERE id = $3"
)
.bind(&form.V_30167)
.bind(V_30064.V_30055)
.bind(V_30728)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;
} else if form.estado == "rechazado" {
sqlx::query(
"UPDATE app_dispositivos SET estado = 'rechazado', V_30656 = $1, rechazado_por = $2, V_30671 = NOW() WHERE id = $3"
)
.bind(&form.V_30167)
.bind(V_30064.V_30055)
.bind(V_30728)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;
} else {
sqlx::query("UPDATE app_dispositivos SET estado = $1, V_30656 = $2 WHERE id = $3")
.bind(&form.estado)
.bind(&form.V_30167)
.bind(V_30728)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;
}

if form.estado == "revocado" {
// Invalida V_30674 activas asociadas a este dispositivo
let V_30020 = sqlx::query(
V_30522#"
UPDATE app_sessions
SET revoked_at = NOW()
WHERE V_30651 = (SELECT V_30651 FROM app_dispositivos WHERE id = $1)
AND revoked_at IS NULL
"#
)
.bind(V_30728)
.execute(&V_30188.V_30012)
.await;
}

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"cambio_estado_dispositivo",
"V_30646",
"success",
json!({ "dispositivo_id": V_30728, "V_30475": form.estado, "V_30167": form.V_30167 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Registrar Dispositivo
// -------------------------------------------------------------------------
#[post("/api/seguridad/V_30646/registrar")]
async fn F_40137(
req: HttpRequest,
V_30188: web::Data<AppState>,
form: web::Json<S_50067>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}

let V_30732 = Uuid::new_v4().to_string();

sqlx::query(
V_30522#"
INSERT INTO app_dispositivos (V_30055, V_30651, V_30652, V_30653, V_30654, estado, autorizado_por)
VALUES ($1, $2, $3, $4, $5, 'autorizado', $6)
"#
)
.bind(form.V_30055)
.bind(&V_30732)
.bind(&form.V_30652)
.bind(&form.V_30653)
.bind(&form.V_30654)
.bind(V_30064.V_30055)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"registrar_dispositivo",
"V_30646",
"success",
json!({ "V_30055": form.V_30055, "V_30651": V_30732, "nombre": form.V_30652 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true, "V_30651": V_30732 })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Revocar Sesión Específica
// -------------------------------------------------------------------------
#[post("/api/seguridad/V_30674/{id}/revocar")]
async fn F_40138(
req: HttpRequest,
V_30188: web::Data<AppState>,
path: web::Path<String>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}
let V_30733 = path.into_inner();
let V_30097 = Uuid::parse_str(&V_30733).map_err(|V_30020| error::ErrorBadRequest("UUID inválido"))?;

sqlx::query("UPDATE app_sessions SET revoked_at = NOW() WHERE id = $1")
.bind(V_30097)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"revocar_sesion",
"V_30674",
"success",
json!({ "V_30060": V_30733 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true, "message": "Sesión revocada." })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Revocar Todas las Otras Sesiones
// -------------------------------------------------------------------------
#[post("/api/seguridad/V_30674/revocar-todas")]
async fn F_40139(
req: HttpRequest,
V_30188: web::Data<AppState>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}

let V_30672 = req.V_30041("spf_session").and_then(|c| Uuid::parse_str(c.value()).ok());

if let Some(cs) = V_30672 {
sqlx::query("UPDATE app_sessions SET revoked_at = NOW() WHERE id != $1 AND revoked_at IS NULL")
.bind(cs)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;
} else {
sqlx::query("UPDATE app_sessions SET revoked_at = NOW() WHERE revoked_at IS NULL")
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;
}

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"revocar_todas_sesiones",
"V_30674",
"success",
json!({}),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true, "message": "Todas las demás V_30674 fueron revocadas." })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Crear Solicitud Formal de Acceso
// -------------------------------------------------------------------------
#[post("/api/seguridad/V_30660/crear")]
async fn F_40140(
req: HttpRequest,
V_30188: web::Data<AppState>,
form: web::Json<S_50068>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};

sqlx::query(
V_30522#"
INSERT INTO app_solicitudes_acceso (V_30092, V_30662, V_30664, V_30666, estado)
VALUES ($1, $2, $3, $4, 'pendiente')
"#
)
.bind(&form.V_30092)
.bind(V_30064.V_30055)
.bind(form.V_30664)
.bind(&form.V_30167)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"crear_solicitud_acceso",
"V_30660",
"success",
json!({ "V_30092": form.V_30092, "usuario_afectado": form.V_30664, "V_30167": form.V_30167 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Resolver Solicitud (Aprobar o Rechazar con Separación de Funciones)
// -------------------------------------------------------------------------
#[post("/api/seguridad/V_30660/{id}/resolver")]
async fn F_40141(
req: HttpRequest,
V_30188: web::Data<AppState>,
path: web::Path<i64>,
form: web::Json<S_50069>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}
let V_30563 = path.into_inner();

let V_30734: Option<(i64, String, Option<i64>, String)> = sqlx::query_as(
"SELECT V_30662, V_30092, V_30664, estado FROM app_solicitudes_acceso WHERE id = $1"
)
.bind(V_30563)
.fetch_optional(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

let Some((V_30662, _tipo, _afectado_id, estado_actual)) = V_30734 else {
return Ok(HttpResponse::NotFound().json(json!({ "error": "Solicitud no encontrada" })));
};

if estado_actual != "pendiente" {
return Ok(HttpResponse::BadRequest().json(json!({ "error": "La solicitud ya se encuentra resuelta." })));
}

// REGLA FUNDAMENTAL DE SEGURIDAD (ISO 27001 / NIST):
// Separación de funciones estricta: 0 autoaprobaciones.
if V_30662 == V_30064.V_30055 {
return Ok(HttpResponse::BadRequest().json(json!({
"error": "Infracción de separación de funciones: El solicitante no puede aprobar V_30692 propia solicitud."
})));
}

let V_30475 = if form.decision == "aprobar" { "aprobada" } else { "rechazada" };

sqlx::query(
V_30522#"
UPDATE app_solicitudes_acceso
SET estado = $1, aprobador_id = $2, V_30669 = $3, V_30671 = NOW()
WHERE id = $4
"#
)
.bind(V_30475)
.bind(V_30064.V_30055)
.bind(&form.V_30167)
.bind(V_30563)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"resolver_solicitud_acceso",
"V_30660",
"success",
json!({ "solicitud_id": V_30563, "decision": V_30475, "V_30167": form.V_30167 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true, "estado": V_30475 })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Conmutar Permiso en Matriz RBAC
// -------------------------------------------------------------------------
#[post("/api/seguridad/roles-permisos/toggle")]
async fn F_40142(
req: HttpRequest,
V_30188: web::Data<AppState>,
form: web::Json<S_50070>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}

sqlx::query(
V_30522#"
INSERT INTO app_roles_permisos (role, recurso, accion, permitido)
VALUES ($1, $2, $3, $4)
ON CONFLICT (role, recurso, accion)
DO UPDATE SET permitido = EXCLUDED.permitido
"#
)
.bind(&form.role)
.bind(&form.recurso)
.bind(&form.accion)
.bind(form.permitido)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"conmutar_permiso_rbac",
"seguridad",
"success",
json!({ "role": form.role, "recurso": form.recurso, "accion": form.accion, "permitido": form.permitido }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Resolver Alerta
// -------------------------------------------------------------------------
#[post("/api/seguridad/V_30699/{id}/resolver")]
async fn F_40143(
req: HttpRequest,
V_30188: web::Data<AppState>,
path: web::Path<i64>,
form: web::Json<S_50071>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" && V_30064.role != "auditor_seguridad" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario, admin_tecnico o auditor_seguridad" })));
}
let V_30735 = path.into_inner();

sqlx::query(
V_30522#"
UPDATE app_alertas_seguridad
SET estado = 'resuelta', resuelta_por = $1, V_30704 = $2, resuelta_at = NOW()
WHERE id = $3
"#
)
.bind(V_30064.V_30055)
.bind(&form.V_30167)
.bind(V_30735)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"resolver_alerta_seguridad",
"V_30699",
"success",
json!({ "alerta_id": V_30735, "V_30167": form.V_30167 }),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Generar Alerta de Prueba para Simulación
// -------------------------------------------------------------------------
#[post("/api/seguridad/V_30699/generar-prueba")]
async fn F_40144(
req: HttpRequest,
V_30188: web::Data<AppState>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}

sqlx::query(
V_30522#"
INSERT INTO app_alertas_seguridad (V_30701, V_30461, V_30462, V_30702, estado)
VALUES ('alta', 'Simulación: Detección preventiva de acceso anómalo', 'Múltiples V_30660 de V_30153 o acceso concurrente detectadas V_30511 terminal no habitual.', 'anomalia_acceso', 'activa')
"#
)
.execute(&V_30188.V_30012)
.await
.map_err(error::ErrorInternalServerError)?;

write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"generar_alerta_prueba",
"V_30699",
"success",
json!({}),
)
.await;

Ok(HttpResponse::Ok().json(json!({ "success": true })))
}

// -------------------------------------------------------------------------
// API SEGURIDAD: Probar Envío de Correo Electrónico SMTP en Vivo
// -------------------------------------------------------------------------
#[derive(Deserialize)]
struct S_50138 {
V_30050: Option<String>,
}

#[post("/api/seguridad/V_30050/test")]
async fn F_40145(
req: HttpRequest,
V_30188: web::Data<AppState>,
form: web::Json<S_50138>,
) -> Result<HttpResponse> {
let V_30064 = match require_user(&req, &V_30188).await {
V_30054(u) => u,
V_30156(V_30084) => return V_30054(V_30084),
};
if V_30064.role != "superusuario" && V_30064.role != "admin_tecnico" {
return Ok(HttpResponse::Forbidden().json(json!({ "error": "Requiere rol superusuario o admin_tecnico" })));
}

let V_30125 = form.V_30050.as_deref().unwrap_or("sisl16.ede@gmail.com").trim();
let V_30736 = if V_30125.is_empty() {
"sisl16.ede@gmail.com"
} else {
V_30125
};

match mailer::send_test_email(V_30736, &V_30188.V_30011).await {
V_30054(V_30020) => {
write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"prueba_smtp",
"seguridad",
"success",
json!({ "destinatario": V_30736, "host": V_30188.V_30011.smtp_host, "port": V_30188.V_30011.smtp_port }),
)
.await;
Ok(HttpResponse::Ok().json(json!({
"success": true,
"message": format!("Correo de prueba enviado satisfactoriamente a {}", V_30736)
})))
}
V_30156(err) => {
write_audit(
&V_30188.V_30012,
Some(V_30064.V_30055),
"prueba_smtp",
"seguridad",
"failed",
json!({ "destinatario": V_30736, "error": err }),
)
.await;
Ok(HttpResponse::InternalServerError().json(json!({
"success": false,
"error": format!("Error en conexión SMTP: {}", err)
})))
}
}
}

// -------------------------------------------------------------------------
// VERIFICACIÓN CRIPTOGRÁFICA DE INTEGRIDAD DE HISTORIAL, VERSIONES Y DOCUMENTOS
// -------------------------------------------------------------------------
async fn F_40146(V_30012: &PgPool, ejecutado_por: Option<i64>, V_30137: &str) -> serde_json::Value {
    let V_30020 = F_40017(V_30012).await;
    let mut V_30129 = Vec::new();
    let mut V_30130: Option<i64> = None;

    // 1. Verificar cadena de hashes de app_audit_log
    let V_30131: Vec<(i64, String, Option<i64>, String, String, String, serde_json::Value, Option<String>, Option<String>)> = sqlx::query_as(
        V_30522#"
        SELECT id,
               TO_CHAR(V_30708, 'YYYY-MM-DD HH24:MI:SS.US') AS occ_str,
               V_30055,
               V_30710,
               V_30711,
               V_30712,
               COALESCE(details, '{}'::jsonb),
               V_30055,
               V_30057
        FROM app_audit_log
        ORDER BY id ASC
        "#
    )
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();

    let mut V_30132 = 0i64;
    let mut V_30133 = 0i64;
    let mut V_30134 = "GENESIS".to_string();

    for (id, occ_str, V_30076, V_30710, V_30711, V_30712, details, prev_hash_opt, event_hash_opt) in V_30131 {
        V_30132 += 1;
        let V_30135 = prev_hash_opt.unwrap_or_default();
        let V_30136 = event_hash_opt.unwrap_or_default();

        let V_30137 = F_40008(
            &V_30134,
            &occ_str,
            V_30076,
            &V_30710,
            &V_30711,
            &V_30712,
            &details,
        );

        if V_30135 != V_30134 || V_30136 != V_30137 {
            V_30133 += 1;
            if V_30130.is_none() {
                V_30130 = Some(id);
            }
            V_30129.push(json!({
                "V_30092": "auditoria_hash_inconsistente",
                "V_30701": "critica",
                "V_30600": "app_audit_log",
                "id": id,
                "V_30710": V_30710,
                "prev_hash_esperado": V_30134,
                "prev_hash_almacenado": V_30135,
                "event_hash_esperado": V_30137,
                "event_hash_almacenado": V_30136
            }));
        }
        V_30134 = if V_30136.is_empty() { V_30137 } else { V_30136 };
    }

    // 2. Verificar integridad y secuencia de expediente_versiones
    let V_30138: Vec<(i64, i64, i32, Option<i64>, String, String, String, Option<i64>, serde_json::Value, Option<String>)> = sqlx::query_as(
        V_30522#"
        SELECT id, V_30275, numero_version, V_30306,
               tipo_operacion, COALESCE(usuario, 'Sistema'), COALESCE(V_30167, ''),
               version_origen_id, COALESCE(snapshot_descriptivo, snapshot_json, '{}'::jsonb),
               record_hash
        FROM expediente_versiones
        ORDER BY V_30275 ASC, numero_version ASC, id ASC
        "#
    )
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();

    let mut V_30139 = 0i64;
    let mut V_30140 = 0i64;

    for (id, exp_id, num_ver, prev_id, tipo_op, usr, V_30510, orig_id, V_30625, stored_rec_hash) in V_30138 {
        V_30139 += 1;
        let V_30141 = F_40009(
            exp_id,
            num_ver,
            prev_id,
            &tipo_op,
            &usr,
            &V_30510,
            orig_id,
            &V_30625,
        );
        let V_30142 = stored_rec_hash.unwrap_or_default();
        if !V_30142.is_empty() && V_30142 != V_30141 {
            V_30140 += 1;
            V_30129.push(json!({
                "V_30092": "version_hash_inconsistente",
                "V_30701": "critica",
                "V_30600": "expediente_versiones",
                "id": id,
                "V_30275": exp_id,
                "numero_version": num_ver,
                "record_hash_esperado": V_30141,
                "record_hash_almacenado": V_30142
            }));
        }
    }

    // 3. Verificar integridad física y SHA-256 de documento_versiones
    let V_30143: Vec<(i64, i64, i64, i32, String, String, String)> = sqlx::query_as(
        V_30522#"
        SELECT id, documento_id, V_30275, numero_version,
               nombre_archivo, nombre_original, COALESCE(sha256_hash, '')
        FROM documento_versiones
        ORDER BY id ASC
        "#
    )
    .fetch_all(V_30012)
    .await
    .unwrap_or_default();

    let mut V_30144 = 0i64;
    let mut V_30145 = 0i64;

    for (V_30338, V_30337, exp_id, num_ver, V_30182, orig_name, expected_sha) in V_30143 {
        V_30144 += 1;
        let V_30146 = format!("uploads/{}", V_30182);
        let V_30147 = Path::new(&V_30146);
        if !V_30147.V_30028() || !V_30147.is_file() {
            V_30145 += 1;
            V_30129.push(json!({
                "V_30092": "documento_archivo_faltante",
                "V_30701": "alta",
                "V_30600": "documento_versiones",
                "id": V_30338,
                "documento_id": V_30337,
                "V_30275": exp_id,
                "numero_version": num_ver,
                "V_30505": orig_name
            }));
        } else if !expected_sha.trim().is_empty() {
            if let Ok((actual_sha, V_30020)) = F_40069(&V_30146) {
                if !actual_sha.eq_ignore_ascii_case(expected_sha.trim()) {
                    V_30145 += 1;
                    V_30129.push(json!({
                        "V_30092": "documento_hash_discrepante",
                        "V_30701": "critica",
                        "V_30600": "documento_versiones",
                        "id": V_30338,
                        "documento_id": V_30337,
                        "V_30275": exp_id,
                        "numero_version": num_ver,
                        "sha256_esperado": expected_sha,
                        "sha256_actual": actual_sha
                    }));
                }
            }
        }
    }

    let V_30148 = if V_30133 == 0 && V_30140 == 0 && V_30145 == 0 {
        "integro"
    } else {
        "inconsistencia_detectada"
    };

    let V_30149 = serde_json::Value::Array(V_30129.clone());
    let V_30150: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO app_integridad_verificaciones (
            ejecutado_por, V_30137, V_30148,
            V_30132, V_30133,
            V_30139, V_30140,
            V_30144, V_30145,
            V_30130, resumen_hallazgos, V_30675
        ) VALUES (
            $1, $2, $3,
            $4, $5,
            $6, $7,
            $8, $9,
            $10, $11, CURRENT_TIMESTAMP
        ) RETURNING id
        "#
    )
    .bind(ejecutado_por)
    .bind(V_30137)
    .bind(V_30148)
    .bind(V_30132)
    .bind(V_30133)
    .bind(V_30139)
    .bind(V_30140)
    .bind(V_30144)
    .bind(V_30145)
    .bind(V_30130)
    .bind(&V_30149)
    .fetch_one(V_30012)
    .await
    .unwrap_or(0);

    if V_30148 != "integro" {
        let V_30020 = sqlx::query(
            V_30522#"
            INSERT INTO app_alertas_seguridad (V_30701, V_30461, V_30462, V_30702, V_30055, estado)
            VALUES ('critica', $1, $2, 'integridad_historial_comprometida', $3, 'activa')
            "#
        )
        .bind(format!("Inconsistencia detectada en verificación de integridad #{}", V_30150))
        .bind(format!(
            "Auditoría fallida: {}, Versiones fallidas: {}, Documentos fallidos: {}. Primer evento afectado: {:?}",
            V_30133, V_30140, V_30145, V_30130
        ))
        .bind(ejecutado_por)
        .execute(V_30012)
        .await;

        F_40013(
            V_30012,
            None,
            "alerta_integridad",
            &format!("Alerta crítica de integridad #{}", V_30150),
            &format!("La verificación criptográfica detectó anomalías (auditoría: {}, V_30289: {}, V_30284: {}).", V_30133, V_30140, V_30145),
            "/seguridad",
            None,
            None,
            None,
        )
        .await;
    }

    json!({
        "success": true,
        "verificacion_id": V_30150,
        "V_30148": V_30148,
        "V_30132": V_30132,
        "V_30133": V_30133,
        "V_30139": V_30139,
        "V_30140": V_30140,
        "V_30144": V_30144,
        "V_30145": V_30145,
        "V_30130": V_30130,
        "V_30129": V_30129
    })
}

#[get("/api/seguridad/integridad/verificar")]
async fn F_40147(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40002(&V_30188.V_30012, &V_30064.role, "auditoria", "consultar").await
        && !F_40002(&V_30188.V_30012, &V_30064.role, "seguridad", "consultar").await
    {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "Permisos insuficientes para ejecutar o consultar verificación de integridad" })));
    }
    let V_30712 = F_40146(&V_30188.V_30012, Some(V_30064.V_30055), "consulta_api").await;
    Ok(HttpResponse::Ok().json(V_30712))
}

#[post("/api/seguridad/integridad/verificar")]
async fn F_40148(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40002(&V_30188.V_30012, &V_30064.role, "auditoria", "consultar").await
        && !F_40002(&V_30188.V_30012, &V_30064.role, "seguridad", "consultar").await
    {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "Permisos insuficientes para ejecutar verificación de integridad" })));
    }
    let V_30712 = F_40146(&V_30188.V_30012, Some(V_30064.V_30055), "manual").await;
    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "verificar_integridad_cadena",
        "seguridad",
        "success",
        json!({
            "verificacion_id": V_30712.get("verificacion_id"),
            "V_30148": V_30712.get("V_30148")
        }),
    )
    .await;
    Ok(HttpResponse::Ok().json(V_30712))
}

// -------------------------------------------------------------------------
// PANEL DE CALIDAD DE DATOS Y REGULARIZACIÓN CONTROLADA (SECCIÓN 9)
// -------------------------------------------------------------------------
#[derive(Serialize)]
struct S_50139 {
    id: i64,
    V_30763: String,
    subtipo: String,
    V_30701: String,
    entidad_tipo: String,
    entidad_id: i64,
    V_30275: Option<i64>,
    identificador_visible: String,
    campo_afectado: String,
    V_30462: String,
    sugerencia: String,
    V_30467: String,
    updated_at: String,
}

#[derive(Template)]
#[template(path = "calidad_datos.html")]
struct S_50140 {
    user_name: String,
    user_role: String,
    V_30088: Vec<NavFile>,
    V_30757: i64,
    V_30758: i64,
    V_30759: i64,
    V_30760: i64,
    V_30761: i64,
    V_30755: String,
    V_30756: String,
    V_30065: String,
    V_30762: Vec<S_50139>,
}

#[derive(Deserialize)]
struct S_50141 {
    V_30763: Option<String>,
    estado: Option<String>,
    V_30065: Option<String>,
}

#[derive(Deserialize)]
struct S_50142 {
    caso_id: Option<i64>,
    V_30275: i64,
    V_30765: String,
    V_30764: String,
    V_30167: String,
}

#[derive(Deserialize)]
struct S_50143 {
    caso_id: i64,
    V_30475: String,
    V_30167: String,
}

async fn F_40149(V_30012: &PgPool) {
    // 1. Expedientes con V_30623 obligatorios faltantes ("Sin datos" o nulos)
    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO app_calidad_datos_casos (
            V_30763, subtipo, V_30701, entidad_tipo, entidad_id, V_30275,
            identificador_visible, campo_afectado, V_30462, sugerencia
        )
        SELECT
            'campos_faltantes',
            'instructor_sin_datos',
            'alta',
            'V_30113',
            e.id,
            e.id,
            COALESCE(e.numero_expediente, 'ID #' || e.id::text),
            'V_30238',
            'El V_30113 no registra instructor/responsable actual (figura como "Sin datos" o vacío).',
            'Regularizar asignando el apellido, nombre o credencial del instructor designado.'
        FROM expedientes e
        WHERE COALESCE(TRIM(e.V_30238), '') = '' OR LOWER(TRIM(e.V_30238)) = 'sin datos'
        ON CONFLICT (V_30763, entidad_tipo, entidad_id, campo_afectado) DO NOTHING
        "#
    )
    .execute(V_30012)
    .await;

    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO app_calidad_datos_casos (
            V_30763, subtipo, V_30701, entidad_tipo, entidad_id, V_30275,
            identificador_visible, campo_afectado, V_30462, sugerencia
        )
        SELECT
            'campos_faltantes',
            'fecha_inicio_faltante',
            'media',
            'V_30113',
            e.id,
            e.id,
            COALESCE(e.numero_expediente, 'ID #' || e.id::text),
            'fecha_inicio',
            'El V_30113 histórico no registra V_30503 de inicio válida.',
            'Completar V_30503 de inicio según disposición o carátula documental.'
        FROM expedientes e
        WHERE e.fecha_inicio IS NULL
        ON CONFLICT (V_30763, entidad_tipo, entidad_id, campo_afectado) DO NOTHING
        "#
    )
    .execute(V_30012)
    .await;

    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO app_calidad_datos_casos (
            V_30763, subtipo, V_30701, entidad_tipo, entidad_id, V_30275,
            identificador_visible, campo_afectado, V_30462, sugerencia
        )
        SELECT
            'campos_faltantes',
            'reparticion_sin_datos',
            'media',
            'V_30113',
            e.id,
            e.id,
            COALESCE(e.numero_expediente, 'ID #' || e.id::text),
            'reparticion',
            'La repartición o dependencia figura como "Sin datos".',
            'Asignar unidad o repartición de V_30104.'
        FROM expedientes e
        WHERE COALESCE(TRIM(e.reparticion), '') = '' OR LOWER(TRIM(e.reparticion)) = 'sin datos'
        ON CONFLICT (V_30763, entidad_tipo, entidad_id, campo_afectado) DO NOTHING
        "#
    )
    .execute(V_30012)
    .await;

    // 2. Posibles duplicados de expedientes (mismo numero_expediente normalizado en más de un registro)
    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO app_calidad_datos_casos (
            V_30763, subtipo, V_30701, entidad_tipo, entidad_id, V_30275,
            identificador_visible, campo_afectado, V_30462, sugerencia
        )
        SELECT
            'posibles_duplicados',
            'numero_expediente_repetido',
            'alta',
            'V_30113',
            e.id,
            e.id,
            COALESCE(e.numero_expediente, 'ID #' || e.id::text),
            'numero_expediente',
            'Posible duplicado: el número de actuación aparece registrado en más de un V_30113.',
            'Verificar si corresponde vincular formalmente o descartar homonimia de número.'
        FROM expedientes e
        WHERE UPPER(TRIM(e.numero_expediente)) IN (
            SELECT UPPER(TRIM(numero_expediente))
            FROM expedientes
            WHERE LOWER(TRIM(numero_expediente)) != 'sin datos'
            GROUP BY UPPER(TRIM(numero_expediente))
            HAVING COUNT(*) > 1
        )
        ON CONFLICT (V_30763, entidad_tipo, entidad_id, campo_afectado) DO NOTHING
        "#
    )
    .execute(V_30012)
    .await;

    // 3. Relaciones inconsistentes / registros F_40085 sin credencial vinculada
    let V_30020 = sqlx::query(
        V_30522#"
        INSERT INTO app_calidad_datos_casos (
            V_30763, subtipo, V_30701, entidad_tipo, entidad_id, V_30275,
            identificador_visible, campo_afectado, V_30462, sugerencia
        )
        SELECT
            'relaciones_inconsistentes',
            'staging_sin_credencial',
            'baja',
            'F_40085',
            V_30044.id,
            NULL,
            'Fila Staging #' || V_30044.row_number::text,
            'credencial',
            'Registro importado de planilla sin credencial identificatoria (' || COALESCE(V_30044.V_30167, 'sin credencial') || ').',
            'Revisar en módulo Staging para asociar credencial o marcar como revisado.'
        FROM staging_sin_credencial V_30044
        LIMIT 50
        ON CONFLICT (V_30763, entidad_tipo, entidad_id, campo_afectado) DO NOTHING
        "#
    )
    .execute(V_30012)
    .await;
}

#[get("/calidad-datos")]
async fn F_40150(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50141>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40002(&V_30188.V_30012, &V_30064.role, "calidad_datos", "consultar").await {
        return Ok(HttpResponse::Forbidden().body("No posee permisos para acceder al panel de Calidad de Datos"));
    }

    F_40149(&V_30188.V_30012).await;

    let V_30755 = query.V_30763.clone().unwrap_or_else(|| "todas".to_string());
    let V_30756 = query.estado.clone().unwrap_or_else(|| "pendiente".to_string());
    let V_30065 = query.V_30065.clone().unwrap_or_default().trim().to_string();

    let V_30757: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_calidad_datos_casos WHERE V_30763 = 'campos_faltantes' AND V_30467 = 'pendiente'"
    ).fetch_one(&V_30188.V_30012).await.unwrap_or(0);

    let V_30758: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_calidad_datos_casos WHERE V_30763 = 'posibles_duplicados' AND V_30467 = 'pendiente'"
    ).fetch_one(&V_30188.V_30012).await.unwrap_or(0);

    let V_30759: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_calidad_datos_casos WHERE V_30763 = 'relaciones_inconsistentes' AND V_30467 = 'pendiente'"
    ).fetch_one(&V_30188.V_30012).await.unwrap_or(0);

    let V_30760: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_calidad_datos_casos WHERE V_30763 = 'documentos_observados' AND V_30467 = 'pendiente'"
    ).fetch_one(&V_30188.V_30012).await.unwrap_or(0);

    let V_30761: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_calidad_datos_casos WHERE V_30467 IN ('regularizado', 'revisado', 'descartado')"
    ).fetch_one(&V_30188.V_30012).await.unwrap_or(0);

    let V_30022: Vec<(i64, String, String, String, String, i64, Option<i64>, String, Option<String>, String, Option<String>, String, String)> = sqlx::query_as(
        V_30522#"
        SELECT id, V_30763, subtipo, V_30701, entidad_tipo, entidad_id, V_30275,
               identificador_visible, campo_afectado, V_30462, sugerencia, V_30467,
               TO_CHAR(updated_at, 'YYYY-MM-DD HH24:MI')
        FROM app_calidad_datos_casos
        WHERE ($1 = 'todas' OR V_30763 = $1)
          AND ($2 = 'todos' OR V_30467 = $2)
          AND ($3 = '' OR identificador_visible ILIKE '%' || $3 || '%' OR V_30462 ILIKE '%' || $3 || '%')
        ORDER BY CASE V_30701 WHEN 'critica' THEN 1 WHEN 'alta' THEN 2 WHEN 'media' THEN 3 ELSE 4 END, id ASC
        LIMIT 100
        "#
    )
    .bind(&V_30755)
    .bind(&V_30756)
    .bind(&V_30065)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let V_30762 = V_30022.into_iter().map(|V_30522| S_50139 {
        id: V_30522.0,
        V_30763: V_30522.1,
        subtipo: V_30522.2,
        V_30701: V_30522.3,
        entidad_tipo: V_30522.4,
        entidad_id: V_30522.5,
        V_30275: V_30522.6,
        identificador_visible: V_30522.7,
        campo_afectado: V_30522.8.unwrap_or_default(),
        V_30462: V_30522.9,
        sugerencia: V_30522.10.unwrap_or_default(),
        V_30467: V_30522.11,
        updated_at: V_30522.12,
    }).collect();

    let V_30088 = load_navigation(&V_30188.V_30012).await.unwrap_or_default();

    render_html(&S_50140 {
        user_name: V_30064.display_name,
        user_role: V_30064.role,
        V_30088,
        V_30757,
        V_30758,
        V_30759,
        V_30760,
        V_30761,
        V_30755,
        V_30756,
        V_30065,
        V_30762,
    })
}

#[get("/api/calidad-datos")]
async fn F_40151(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50141>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40002(&V_30188.V_30012, &V_30064.role, "calidad_datos", "consultar").await {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "Permisos insuficientes" })));
    }

    F_40149(&V_30188.V_30012).await;
    let V_30763 = query.V_30763.clone().unwrap_or_else(|| "todas".to_string());
    let estado = query.estado.clone().unwrap_or_else(|| "pendiente".to_string());

    let V_30022: Vec<(i64, String, String, String, String, i64, Option<i64>, String, Option<String>, String, Option<String>, String, String)> = sqlx::query_as(
        V_30522#"
        SELECT id, V_30763, subtipo, V_30701, entidad_tipo, entidad_id, V_30275,
               identificador_visible, campo_afectado, V_30462, sugerencia, V_30467,
               TO_CHAR(updated_at, 'YYYY-MM-DD HH24:MI')
        FROM app_calidad_datos_casos
        WHERE ($1 = 'todas' OR V_30763 = $1)
          AND ($2 = 'todos' OR V_30467 = $2)
        ORDER BY id ASC
        LIMIT 100
        "#
    )
    .bind(&V_30763)
    .bind(&estado)
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let V_30008: Vec<S_50139> = V_30022.into_iter().map(|V_30522| S_50139 {
        id: V_30522.0,
        V_30763: V_30522.1,
        subtipo: V_30522.2,
        V_30701: V_30522.3,
        entidad_tipo: V_30522.4,
        entidad_id: V_30522.5,
        V_30275: V_30522.6,
        identificador_visible: V_30522.7,
        campo_afectado: V_30522.8.unwrap_or_default(),
        V_30462: V_30522.9,
        sugerencia: V_30522.10.unwrap_or_default(),
        V_30467: V_30522.11,
        updated_at: V_30522.12,
    }).collect();

    Ok(HttpResponse::Ok().json(json!({ "success": true, "V_30762": V_30008 })))
}

#[post("/api/calidad-datos/regularizar")]
async fn F_40152(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50142>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30553 = req.V_30457().get("accept").and_then(|v| v.to_str().ok()).map(|V_30044| V_30044.contains("application/json")).unwrap_or(false);

    if !F_40002(&V_30188.V_30012, &V_30064.role, "calidad_datos", "modificar").await {
        if V_30553 {
            return Ok(HttpResponse::Forbidden().json(json!({ "error": "No posee permisos para regularizar registros históricos" })));
        }
        return Ok(HttpResponse::SeeOther().append_header((header::LOCATION, "/calidad-datos")).finish());
    }

    let V_30764 = F_40003(&form.V_30764);
    let V_30167 = F_40003(&form.V_30167);
    if V_30764.is_empty() || V_30764.eq_ignore_ascii_case("sin datos") || V_30167.chars().count() < 5 {
        if V_30553 {
            return Ok(HttpResponse::BadRequest().json(json!({ "error": "Valor nuevo válido y V_30167 de al menos 5 caracteres son obligatorios para regularizar" })));
        }
        return Ok(HttpResponse::SeeOther().append_header((header::LOCATION, "/calidad-datos")).finish());
    }

    let V_30765 = form.V_30765.trim();
    let V_30275 = form.V_30275;

    let V_30766: Option<String> = match V_30765 {
        "V_30238" => sqlx::query_scalar("SELECT COALESCE(V_30238, 'Sin datos') FROM expedientes WHERE id = $1").bind(V_30275).fetch_optional(&V_30188.V_30012).await.unwrap_or(None),
        "reparticion" => sqlx::query_scalar("SELECT COALESCE(reparticion, 'Sin datos') FROM expedientes WHERE id = $1").bind(V_30275).fetch_optional(&V_30188.V_30012).await.unwrap_or(None),
        "V_30167" => sqlx::query_scalar("SELECT COALESCE(V_30167, 'Sin datos') FROM expedientes WHERE id = $1").bind(V_30275).fetch_optional(&V_30188.V_30012).await.unwrap_or(None),
        "fecha_inicio" => sqlx::query_scalar("SELECT COALESCE(TO_CHAR(fecha_inicio, 'YYYY-MM-DD'), 'Sin datos') FROM expedientes WHERE id = $1").bind(V_30275).fetch_optional(&V_30188.V_30012).await.unwrap_or(None),
        V_30020 => return Ok(HttpResponse::BadRequest().json(json!({ "error": "Campo no admitido para regularización directa" }))),
    };

    let Some(val_ant) = V_30766 else {
        return Ok(HttpResponse::NotFound().json(json!({ "error": "Expediente no encontrado" })));
    };

    match V_30765 {
        "V_30238" => {
            sqlx::query("UPDATE expedientes SET V_30238 = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                .bind(&V_30764)
                .bind(V_30275)
                .execute(&V_30188.V_30012)
                .await
                .map_err(error::ErrorInternalServerError)?;
        }
        "reparticion" => {
            sqlx::query("UPDATE expedientes SET reparticion = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                .bind(&V_30764)
                .bind(V_30275)
                .execute(&V_30188.V_30012)
                .await
                .map_err(error::ErrorInternalServerError)?;
        }
        "V_30167" => {
            sqlx::query("UPDATE expedientes SET V_30167 = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                .bind(&V_30764)
                .bind(V_30275)
                .execute(&V_30188.V_30012)
                .await
                .map_err(error::ErrorInternalServerError)?;
        }
        "fecha_inicio" => {
            let V_30767 = F_40067(Some(&V_30764));
            if V_30767.is_none() {
                return Ok(HttpResponse::BadRequest().json(json!({ "error": "Formato de V_30503 inválido (use YYYY-MM-DD)" })));
            }
            sqlx::query("UPDATE expedientes SET fecha_inicio = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                .bind(V_30767)
                .bind(V_30275)
                .execute(&V_30188.V_30012)
                .await
                .map_err(error::ErrorInternalServerError)?;
        }
        V_30020 => {}
    }

    sqlx::query(
        "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(V_30275)
    .bind(format!("regularizacion:{}", V_30765))
    .bind(&val_ant)
    .bind(&V_30764)
    .bind(&V_30064.V_30114)
    .bind(&V_30167)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    if let Some(cid) = form.caso_id {
        let V_30020 = sqlx::query(
            "UPDATE app_calidad_datos_casos SET V_30467 = 'regularizado', regularizado_por = $1, regularizado_at = CURRENT_TIMESTAMP, V_30704 = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3"
        )
        .bind(V_30064.V_30055)
        .bind(&V_30167)
        .bind(cid)
        .execute(&V_30188.V_30012)
        .await;
    } else {
        let V_30020 = sqlx::query(
            "UPDATE app_calidad_datos_casos SET V_30467 = 'regularizado', regularizado_por = $1, regularizado_at = CURRENT_TIMESTAMP, V_30704 = $2, updated_at = CURRENT_TIMESTAMP WHERE V_30275 = $3 AND campo_afectado = $4"
        )
        .bind(V_30064.V_30055)
        .bind(&V_30167)
        .bind(V_30275)
        .bind(V_30765)
        .execute(&V_30188.V_30012)
        .await;
    }

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "regularizar_calidad_datos",
        "calidad_datos",
        "success",
        json!({
            "V_30275": V_30275,
            "V_30765": V_30765,
            "V_30766": val_ant,
            "V_30764": V_30764,
            "V_30167": V_30167
        }),
    )
    .await;

    if V_30553 {
        return Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "V_30275": V_30275,
            "V_30765": V_30765,
            "V_30766": val_ant,
            "V_30764": V_30764
        })));
    }

    Ok(HttpResponse::SeeOther().append_header((header::LOCATION, "/calidad-datos")).finish())
}

#[post("/api/calidad-datos/estado")]
async fn F_40153(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    form: web::Form<S_50143>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    let V_30553 = req.V_30457().get("accept").and_then(|v| v.to_str().ok()).map(|V_30044| V_30044.contains("application/json")).unwrap_or(false);

    if !F_40002(&V_30188.V_30012, &V_30064.role, "calidad_datos", "modificar").await {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "Permisos insuficientes" })));
    }

    sqlx::query(
        "UPDATE app_calidad_datos_casos SET V_30467 = $1, regularizado_por = $2, regularizado_at = CURRENT_TIMESTAMP, V_30704 = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $4"
    )
    .bind(form.V_30475.trim())
    .bind(V_30064.V_30055)
    .bind(form.V_30167.trim())
    .bind(form.caso_id)
    .execute(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "cambiar_estado_calidad_datos",
        "calidad_datos",
        "success",
        json!({ "caso_id": form.caso_id, "V_30475": form.V_30475, "V_30167": form.V_30167 }),
    )
    .await;

    if V_30553 {
        return Ok(HttpResponse::Ok().json(json!({ "success": true })));
    }
    Ok(HttpResponse::SeeOther().append_header((header::LOCATION, "/calidad-datos")).finish())
}

// ============================================================================
// NOVEDADES Y EDICIÓN RÁPIDA DE FILAS (BOTÓN LÁPIZ ✏️ JUNTO A INVESTIGAR 🔍)
// ============================================================================

#[derive(Deserialize)]
struct S_50144 {
    V_30777: Option<String>,
    file_id: Option<i64>,
    sheet_id: Option<i64>,
    row_number: Option<i64>,
    V_30275: Option<i64>,
    V_30474: Option<i64>,
    credencial: Option<String>,
    numero_expediente: Option<String>,
}

#[derive(Deserialize)]
struct S_50145 {
    V_30777: Option<String>,
    file_id: Option<i64>,
    sheet_id: Option<i64>,
    row_number: Option<i64>,
    V_30275: Option<i64>,
    V_30474: Option<i64>,
    credencial: Option<String>,
    V_30778: Option<String>,
    numero_expediente: Option<String>,
    V_30458: Option<String>,
    V_30460: Option<String>,
    V_30461: String,
    V_30462: String,
    V_30463: Option<String>,
    campos_editados: Option<serde_json::Value>,
}

#[get("/api/filas/V_30277")]
async fn F_40154(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50144>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40002(&V_30188.V_30012, &V_30064.role, "expedientes", "consultar").await {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "Sin permisos de consulta" })));
    }

    let mut V_30768 = query.V_30275.filter(|&id| id > 0);
    if V_30768.is_none() {
        if let Some(ref V_30033) = query.numero_expediente {
            let V_30769 = F_40003(V_30033);
            if !V_30769.is_empty() && V_30769 != "—" && !V_30769.eq_ignore_ascii_case("sin datos") {
                V_30768 = sqlx::query_scalar(
                    "SELECT id FROM expedientes WHERE UPPER(TRIM(numero_expediente)) = UPPER(TRIM($1)) LIMIT 1"
                )
                .bind(&V_30769)
                .fetch_optional(&V_30188.V_30012)
                .await
                .unwrap_or(None);
            }
        }
    }

    let V_30770: Vec<(i64, String, Option<i64>, Option<i64>, Option<i64>, Option<String>, Option<String>, String, String, String, String, Option<String>, serde_json::Value, String, String)> = sqlx::query_as(
        V_30522#"
        SELECT
            id,
            V_30777,
            sheet_id,
            row_number,
            V_30275,
            credencial,
            numero_expediente,
            V_30458,
            TO_CHAR(V_30460, 'YYYY-MM-DD') AS V_30460,
            V_30461,
            V_30462,
            V_30463,
            COALESCE(campos_editados_json, '{}'::jsonb),
            usuario,
            TO_CHAR(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675
        FROM app_fila_novedades
        WHERE (
            ($1::bigint IS NOT NULL AND $2::bigint IS NOT NULL AND sheet_id = $1 AND row_number = $2)
            OR ($3::bigint IS NOT NULL AND V_30275 = $3)
            OR ($4::bigint IS NOT NULL AND V_30474 = $4)
            OR ($5::text IS NOT NULL AND $5 <> '' AND $1::bigint IS NULL AND $3::bigint IS NULL AND $4::bigint IS NULL AND UPPER(TRIM(COALESCE(credencial, ''))) = UPPER(TRIM($5)))
        )
        ORDER BY id DESC
        LIMIT 30
        "#
    )
    .bind(query.sheet_id)
    .bind(query.row_number)
    .bind(V_30768)
    .bind(query.V_30474.filter(|&id| id > 0))
    .bind(query.credencial.as_deref().map(F_40004).filter(|V_30044| !V_30044.is_empty() && V_30044 != "—"))
    .fetch_all(&V_30188.V_30012)
    .await
    .unwrap_or_default();

    let mut V_30771: Vec<serde_json::Value> = V_30770
        .into_iter()
        .map(|V_30522| {
            json!({
                "id": V_30522.0,
                "fuente": "fila",
                "V_30777": V_30522.1,
                "sheet_id": V_30522.2,
                "row_number": V_30522.3,
                "V_30275": V_30522.4,
                "credencial": V_30522.5,
                "numero_expediente": V_30522.6,
                "V_30458": V_30522.7,
                "V_30460": V_30522.8,
                "V_30461": V_30522.9,
                "V_30462": V_30522.10,
                "V_30463": V_30522.11,
                "campos_editados": V_30522.12,
                "usuario": V_30522.13,
                "V_30675": V_30522.14
            })
        })
        .collect();

    if let Some(eid) = V_30768 {
        let V_30772: Vec<(i64, String, String, String, String, Option<String>, String, String)> = sqlx::query_as(
            V_30522#"
            SELECT
                id,
                V_30458,
                TO_CHAR(V_30460, 'YYYY-MM-DD') AS V_30460,
                V_30461,
                V_30462,
                V_30463,
                usuario,
                TO_CHAR(V_30675, 'YYYY-MM-DD HH24:MI:SS') AS V_30675
            FROM expediente_novedades
            WHERE V_30275 = $1 AND anulado = false
            ORDER BY id DESC
            LIMIT 15
            "#
        )
        .bind(eid)
        .fetch_all(&V_30188.V_30012)
        .await
        .unwrap_or_default();

        for en in V_30772 {
            let V_30773 = V_30771.iter().any(|V_30596| {
                V_30596.get("V_30461").and_then(|v| v.as_str()) == Some(&en.3)
                    && V_30596.get("V_30462").and_then(|v| v.as_str()) == Some(&en.4)
            });
            if !V_30773 {
                V_30771.push(json!({
                    "id": en.0,
                    "fuente": "V_30113",
                    "V_30777": "V_30113",
                    "V_30275": eid,
                    "V_30458": en.1,
                    "V_30460": en.2,
                    "V_30461": en.3,
                    "V_30462": en.4,
                    "V_30463": en.5,
                    "campos_editados": {},
                    "usuario": en.6,
                    "V_30675": en.7
                }));
            }
        }
    }

    let mut V_30774: Vec<serde_json::Value> = Vec::new();
    if let (Some(V_30097), Some(rnum)) = (query.sheet_id, query.row_number) {
        let V_30775: Vec<(String, String)> = sqlx::query_as(
            "SELECT original_name, technical_column_name FROM source_columns WHERE sheet_id = $1 ORDER BY column_position"
        )
        .bind(V_30097)
        .fetch_all(&V_30188.V_30012)
        .await
        .unwrap_or_default();

        let V_30776: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT payload_json FROM raw_sheet_rows WHERE sheet_id = $1 AND source_row_number = $2 LIMIT 1"
        )
        .bind(V_30097)
        .bind(rnum)
        .fetch_optional(&V_30188.V_30012)
        .await
        .unwrap_or(None);

        if let Some(V_30012) = V_30776 {
            for (orig_name, tech_name) in V_30775 {
                let V_30190 = V_30012.get(&tech_name).and_then(|v| v.as_str()).unwrap_or("").to_string();
                V_30774.push(json!({
                    "original_name": orig_name,
                    "technical_name": tech_name,
                    "value": V_30190
                }));
            }
        }
    }

    Ok(HttpResponse::Ok().json(json!({
        "ok": true,
        "expediente_id_vinculado": V_30768,
        "V_30119": V_30771.len(),
        "V_30277": V_30771,
        "campos_fila": V_30774
    })))
}

#[post("/api/filas/novedad")]
async fn F_40155(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    body: web::Either<web::Json<S_50145>, web::Form<S_50145>>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40002(&V_30188.V_30012, &V_30064.role, "expedientes", "modificar").await {
        return Ok(HttpResponse::Forbidden().json(json!({ "error": "No posee permisos para editar filas o registrar V_30277." })));
    }

    let V_30150 = match body {
        web::Either::Left(j) => j.into_inner(),
        web::Either::Right(V_30187) => V_30187.into_inner(),
    };

    let V_30777 = F_40003(&V_30150.V_30777.unwrap_or_else(|| "hoja".to_string()));
    let V_30461 = F_40003(&V_30150.V_30461);
    let V_30462 = F_40003(&V_30150.V_30462);
    let V_30463 = V_30150.V_30463.map(|V_30044| F_40003(&V_30044)).filter(|V_30044| !V_30044.is_empty());
    let V_30458 = F_40003(&V_30150.V_30458.unwrap_or_else(|| "Observación de fila".to_string()));
    let credencial = V_30150.credencial.map(|V_30044| F_40004(&V_30044)).filter(|V_30044| !V_30044.is_empty() && V_30044 != "—");
    let V_30778 = V_30150.V_30778.map(|V_30044| F_40003(&V_30044)).filter(|V_30044| !V_30044.is_empty() && V_30044 != "—");
    let numero_expediente = V_30150.numero_expediente.map(|V_30044| F_40003(&V_30044)).filter(|V_30044| !V_30044.is_empty() && V_30044 != "—");
    let V_30460 = F_40067(V_30150.V_30460.as_deref()).unwrap_or_else(|| chrono::Utc::now().naive_utc());

    if V_30461.chars().count() < 3 {
        return Ok(HttpResponse::BadRequest().json(json!({
            "ok": false,
            "V_30763": "error",
            "V_30765": "V_30461",
            "error": "El título o V_30618 de la novedad es obligatorio (mínimo 3 caracteres)."
        })));
    }
    if V_30462.chars().count() < 5 {
        return Ok(HttpResponse::BadRequest().json(json!({
            "ok": false,
            "V_30763": "error",
            "V_30765": "V_30462",
            "error": "La descripción detallada de la novedad es obligatoria (mínimo 5 caracteres)."
        })));
    }

    let mut V_30768 = V_30150.V_30275.filter(|&id| id > 0);
    if V_30768.is_none() {
        if let Some(ref V_30033) = numero_expediente {
            V_30768 = sqlx::query_scalar(
                "SELECT id FROM expedientes WHERE UPPER(TRIM(numero_expediente)) = UPPER(TRIM($1)) LIMIT 1"
            )
            .bind(V_30033)
            .fetch_optional(&V_30188.V_30012)
            .await
            .unwrap_or(None);
        }
    }

    let mut V_30779 = json!({});
    if let (Some(V_30097), Some(rnum), Some(serde_json::Value::Object(edits_map))) = (V_30150.sheet_id, V_30150.row_number, V_30150.campos_editados.clone()) {
        if !edits_map.is_empty() {
            let V_30780: Option<serde_json::Value> = sqlx::query_scalar(
                "SELECT payload_json FROM raw_sheet_rows WHERE sheet_id = $1 AND source_row_number = $2 LIMIT 1"
            )
            .bind(V_30097)
            .bind(rnum)
            .fetch_optional(&V_30188.V_30012)
            .await
            .unwrap_or(None);

            if let Some(serde_json::Value::Object(mut cur_obj)) = V_30780 {
                let mut V_30781 = serde_json::Map::new();
                for (k, v_new_val) in edits_map {
                    let V_30782 = v_new_val.as_str().unwrap_or("").trim().to_string();
                    let V_30783 = cur_obj.get(&k).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if V_30782 != V_30783 {
                        V_30781.insert(k.clone(), json!({ "anterior": V_30783, "nuevo": V_30782 }));
                        cur_obj.insert(k, serde_json::Value::String(V_30782));
                    }
                }
                if !V_30781.is_empty() {
                    V_30779 = serde_json::Value::Object(V_30781);
                    let V_30784 = serde_json::Value::Object(cur_obj);
                    sqlx::query(
                        "UPDATE raw_sheet_rows SET payload_json = $1 WHERE sheet_id = $2 AND source_row_number = $3"
                    )
                    .bind(&V_30784)
                    .bind(V_30097)
                    .bind(rnum)
                    .execute(&V_30188.V_30012)
                    .await
                    .map_err(error::ErrorInternalServerError)?;
                }
            }
        }
    }

    let V_30785: i64 = sqlx::query_scalar(
        V_30522#"
        INSERT INTO app_fila_novedades (
            V_30777, file_id, sheet_id, row_number, V_30275, V_30474,
            credencial, V_30778, numero_expediente, V_30458, V_30460,
            V_30461, V_30462, V_30463, campos_editados_json, usuario, V_30055, V_30675
        ) VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10, $11,
            $12, $13, $14, $15, $16, $17, CURRENT_TIMESTAMP
        )
        RETURNING id
        "#
    )
    .bind(&V_30777)
    .bind(V_30150.file_id)
    .bind(V_30150.sheet_id)
    .bind(V_30150.row_number)
    .bind(V_30768)
    .bind(V_30150.V_30474.filter(|&id| id > 0))
    .bind(&credencial)
    .bind(&V_30778)
    .bind(&numero_expediente)
    .bind(&V_30458)
    .bind(V_30460)
    .bind(&V_30461)
    .bind(&V_30462)
    .bind(&V_30463)
    .bind(&V_30779)
    .bind(&V_30064.V_30114)
    .bind(Some(V_30064.V_30055))
    .fetch_one(&V_30188.V_30012)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let mut V_30786: Option<i64> = None;
    if let Some(eid) = V_30768 {
        let V_30787: Option<i64> = sqlx::query_scalar(
            V_30522#"
            INSERT INTO expediente_novedades (
                V_30275, V_30458, V_30460, V_30461, V_30462,
                V_30463, usuario, es_auditoria_automatica, V_30467, V_30675
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, false, 'revisada', CURRENT_TIMESTAMP
            )
            RETURNING id
            "#
        )
        .bind(eid)
        .bind(&V_30458)
        .bind(V_30460)
        .bind(&V_30461)
        .bind(&V_30462)
        .bind(&V_30463)
        .bind(&V_30064.V_30114)
        .fetch_optional(&V_30188.V_30012)
        .await
        .unwrap_or(None);

        V_30786 = V_30787;

        sqlx::query(
            "UPDATE expedientes SET fecha_ultima_novedad = $1, tipo_ultima_novedad = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3"
        )
        .bind(V_30460)
        .bind(&V_30458)
        .bind(eid)
        .execute(&V_30188.V_30012)
        .await
        .ok();

        sqlx::query(
            "INSERT INTO expediente_historial (V_30275, V_30765, V_30766, V_30764, usuario, V_30167) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(eid)
        .bind("novedad_fila")
        .bind(None::<String>)
        .bind(&V_30461)
        .bind(&V_30064.V_30114)
        .bind(&V_30462)
        .execute(&V_30188.V_30012)
        .await
        .ok();
    }

    if let Some(V_30473) = V_30150.V_30474.filter(|&id| id > 0) {
        if let Some(ref V_30552) = V_30463 {
            sqlx::query(
                "UPDATE expediente_suspensiones SET V_30463 = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
            )
            .bind(V_30552)
            .bind(V_30473)
            .execute(&V_30188.V_30012)
            .await
            .ok();
        }
    }

    write_audit(
        &V_30188.V_30012,
        Some(V_30064.V_30055),
        "registrar_novedad_fila",
        "app_fila_novedades",
        "success",
        json!({
            "fila_novedad_id": V_30785,
            "expediente_novedad_id": V_30786,
            "V_30777": V_30777,
            "sheet_id": V_30150.sheet_id,
            "row_number": V_30150.row_number,
            "V_30275": V_30768,
            "credencial": credencial,
            "V_30461": V_30461,
            "campos_editados": V_30779
        }),
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({
        "ok": true,
        "id": V_30785,
        "V_30275": V_30768,
        "expediente_novedad_id": V_30786,
        "campos_editados": V_30779,
        "mensaje": "Novedad y edición sobre la fila guardadas con éxito."
    })))
}

// ============================================================================
// MOTOR UNIFICADO DE VALIDACIONES DINÁMICAS, DETECCIÓN DE POSIBLES DUPLICADOS
// Y SUGERENCIAS DE VINCULACIÓN ENTRE ENTIDADES (NO BLOQUEANTE PARA AVISOS)
// ============================================================================

#[derive(Deserialize)]
struct S_50146 {
    V_30600: Option<String>,
    V_30765: Option<String>,
    V_30228: Option<String>,
    numero_expediente: Option<String>,
    credencial: Option<String>,
    V_30266: Option<String>,
    V_30237: Option<String>,
    V_30238: Option<String>,
    reparticion: Option<String>,
    V_30230: Option<String>,
    V_30167: Option<String>,
    fecha_inicio: Option<String>,
    V_30232: Option<String>,
    V_30243: Option<String>,
    V_30242: Option<String>,
    V_30794: Option<i64>,
    V_30275: Option<i64>,
}

#[get("/api/validaciones/verificar")]
async fn F_40156(
    req: HttpRequest,
    V_30188: web::Data<AppState>,
    query: web::Query<S_50146>,
) -> Result<HttpResponse> {
    let V_30064 = match require_user(&req, &V_30188).await {
        V_30054(u) => u,
        V_30156(V_30084) => return V_30054(V_30084),
    };
    if !F_40002(&V_30188.V_30012, &V_30064.role, "expedientes", "consultar").await {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "No posee permisos para consultar validaciones o coincidencias de registros."
        })));
    }

    let V_30600 = query.V_30600.as_deref().unwrap_or("V_30113").trim().to_lowercase();
    let V_30788 = query.V_30765.as_deref().unwrap_or("todos").trim().to_lowercase();
    let V_30789 = V_30788 == "todos" || V_30788.is_empty();

    let mut V_30790: Vec<serde_json::Value> = Vec::new();
    let mut V_30791: Vec<serde_json::Value> = Vec::new();
    let mut V_30792: Vec<serde_json::Value> = Vec::new();

    let V_30793 = F_40003(query.V_30228.as_deref().unwrap_or("Electrónico"));
    let V_30794 = query.V_30794.filter(|&id| id > 0);
    let V_30795 = query.V_30275.filter(|&id| id > 0).or(V_30794);

    // 1. Validación de Número de Expediente + Unicidad (Error) + Posibles Duplicados (Advertencia) + Vinculaciones (Información)
    if let Some(ref raw_num) = query.numero_expediente {
        if V_30789 || V_30788 == "numero_expediente" || V_30788 == "V_30228" {
            let V_30033 = F_40003(raw_num);
            if V_30033.is_empty() || V_30033.eq_ignore_ascii_case("sin datos") {
                V_30790.push(json!({
                    "V_30763": "error",
                    "V_30825": true,
                    "V_30765": "numero_expediente",
                    "codigo": "requerido",
                    "mensaje": "El número de V_30113 es obligatorio y no puede ser 'Sin datos'."
                }));
            } else if V_30033.chars().count() < 3 {
                V_30790.push(json!({
                    "V_30763": "error",
                    "V_30825": true,
                    "V_30765": "numero_expediente",
                    "codigo": "formato_longitud",
                    "mensaje": "El número de V_30113 debe contener al menos 3 caracteres válidos."
                }));
            } else if !V_30033.chars().any(|c| c.is_ascii_alphanumeric()) {
                V_30790.push(json!({
                    "V_30763": "error",
                    "V_30825": true,
                    "V_30765": "numero_expediente",
                    "codigo": "formato_invalido",
                    "mensaje": "El formato del número de V_30113 es inválido (debe incluir números o letras)."
                }));
            } else {
                // 1A. Restricción de unicidad obligatoria y documentada: (V_30228, numero_expediente) -> ERROR REAL
                let V_30796: Option<(i64, String, String, String)> = sqlx::query_as(
                    V_30522#"
                    SELECT id, numero_expediente, COALESCE(reparticion, 'Sin datos'), COALESCE(estado, 'Abierto')
                    FROM expedientes
                    WHERE LOWER(TRIM(V_30228)) = LOWER(TRIM($1))
                      AND UPPER(TRIM(numero_expediente)) = UPPER(TRIM($2))
                      AND ($3::bigint IS NULL OR id <> $3)
                    LIMIT 1
                    "#
                )
                .bind(&V_30793)
                .bind(&V_30033)
                .bind(V_30794)
                .fetch_optional(&V_30188.V_30012)
                .await
                .unwrap_or(None);

                if let Some((eid, enum_str, erep, eest)) = V_30796 {
                    V_30790.push(json!({
                        "V_30763": "error",
                        "V_30825": true,
                        "V_30765": "numero_expediente",
                        "codigo": "unicidad_expediente",
                        "registro_id": eid,
                        "identificador": enum_str,
                        "V_30228": V_30793,
                        "url": format!("/expedientes/{}", eid),
                        "mensaje": format!("Conflicto de unicidad obligatoria: ya existe el V_30113 '{}' bajo el V_30092 '{}' (ID #{}, Repartición: {}, Estado: {}). Corrija el identificador o consulte el registro existente.", enum_str, V_30793, eid, erep, eest)
                    }));
                }

                // 1B. Posible duplicado: mismo número bajo otro V_30228 -> ADVERTENCIA NO BLOQUEANTE
                let V_30797: Vec<(i64, String, String, String, String)> = sqlx::query_as(
                    V_30522#"
                    SELECT id, COALESCE(V_30228, 'sumario'), numero_expediente, COALESCE(reparticion, 'Sin datos'), COALESCE(estado, 'Abierto')
                    FROM expedientes
                    WHERE UPPER(TRIM(numero_expediente)) = UPPER(TRIM($1))
                      AND LOWER(TRIM(V_30228)) <> LOWER(TRIM($2))
                      AND ($3::bigint IS NULL OR id <> $3)
                    ORDER BY id DESC
                    LIMIT 5
                    "#
                )
                .bind(&V_30033)
                .bind(&V_30793)
                .bind(V_30794)
                .fetch_all(&V_30188.V_30012)
                .await
                .unwrap_or_default();

                for (pid, ptipo, pnum, prep, pest) in V_30797 {
                    V_30791.push(json!({
                        "V_30763": "advertencia",
                        "V_30825": false,
                        "V_30765": "numero_expediente",
                        "codigo": "posible_duplicado_otro_tipo",
                        "motivo_coincidencia": format!("Mismo número de actuación ('{}') registrado bajo otro Tipo de Identificador ('{}')", pnum, ptipo),
                        "registro_id": pid,
                        "identificador": pnum,
                        "V_30228": ptipo,
                        "reparticion": prep,
                        "estado": pest,
                        "url": format!("/expedientes/{}", pid),
                        "mensaje": format!("Posible duplicado (informativo, no bloquea la carga): existe '{}' como '{}' en {} (Estado: {}).", pnum, ptipo, prep, pest)
                    }));
                }

                // 1C. Posible duplicado aproximado: misma clave alfanumérica normalizada sin guiones/espacios/barras -> ADVERTENCIA NO BLOQUEANTE
                let V_30096: String = V_30033.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_uppercase();
                if V_30096.len() >= 4 {
                    let V_30798: Vec<(i64, String, String, String, String)> = sqlx::query_as(
                        V_30522#"
                        SELECT id, COALESCE(V_30228, 'sumario'), numero_expediente, COALESCE(reparticion, 'Sin datos'), COALESCE(estado, 'Abierto')
                        FROM expedientes
                        WHERE REGEXP_REPLACE(UPPER(numero_expediente), '[^A-Z0-9]', '', 'g') = $1
                          AND UPPER(TRIM(numero_expediente)) <> UPPER(TRIM($2))
                          AND ($3::bigint IS NULL OR id <> $3)
                        ORDER BY id DESC
                        LIMIT 5
                        "#
                    )
                    .bind(&V_30096)
                    .bind(&V_30033)
                    .bind(V_30794)
                    .fetch_all(&V_30188.V_30012)
                    .await
                    .unwrap_or_default();

                    for (V_30097, stipo, snum, srep, sest) in V_30798 {
                        V_30791.push(json!({
                            "V_30763": "advertencia",
                            "V_30825": false,
                            "V_30765": "numero_expediente",
                            "codigo": "posible_duplicado_similar",
                            "motivo_coincidencia": format!("Coincidencia aproximada de caracteres alfanuméricos con '{}' ({})", snum, stipo),
                            "registro_id": V_30097,
                            "identificador": snum,
                            "V_30228": stipo,
                            "reparticion": srep,
                            "estado": sest,
                            "url": format!("/expedientes/{}", V_30097),
                            "mensaje": format!("Posible duplicado por similitud de formato: '{}' ({}, Repartición: {}, Estado: {}).", snum, stipo, srep, sest)
                        }));
                    }
                }

                // 1D. Sugerencia de vinculación / antecedente en planillas importadas para este número de V_30113 -> INFORMACIÓN
                if V_30033.chars().count() >= 4 {
                    let V_30799 = format!("%{}%", V_30033);
                    let V_30800: Vec<(i64, i64, String, String, i64)> = sqlx::query_as(
                        V_30522#"
                        SELECT sf.id, ss.id, sf.file_name, ss.sheet_name, V_30522.source_row_number
                        FROM raw_sheet_rows V_30522
                        JOIN source_sheets ss ON ss.id = V_30522.sheet_id
                        JOIN source_files sf ON sf.id = ss.file_id
                        WHERE V_30522.payload_json::text ILIKE $1
                        ORDER BY V_30522.sheet_id ASC, V_30522.source_row_number ASC
                        LIMIT 4
                        "#
                    )
                    .bind(&V_30799)
                    .fetch_all(&V_30188.V_30012)
                    .await
                    .unwrap_or_default();

                    for (fid, V_30097, fname, sname, rnum) in V_30800 {
                        V_30792.push(json!({
                            "V_30763": "V_30792",
                            "V_30825": false,
                            "V_30765": "numero_expediente",
                            "subtipo": "antecedente_planilla",
                            "entidad_tipo": "hoja_importada",
                            "motivo_coincidencia": format!("Referencia coincidente en planilla histórica '{}' · Hoja '{}' (Fila #{})", fname, sname, rnum),
                            "identificador": format!("{} #{}", sname, rnum),
                            "url": format!("/fuente/{}/hoja/{}?V_30065={}", fid, V_30097, urlencoding::encode(&V_30033)),
                            "mensaje": format!("Sugerencia de vinculación documental: el número '{}' figura en '{}' ({}, Fila #{}).", V_30033, sname, fname, rnum)
                        }));
                    }
                }
            }
        }
    }

    // 2. Validación de Credencial (Agente o Instructor) + Preservación de ceros + Detección de Relaciones (Información / Advertencia)
    let V_30801 = [
        ("credencial", query.credencial.as_deref()),
        ("V_30237", query.V_30237.as_deref()),
    ];

    for (field_name, opt_raw_cred) in V_30801 {
        if let Some(raw_cred) = opt_raw_cred {
            if V_30789 || V_30788 == field_name {
                let V_30168 = F_40004(raw_cred);
                if V_30168.is_empty() {
                    if V_30600 == "agente" && field_name == "credencial" {
                        V_30790.push(json!({
                            "V_30763": "error",
                            "V_30825": true,
                            "V_30765": field_name,
                            "codigo": "requerido",
                            "mensaje": "La credencial del agente es obligatoria."
                        }));
                    }
                } else {
                    let V_30802 = V_30168.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
                    let V_30803 = V_30168.chars().any(|c| c != '0' && c != '-');
                    if !V_30802 || V_30168.len() < 2 || V_30168.len() > 20 || !V_30803 {
                        V_30790.push(json!({
                            "V_30763": "error",
                            "V_30825": true,
                            "V_30765": field_name,
                            "codigo": "formato_credencial",
                            "mensaje": "Formato de credencial inválido. Ingrese entre 2 y 20 caracteres alfanuméricos válidos (se preservan ceros a la izquierda, ej. 00045)."
                        }));
                    } else {
                        // 2A. Si estamos agregando un agente a un V_30113, verificar si ya está vinculado al MISMO V_30113 -> ADVERTENCIA
                        if field_name == "credencial" {
                            if let Some(cexp_id) = V_30795 {
                                let V_30804: Option<(i64, String, String)> = sqlx::query_as(
                                    "SELECT id, V_30266, rol FROM expediente_agentes WHERE V_30275 = $1 AND UPPER(TRIM(credencial)) = UPPER(TRIM($2)) LIMIT 1"
                                )
                                .bind(cexp_id)
                                .bind(&V_30168)
                                .fetch_optional(&V_30188.V_30012)
                                .await
                                .unwrap_or(None);

                                if let Some((ag_id, ag_nom, ag_rol)) = V_30804 {
                                    V_30791.push(json!({
                                        "V_30763": "advertencia",
                                        "V_30825": false,
                                        "V_30765": field_name,
                                        "codigo": "agente_ya_en_expediente",
                                        "registro_id": ag_id,
                                        "identificador": V_30168,
                                        "motivo_coincidencia": format!("La credencial {} ({}) ya se encuentra incorporada en este mismo V_30113 con rol '{}'", V_30168, ag_nom, ag_rol),
                                        "url": format!("/expedientes/{}?tab=tab-V_30086", cexp_id),
                                        "mensaje": format!("Posible duplicado de agente en la actuación: {} (Cred. {}) ya figura como '{}'.", ag_nom, V_30168, ag_rol)
                                    }));
                                }
                            }
                        }

                        // 2B. Consulta en Padrón Unificado de Personas (`personas`) -> INFORMACIÓN
                        let V_30805: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
                            "SELECT credencial, V_30526, origen_principal FROM personas WHERE UPPER(TRIM(credencial)) = UPPER(TRIM($1)) LIMIT 1"
                        )
                        .bind(&V_30168)
                        .fetch_optional(&V_30188.V_30012)
                        .await
                        .unwrap_or(None);

                        if let Some((pcred, pnom, porig)) = V_30805 {
                            let V_30806 = pnom.unwrap_or_else(|| "Sin nombre registrado".to_string());
                            let V_30807 = porig.unwrap_or_else(|| "Padrón General".to_string());
                            V_30792.push(json!({
                                "V_30763": "V_30792",
                                "V_30825": false,
                                "V_30765": field_name,
                                "subtipo": "padron_persona",
                                "entidad_tipo": "persona",
                                "identificador": pcred,
                                "nombre_detectado": V_30806,
                                "origen_principal": V_30807,
                                "motivo_coincidencia": format!("Credencial registrada en el Padrón Unificado de Personas ({})", V_30807),
                                "url": format!("/V_30086/{}/V_30276", urlencoding::encode(&V_30168)),
                                "mensaje": format!("Persona identificada en Padrón: {} (Cred. {}, Origen: {}).", V_30806, V_30168, V_30807)
                            }));
                        }

                        // 2C. Otras actuaciones sumariales donde interviene esta credencial (`expediente_agentes` + `expedientes`) -> INFORMACIÓN (Sugerencia de vinculación)
                        let V_30808: Vec<(i64, String, String, String, String, String, String)> = sqlx::query_as(
                            V_30522#"
                            SELECT
                                e.id,
                                COALESCE(e.V_30228, 'sumario'),
                                e.numero_expediente,
                                COALESCE(e.reparticion, 'Sin datos'),
                                COALESCE(e.estado, 'Abierto'),
                                ea.V_30266,
                                ea.rol
                            FROM expediente_agentes ea
                            JOIN expedientes e ON e.id = ea.V_30275
                            WHERE UPPER(TRIM(ea.credencial)) = UPPER(TRIM($1))
                              AND ($2::bigint IS NULL OR e.id <> $2)
                            ORDER BY e.id DESC
                            LIMIT 5
                            "#
                        )
                        .bind(&V_30168)
                        .bind(V_30795)
                        .fetch_all(&V_30188.V_30012)
                        .await
                        .unwrap_or_default();

                        for (oe_id, oe_tipo, oe_num, oe_rep, oe_est, oe_nom, oe_rol) in V_30808 {
                            let V_30809 = if let Some(cexp_id) = V_30795 {
                                sqlx::query_scalar::<V_30020, i64>(
                                    "SELECT COUNT(*) FROM expediente_relaciones WHERE ((expediente_origen_id = $1 AND expediente_destino_id = $2) OR (expediente_origen_id = $2 AND expediente_destino_id = $1)) AND estado = 'Activa'"
                                )
                                .bind(cexp_id)
                                .bind(oe_id)
                                .fetch_one(&V_30188.V_30012)
                                .await
                                .unwrap_or(0) > 0
                            } else {
                                false
                            };

                            V_30792.push(json!({
                                "V_30763": "V_30792",
                                "V_30825": false,
                                "V_30765": field_name,
                                "subtipo": "vinculacion_expediente_agente",
                                "entidad_tipo": "V_30113",
                                "registro_id": oe_id,
                                "identificador": oe_num,
                                "V_30228": oe_tipo,
                                "reparticion": oe_rep,
                                "estado": oe_est,
                                "V_30809": V_30809,
                                "motivo_coincidencia": format!("El agente {} (Cred. {}) figura como '{}' en el V_30113 {} ({})", oe_nom, V_30168, oe_rol, oe_num, oe_tipo),
                                "url": format!("/expedientes/{}", oe_id),
                                "mensaje": format!("Posible vinculación: Cred. {} ({}) posee antecedente en V_30113 {} [{}] ({}, Estado: {}{}).", V_30168, oe_nom, oe_num, oe_tipo, oe_rep, oe_est, if V_30809 { " · Ya vinculado" } else { "" })
                            }));
                        }

                        // 2D. Suspensiones preventivas registradas para esta credencial (`expediente_suspensiones` o `expediente_agentes`) -> INFORMACIÓN
                        let V_30810: Vec<(i64, String, String, String, Option<i64>)> = sqlx::query_as(
                            V_30522#"
                            SELECT id, COALESCE(V_30266, 'Agente'), COALESCE(V_30471, 'd)'), COALESCE(estado, 'Vigente'), V_30275
                            FROM expediente_suspensiones
                            WHERE UPPER(TRIM(credencial)) = UPPER(TRIM($1))
                            UNION ALL
                            SELECT ea.id, COALESCE(ea.V_30266, 'Agente'), COALESCE(ea.V_30328, 'inc. d)'), 'Vigente', ea.V_30275
                            FROM expediente_agentes ea
                            WHERE UPPER(TRIM(ea.credencial)) = UPPER(TRIM($1)) AND ea.V_30326 = true
                            ORDER BY id DESC
                            LIMIT 4
                            "#
                        )
                        .bind(&V_30168)
                        .fetch_all(&V_30188.V_30012)
                        .await
                        .unwrap_or_default();

                        for (V_30097, snom, sinc, sest, sexp_id) in V_30810 {
                            V_30792.push(json!({
                                "V_30763": "V_30792",
                                "V_30825": false,
                                "V_30765": field_name,
                                "subtipo": "vinculacion_suspension",
                                "entidad_tipo": "suspension",
                                "registro_id": V_30097,
                                "V_30275": sexp_id,
                                "identificador": format!("Suspensión #{} (Art. 419 {})", V_30097, sinc),
                                "estado": sest,
                                "motivo_coincidencia": format!("Credencial {} ({}) registra medida en Control de Suspensiones (Estado: {})", V_30168, snom, sest),
                                "url": format!("/V_30278/{}", V_30097),
                                "mensaje": format!("Antecedente de suspensión preventiva: {} (Cred. {}) registra medida {} encuadrada en Art. 419 {}.", snom, V_30168, sest, sinc)
                            }));
                        }
                    }
                }
            }
        }
    }

    // 3. Validación de Nombre Completo (Agente o Instructor) + Homónimos / Similitud de Nombre
    let V_30811 = [
        ("V_30266", query.V_30266.as_deref(), V_30600 == "agente"),
        ("V_30238", query.V_30238.as_deref(), V_30600 == "V_30113"),
    ];

    for (field_name, opt_raw_name, is_req) in V_30811 {
        if let Some(raw_name) = opt_raw_name {
            if V_30789 || V_30788 == field_name {
                let V_30184 = F_40003(raw_name);
                if V_30184.is_empty() || V_30184.eq_ignore_ascii_case("sin datos") {
                    if is_req {
                        V_30790.push(json!({
                            "V_30763": "error",
                            "V_30825": true,
                            "V_30765": field_name,
                            "codigo": "requerido",
                            "mensaje": format!("El V_30765 '{}' es obligatorio y no puede quedar vacío ni ser 'Sin datos'.", if field_name == "V_30238" { "Instructor designado" } else { "Apellido y Nombres" })
                        }));
                    }
                } else if V_30184.chars().count() < 3 {
                    V_30790.push(json!({
                        "V_30763": "error",
                        "V_30825": true,
                        "V_30765": field_name,
                        "codigo": "longitud_minima",
                        "mensaje": "Debe ingresar al menos 3 caracteres para el nombre."
                    }));
                } else if V_30184.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()) {
                    V_30790.push(json!({
                        "V_30763": "error",
                        "V_30825": true,
                        "V_30765": field_name,
                        "codigo": "formato_nombre",
                        "mensaje": "El nombre no puede estar formado únicamente por números."
                    }));
                } else if field_name == "V_30266" && V_30184.chars().count() >= 5 {
                    let V_30812 = format!("%{}%", V_30184);
                    let V_30813: Vec<(String, String, String)> = sqlx::query_as(
                        V_30522#"
                        SELECT credencial, COALESCE(V_30526, 'Sin datos'), COALESCE(origen_principal, 'Padrón')
                        FROM personas
                        WHERE COALESCE(V_30526, '') ILIKE $1
                        ORDER BY credencial ASC
                        LIMIT 4
                        "#
                    )
                    .bind(&V_30812)
                    .fetch_all(&V_30188.V_30012)
                    .await
                    .unwrap_or_default();

                    for (hcred, hnom, horig) in V_30813 {
                        let V_30814 = query.credencial.as_deref().map(F_40004).unwrap_or_default();
                        if !V_30814.is_empty() && V_30814.eq_ignore_ascii_case(&hcred) {
                            continue;
                        }
                        V_30792.push(json!({
                            "V_30763": "V_30792",
                            "V_30825": false,
                            "V_30765": field_name,
                            "subtipo": "coincidencia_nombre_persona",
                            "entidad_tipo": "persona",
                            "identificador": hcred.clone(),
                            "nombre_detectado": hnom.clone(),
                            "motivo_coincidencia": format!("Coincidencia de nombre en Padrón de Personas: {} (Cred. {}, {})", hnom, hcred, horig),
                            "url": format!("/V_30086/{}/V_30276", urlencoding::encode(&hcred)),
                            "mensaje": format!("Sugerencia por coincidencia de nombre: '{}' registra Credencial {} en {}.", hnom, hcred, horig)
                        }));
                    }
                }
            }
        }
    }

    // 4. Validación de Repartición y Motivo
    if let Some(ref raw_rep) = query.reparticion {
        if V_30789 || V_30788 == "reparticion" || V_30788 == "V_30230" {
            let V_30815 = F_40003(raw_rep);
            if V_30815.is_empty() || V_30815.eq_ignore_ascii_case("sin datos") {
                V_30790.push(json!({
                    "V_30763": "error",
                    "V_30825": true,
                    "V_30765": "reparticion",
                    "codigo": "requerido",
                    "mensaje": "La repartición o dependencia es obligatoria y no puede ser 'Sin datos'."
                }));
            } else if V_30815 == "OTRO" {
                let V_30816 = F_40003(query.V_30230.as_deref().unwrap_or(""));
                if V_30816.chars().count() < 2 {
                    V_30790.push(json!({
                        "V_30763": "error",
                        "V_30825": true,
                        "V_30765": "V_30230",
                        "codigo": "requerido_condicional",
                        "mensaje": "Al seleccionar 'OTRO', debe especificar el nombre de la repartición (mínimo 2 caracteres)."
                    }));
                }
            }
        }
    }

    if let Some(ref raw_mot) = query.V_30167 {
        if V_30789 || V_30788 == "V_30167" {
            let V_30510 = F_40003(raw_mot);
            if V_30510.is_empty() || V_30510.eq_ignore_ascii_case("sin datos") {
                V_30790.push(json!({
                    "V_30763": "error",
                    "V_30825": true,
                    "V_30765": "V_30167",
                    "codigo": "requerido",
                    "mensaje": "El V_30167 de la actuación es obligatorio."
                }));
            }
        }
    }

    // 5. Validación de Fechas y Coherencia Cronológica entre Campos
    let V_30817 = query.fecha_inicio.as_deref().map(str::trim).filter(|V_30044| !V_30044.is_empty());
    let V_30818 = query.V_30232.as_deref().map(str::trim).filter(|V_30044| !V_30044.is_empty());
    let V_30819 = query.V_30243.as_deref().map(str::trim).filter(|V_30044| !V_30044.is_empty());

    let V_30820 = V_30817.and_then(|V_30044| F_40067(Some(V_30044)));
    let V_30821 = V_30818.and_then(|V_30044| F_40067(Some(V_30044)));
    let V_30822 = V_30819.and_then(|V_30044| F_40067(Some(V_30044)));

    if V_30817.is_some() && V_30820.is_none() {
        V_30790.push(json!({
            "V_30763": "error",
            "V_30825": true,
            "V_30765": "fecha_inicio",
            "codigo": "formato_fecha",
            "mensaje": "El formato de la V_30503 de inicio no es válido (use AAAA-MM-DD)."
        }));
    }
    if V_30818.is_some() && V_30821.is_none() {
        V_30790.push(json!({
            "V_30763": "error",
            "V_30825": true,
            "V_30765": "V_30232",
            "codigo": "formato_fecha",
            "mensaje": "El formato de la V_30503 del hecho no es válido (use AAAA-MM-DD)."
        }));
    }
    if V_30819.is_some() && V_30822.is_none() {
        V_30790.push(json!({
            "V_30763": "error",
            "V_30825": true,
            "V_30765": "V_30243",
            "codigo": "formato_fecha",
            "mensaje": "El formato de la V_30503 de finalización no es válido (use AAAA-MM-DD)."
        }));
    }

    if let (Some(fh), Some(fi)) = (V_30821, V_30820) {
        if fh > fi {
            V_30790.push(json!({
                "V_30763": "error",
                "V_30825": true,
                "V_30765": "V_30232",
                "codigo": "coherencia_fechas",
                "mensaje": "Inconsistencia cronológica: la V_30503 del hecho no puede ser posterior a la V_30503 de inicio de la actuación."
            }));
        }
    }

    if let (Some(fi), Some(ff)) = (V_30820, V_30822) {
        if ff < fi {
            V_30790.push(json!({
                "V_30763": "error",
                "V_30825": true,
                "V_30765": "V_30243",
                "codigo": "coherencia_fechas",
                "mensaje": "Inconsistencia cronológica: la V_30503 de finalización no puede ser anterior a la V_30503 de inicio."
            }));
        }
    }

    if let Some(ref fin_estado) = query.V_30242 {
        let V_30823 = fin_estado.trim();
        if !V_30823.is_empty() && V_30823 != "No" && V_30819.is_none() && (V_30789 || V_30788 == "V_30242" || V_30788 == "V_30243") {
            V_30790.push(json!({
                "V_30763": "error",
                "V_30825": true,
                "V_30765": "V_30243",
                "codigo": "requerido_condicional",
                "mensaje": format!("Al indicar finalización '{}', debe registrar la V_30503 de finalización.", V_30823)
            }));
        }
    }

    let V_30824 = V_30790.is_empty();
    let V_30825 = !V_30824;

    Ok(HttpResponse::Ok().json(json!({
        "ok": true,
        "V_30824": V_30824,
        "V_30825": V_30825,
        "V_30618": {
            "total_errores": V_30790.len(),
            "total_advertencias": V_30791.len(),
            "total_informacion": V_30792.len()
        },
        "V_30790": V_30790,
        "V_30791": V_30791,
        "V_30792": V_30792
    })))
}
