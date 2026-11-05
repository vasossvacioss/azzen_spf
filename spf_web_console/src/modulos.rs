// Módulo para manejo de registros de módulos
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone, FromRow, Serialize, Deserialize)]
pub struct ModuloRegistro {
    pub id: i32,
    pub nombre_modulo: String,
    pub descripcion: Option<String>,
    pub icono: Option<String>,
    pub orden: i32,
    pub activo: bool,
}

#[derive(Clone, FromRow, Serialize, Deserialize)]
pub struct ModuloVista {
    pub id: i32,
    pub modulo_id: i32,
    pub nombre_vista: String,
    pub tipo_vista: Option<String>,
    pub tabla_origen: Option<String>,
    pub descripcion: Option<String>,
    pub orden: i32,
    pub activo: bool,
}

#[derive(Clone, FromRow, Serialize, Deserialize)]
pub struct ModuloCampo {
    pub id: i32,
    pub vista_id: i32,
    pub nombre_campo: Option<String>,
    pub tipo_campo: Option<String>,
    pub requerido: bool,
    pub orden: i32,
    pub visible: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModuloVistaRendered {
    pub id: i32,
    pub nombre_vista: String,
    pub tipo_vista: Option<String>,
    pub tabla_origen: Option<String>,
    pub descripcion: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModuloRendered {
    pub id: i32,
    pub nombre_modulo: String,
    pub descripcion: Option<String>,
    pub icono: Option<String>,
    pub vistas: Vec<ModuloVistaRendered>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RegistroPersona {
    pub id: String,
    pub titulo: String,
    pub credencial: String,
    pub nombre: String,
    pub apellido: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RegistroExpediente {
    pub id: String,
    pub titulo: String,
    pub numero_expediente: String,
    pub tipo_identificador: String,
    pub estado: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RegistroInvolucrado {
    pub id: String,
    pub titulo: String,
    pub expediente: String,
    pub credencial: String,
    pub nombre: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RegistroSuspension {
    pub id: String,
    pub titulo: String,
    pub credencial: String,
    pub expediente: String,
    pub estado: String,
}

#[derive(Deserialize)]
pub struct CrearRegistroForm {
    pub nombre: Option<String>,
    pub apellido: Option<String>,
    pub credencial: Option<String>,
    pub descripcion: Option<String>,
}
