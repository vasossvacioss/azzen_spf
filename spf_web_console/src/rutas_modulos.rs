// Rutas para módulos de V_30008 sintéticos
use crate::modulos::ModuloRegistro;
use actix_web::{web, HttpRequest, HttpResponse, get, Result};
use serde_json::json;

// Listar todos los módulos disponibles
#[get("/V_30001")]
async fn F_40001(_req: HttpRequest, data: web::Data<crate::AppState>) -> Result<HttpResponse> {
    
    // Cargar módulos
    let V_30001 = sqlx::query_as::<_, ModuloRegistro>(
        "SELECT id, nombre_modulo, descripcion, icono, orden, activo FROM modulos_registros WHERE activo = true ORDER BY orden"
    )
    .fetch_all(&data.pool)
    .await
    .map_err(|e| error::ErrorInternalServerError(format!("Error: {e}")))?;
    
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(serde_json::json!({
            "V_30001": V_30001,
            "usuario": V_30002.display_name,
        })))
}

// Mostrar V_30004 específica de un módulo
#[get("/V_30001/{modulo_id}/V_30004/{vista_id}")]
async fn F_40002(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<(i32, i32)>,
) -> Result<HttpResponse> {
    let V_30002 = extract_user(&req, &data).await?;
    let (modulo_id, vista_id) = path.into_inner();
    
    // Obtener módulo
    let V_30003 = sqlx::query_as::<_, ModuloRegistro>(
        "SELECT id, nombre_modulo, descripcion, icono, orden, activo FROM modulos_registros WHERE id = $1"
    )
    .bind(modulo_id)
    .fetch_one(&data.pool)
    .await
    .map_err(|_| error::ErrorNotFound("Módulo no encontrado"))?;
    
    // Obtener V_30004
    let V_30004 = sqlx::query_as::<_, ModuloVista>(
        "SELECT id, modulo_id, nombre_vista, V_30007, tabla_origen, descripcion, orden, activo FROM modulos_vistas WHERE id = $1 AND modulo_id = $2"
    )
    .bind(vista_id)
    .bind(modulo_id)
    .fetch_one(&data.pool)
    .await
    .map_err(|_| error::ErrorNotFound("Vista no encontrada"))?;
    
    // Obtener vistas disponibles del módulo
    let V_30005 = sqlx::query_as::<_, ModuloVista>(
        "SELECT id, modulo_id, nombre_vista, V_30007, tabla_origen, descripcion, orden, activo FROM modulos_vistas WHERE modulo_id = $1 AND activo = true ORDER BY orden"
    )
    .bind(modulo_id)
    .fetch_all(&data.pool)
    .await
    .unwrap_or_default();
    
    // Cargar V_30008 según V_30006 origen
    let V_30006 = V_30004.tabla_origen.as_deref().unwrap_or("personas");
    let V_30007 = V_30004.V_30007.as_deref().unwrap_or("lista");
    
    let mut V_30008 = vec![];
    
    if V_30007 == "lista" || V_30007 == "crear" {
        V_30008 = match V_30006 {
            "personas" => {
                sqlx::query("SELECT credencial, nombre, apellido, nombre_apellido FROM personas LIMIT 50 OFFSET 0")
                    .fetch_all(&data.pool)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|row| {
                        json!({
                            "id": row.get::<String, _>(0),
                            "titulo": format!("{} {}", row.get::<String, _>(1), row.get::<String, _>(2)),
                            "credencial": row.get::<String, _>(0),
                            "nombre": row.get::<String, _>(1),
                            "apellido": row.get::<String, _>(2),
                        })
                    })
                    .collect()
            },
            "expedientes" => {
                sqlx::query("SELECT id::text, numero_expediente, tipo_identificador, estado FROM expedientes LIMIT 50 OFFSET 0")
                    .fetch_all(&data.pool)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|row| {
                        json!({
                            "id": row.get::<String, _>(0),
                            "titulo": format!("Exp. {}", row.get::<String, _>(1)),
                            "numero": row.get::<String, _>(1),
                            "tipo": row.get::<String, _>(2),
                            "estado": row.get::<String, _>(3),
                        })
                    })
                    .collect()
            },
            "expediente_agentes" => {
                sqlx::query("SELECT id::text, credencial, nombre_completo, rol FROM expediente_agentes LIMIT 50 OFFSET 0")
                    .fetch_all(&data.pool)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|row| {
                        json!({
                            "id": row.get::<String, _>(0),
                            "credencial": row.get::<String, _>(1),
                            "nombre": row.get::<String, _>(2),
                            "rol": row.get::<Option<String>, _>(3).unwrap_or_default(),
                        })
                    })
                    .collect()
            },
            "expediente_suspensiones" => {
                sqlx::query("SELECT id::text, credencial, nombre_completo, estado FROM expediente_suspensiones LIMIT 50 OFFSET 0")
                    .fetch_all(&data.pool)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|row| {
                        json!({
                            "id": row.get::<String, _>(0),
                            "credencial": row.get::<String, _>(1),
                            "nombre": row.get::<String, _>(2),
                            "estado": row.get::<String, _>(3),
                        })
                    })
                    .collect()
            },
            _ => vec![],
        };
    }
    
    #[derive(Template)]
    #[template(path = "modulos_registros.html")]
    struct S_50001 {
        user_name: String,
        modulo_nombre: String,
        modulo_descripcion: String,
        modulo_id: i32,
        vista_nombre: String,
        vista_tipo: String,
        vista_id: i32,
        V_30005: Vec<ModuloVista>,
        V_30008: Vec<Value>,
    }
    
    render_html(&S_50001 {
        user_name: V_30002.display_name,
        modulo_nombre: V_30003.nombre_modulo,
        modulo_descripcion: V_30003.descripcion.unwrap_or_default(),
        modulo_id,
        vista_nombre: V_30004.nombre_vista,
        vista_tipo: V_30007.to_string(),
        vista_id,
        V_30005,
        V_30008,
    })
}

// Dashboard de módulos con estadísticas
#[get("/V_30001/dashboard")]
async fn F_40003(req: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse> {
    let V_30002 = extract_user(&req, &data).await?;
    
    // Cargar estadísticas
    let V_30003: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT 
            (SELECT COUNT(*) FROM personas) as personas,
            (SELECT COUNT(*) FROM expedientes) as expedientes,
            (SELECT COUNT(*) FROM expediente_agentes) as involucrados,
            (SELECT COUNT(*) FROM expediente_suspensiones) as suspensiones"
    )
    .fetch_one(&data.pool)
    .await
    .unwrap_or((0, 0, 0, 0));
    
    // Cargar datos por estado
    let V_30004: Vec<(String, i64)> = sqlx::query_as(
        "SELECT estado, COUNT(*) FROM expedientes GROUP BY estado"
    )
    .fetch_all(&data.pool)
    .await
    .unwrap_or_default();
    
    // Cargar suspensiones activas
    let V_30005: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM expediente_suspensiones WHERE estado = 'Vigente'"
    )
    .fetch_one(&data.pool)
    .await
    .unwrap_or(0);
    
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(serde_json::json!({
            "usuario": V_30002.display_name,
            "estadisticas": {
                "personas": V_30003.0,
                "expedientes": V_30003.1,
                "involucrados": V_30003.2,
                "suspensiones": V_30003.3,
                "V_30005": V_30005,
            },
            "V_30004": V_30004,
        })))
}
