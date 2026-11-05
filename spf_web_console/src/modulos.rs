use serde::{Deserialize, Serialize};
use sqlx::FromRow;
pub struct _S {
pub id: i32,
pub nombre_modulo: String,
pub descripcion: Option<String>,
pub icono: Option<String>,
pub orden: i32,
pub activo: bool,
}
pub struct _S {
pub id: i32,
pub modulo_id: i32,
pub nombre_vista: String,
pub tipo_vista: Option<String>,
pub tabla_origen: Option<String>,
pub descripcion: Option<String>,
pub orden: i32,
pub activo: bool,
}
pub struct _S {
pub id: i32,
pub vista_id: i32,
pub nombre_campo: Option<String>,
pub tipo_campo: Option<String>,
pub requerido: bool,
pub orden: i32,
pub visible: bool,
}
pub struct _S {
pub id: i32,
pub nombre_vista: String,
pub tipo_vista: Option<String>,
pub tabla_origen: Option<String>,
pub descripcion: Option<String>,
}
pub struct _S {
pub id: i32,
pub nombre_modulo: String,
pub descripcion: Option<String>,
pub icono: Option<String>,
pub vistas: Vec<ModuloVistaRendered>,
}
pub struct _S {
pub id: String,
pub titulo: String,
pub credencial: String,
pub nombre: String,
pub apellido: String,
}
pub struct _S {
pub id: String,
pub titulo: String,
pub numero_expediente: String,
pub tipo_identificador: String,
pub estado: String,
}
pub struct _S {
pub id: String,
pub titulo: String,
pub expediente: String,
pub credencial: String,
pub nombre: String,
}
pub struct _S {
pub id: String,
pub titulo: String,
pub credencial: String,
pub expediente: String,
pub estado: String,
}
pub struct _S {
pub nombre: Option<String>,
pub apellido: Option<String>,
pub credencial: Option<String>,
pub descripcion: Option<String>,
}