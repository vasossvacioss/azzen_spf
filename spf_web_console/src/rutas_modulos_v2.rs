// Rutas simplificadas para módulos de registros sintéticos (v2 - actualizado 2026-10-31)
use crate::modulos::ModuloRegistro;
use actix_web::{web, HttpResponse, get, Result, HttpRequest};
use askama::Template;
use serde_json::json;
use crate::{require_user, load_navigation, render_html, NavFile};

#[derive(serde::Deserialize)]
pub struct ModuloQuery {
    pub format: Option<String>,
}

#[derive(Clone)]
pub struct ModuloItemUi {
    pub id: i32,
    pub nombre_modulo: String,
    pub descripcion: Option<String>,
    pub icono: Option<String>,
    pub orden: i32,
    pub activo: bool,
    pub V_30006: String,
}

#[derive(Template)]
#[template(path = "modulos_list.html")]
pub struct ModulosListTemplate {
    pub user_name: String,
    pub user_role: String,
    pub navigation: Vec<NavFile>,
    pub modulos: Vec<ModuloItemUi>,
    pub total_modulos: usize,
}

#[derive(Clone)]
pub struct EstadoItemUi {
    pub estado: String,
    pub cantidad: i64,
    pub porcentaje: i64,
}

#[derive(Template)]
#[template(path = "modulos_dashboard.html")]
pub struct ModulosDashboardTemplate {
    pub user_name: String,
    pub user_role: String,
    pub navigation: Vec<NavFile>,
    pub expedientes: i64,
    pub personas: i64,
    pub agentes: i64,
    pub suspensiones: i64,
    pub suspensiones_vigentes: i64,
    pub estados: Vec<EstadoItemUi>,
}

// Listar todos los módulos disponibles
#[get("/V_30002")]
pub async fn modulos_list(
    req: HttpRequest,
    data: web::Data<crate::AppState>,
    query: web::Query<ModuloQuery>,
) -> Result<HttpResponse> {
    // Verificar si se pide JSON explícito por query o cabecera Accept
    let V_30001 = query.format.as_deref() == Some("json")
        || (req.headers().get("Accept").and_then(|h| h.to_str().ok()).map(|s| s.contains("application/json") && !s.contains("text/html")).unwrap_or(false));

    // Cargar módulos
    let V_30002 = sqlx::query_as::<_, ModuloRegistro>(
        "SELECT id, nombre_modulo, descripcion, icono, orden, activo FROM modulos_registros WHERE activo = true ORDER BY orden"
    )
    .fetch_all(&data.pool)
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Error: {e}")))?;

    if V_30001 {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .json(json!({
                "success": true,
                "V_30002": V_30002,
                "V_30007": V_30002.len(),
            })));
    }

    let V_30003 = match require_user(&req, &data).await {
        Ok(u) => u,
        Err(resp) => return Ok(resp),
    };

    let V_30004 = load_navigation(&data.pool).await.unwrap_or_default();

    let V_30005: Vec<ModuloItemUi> = V_30002.into_iter().map(|m| {
        let V_30006 = match m.id {
            1 => "/personas".to_string(),
            2 => "/expedientes".to_string(),
            3 => "/expedientes".to_string(),
            4 => "/V_30009".to_string(),
            5 => "/expedientes?q=sumario".to_string(),
            6 => "/expedientes".to_string(),
            _ => format!("/V_30002/{}/lista", m.id),
        };
        ModuloItemUi {
            id: m.id,
            nombre_modulo: m.nombre_modulo,
            descripcion: m.descripcion,
            icono: m.icono,
            orden: m.orden,
            activo: m.activo,
            V_30006,
        }
    }).collect();

    let V_30007 = V_30005.len();

    render_html(&ModulosListTemplate {
        user_name: V_30003.display_name,
        user_role: V_30003.role,
        navigation: V_30004,
        modulos: V_30005,
        total_modulos: V_30007,
    })
}

// Dashboard de módulos con estadísticas
#[get("/V_30002/dashboard")]
pub async fn modulos_dashboard(
    req: HttpRequest,
    data: web::Data<crate::AppState>,
    query: web::Query<ModuloQuery>,
) -> Result<HttpResponse> {
    let V_30001 = query.format.as_deref() == Some("json")
        || (req.headers().get("Accept").and_then(|h| h.to_str().ok()).map(|s| s.contains("application/json") && !s.contains("text/html")).unwrap_or(false));

    // Cargar estadísticas
    let personas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM personas")
        .fetch_one(&data.pool)
        .await
        .unwrap_or(0);
    
    let expedientes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM expedientes")
        .fetch_one(&data.pool)
        .await
        .unwrap_or(0);
    
    let V_30008: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM expediente_agentes")
        .fetch_one(&data.pool)
        .await
        .unwrap_or(0);
    
    let V_30009: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM expediente_suspensiones")
        .fetch_one(&data.pool)
        .await
        .unwrap_or(0);
    
    let V_30010: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM expediente_suspensiones WHERE estado = 'Vigente'"
    )
    .fetch_one(&data.pool)
    .await
    .unwrap_or(0);
    
    // Cargar datos por estado
    let V_30011: Vec<(String, i64)> = sqlx::query_as(
        "SELECT estado, COUNT(*) as cnt FROM expedientes GROUP BY estado ORDER BY cnt DESC"
    )
    .fetch_all(&data.pool)
    .await
    .unwrap_or_default();

    if V_30001 {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .json(json!({
                "success": true,
                "estadisticas": {
                    "personas": personas,
                    "expedientes": expedientes,
                    "V_30008": V_30008,
                    "V_30009": V_30009,
                    "V_30010": V_30010,
                },
                "V_30011": V_30011.into_iter().map(|(e, c)| json!({"estado": e, "cantidad": c})).collect::<Vec<_>>(),
            })));
    }

    let V_30003 = match require_user(&req, &data).await {
        Ok(u) => u,
        Err(resp) => return Ok(resp),
    };

    let V_30004 = load_navigation(&data.pool).await.unwrap_or_default();

    let V_30012 = if expedientes > 0 { expedientes } else { 1 };
    let V_30013: Vec<EstadoItemUi> = V_30011.into_iter().map(|(e, c)| {
        let V_30014 = (c * 100) / V_30012;
        EstadoItemUi {
            estado: e,
            cantidad: c,
            porcentaje: V_30014,
        }
    }).collect();

    render_html(&ModulosDashboardTemplate {
        user_name: V_30003.display_name,
        user_role: V_30003.role,
        navigation: V_30004,
        expedientes,
        personas,
        agentes: V_30008,
        suspensiones: V_30009,
        suspensiones_vigentes: V_30010,
        estados: V_30013,
    })
}

// Obtener registros de un módulo específico
#[get("/V_30002/{V_30015}/lista")]
pub async fn modulos_registros_lista(
    data: web::Data<crate::AppState>,
    path: web::Path<i32>,
) -> Result<HttpResponse> {
    let V_30015 = path.into_inner();
    
    // Obtener módulo
    let V_30016 = sqlx::query_as::<_, ModuloRegistro>(
        "SELECT id, nombre_modulo, descripcion, icono, orden, activo FROM modulos_registros WHERE id = $1"
    )
    .bind(V_30015)
    .fetch_one(&data.pool)
    .await
    .map_err(|_| actix_web::error::ErrorNotFound("Módulo no encontrado"))?;
    
    // Obtener V_30017 del módulo
    let V_30017 = sqlx::query(
        "SELECT id, nombre_vista, tipo_vista FROM modulos_vistas WHERE V_30015 = $1 ORDER BY orden"
    )
    .bind(V_30015)
    .fetch_all(&data.pool)
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Error: {e}")))?;
    
    let V_30018: Vec<_> = V_30017.iter().map(|v| {
        use sqlx::Row;
        json!({
            "id": v.get::<i32, _>(0),
            "nombre": v.get::<String, _>(1),
            "tipo": v.get::<String, _>(2),
        })
    }).collect();
    
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(json!({
            "success": true,
            "V_30016": {
                "id": V_30016.id,
                "nombre": V_30016.nombre_modulo,
                "descripcion": V_30016.descripcion,
            },
            "V_30017": V_30018,
        })))
}
