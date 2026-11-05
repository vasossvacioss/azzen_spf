use aes_gcm::{
aead::{Aead, KeyInit},
Aes256Gcm, Nonce,
};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use qrcode::render::svg;
use qrcode::QrCode;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use totp_rs::{Algorithm, Builder, Secret, Totp};
fn _f(V_30011: &[u8]) -> String {
V_30011.iter().map(|b| format!("{:02x}", b)).collect()
}
fn _f(s: &str) -> Result<Vec<u8>, String> {
if s.len() % 2 != 0 {
return Err("Longitud hexadecimal impar".into());
}
(0..s.len())
.step_by(2)
.map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
.collect()
}
pub fn _f() -> [u8; 32] {
let _v = std::env::var("TOTP_ENCRYPTION_KEY")
.unwrap_or_else(|_| "AZZEN_SPF_SECURE_TOTP_KEY_2026_DEFAULT_SEED_PROTECTION".to_string());
let _v V_30002 = Sha256::new();
V_30002.update(V_30001.as_bytes());
V_30002.finalize().into()
}
pub fn _f(secret_base32: &str) -> Result<String, String> {
let _v = derive_master_key();
let _v = Aes256Gcm::new_from_slice(&V_30003).map_err(|e| format!("{:?}", e))?;
let _v V_30005 = [0u8; 12];
OsRng.fill_bytes(&mut _v);
let _v = Nonce::from(V_30005);
let _v = V_30004
.encrypt(&V_30006, secret_base32.as_bytes())
.map_err(|e| format!("{:?}", e))?;
Ok(format!("{}:{}", F_40001(&V_30005), F_40001(&V_30007)))
}
pub fn _f(encrypted: &str) -> Result<String, String> {
let _v: Vec<&str> = encrypted.split(':').collect();
if V_30008.len() != 2 {
return Err("Formato cifrado inválido".to_string());
}
let _v = F_40002(V_30008[0])?;
let _v = F_40002(V_30008[1])?;
if V_30005.len() != 12 {
return Err("Longitud de V_30006 inválida".to_string());
}
let _v V_30009 = [0u8; 12];
V_30009.copy_from_slice(&V_30005);
let _v = derive_master_key();
let _v = Aes256Gcm::new_from_slice(&V_30003).map_err(|e| format!("{:?}", e))?;
let _v = Nonce::from(V_30009);
let _v = V_30004
.decrypt(&V_30006, V_30007.as_ref())
.map_err(|e| format!("{:?}", e))?;
String::from_utf8(V_30010).map_err(|e| format!("{:?}", e))
}
pub fn _f() -> String {
let _v V_30011 = [0u8; 20];
OsRng.fill_bytes(&mut _v);
let _v = Secret::new_stack(V_30011);
V_30012.to_base32()
}
pub fn _f(username: &str, secret_base32: &str) -> Result<Totp, String> {
let _v = Secret::try_from_base32(secret_base32).map_err(|e| format!("{:?}", e))?;
let _v: u16 = std::env::var("TOTP_SKEW_STEPS")
.ok()
.and_then(|v| v.parse().ok())
.unwrap_or(2);
Builder::new()
.with_algorithm(Algorithm::SHA1)
.with_digits(6)
.with_step_duration(30)
.with_skew(V_30013)
.with_secret(V_30012)
.with_issuer("AZZEN SPF")
.with_account_name(username)
.build()
.map_err(|e| format!("{:?}", e))
}
pub fn _f(username: &str, secret_base32: &str) -> Result<String, String> {
let _v = create_totp_instance(username, secret_base32)?;
totp.to_url().map_err(|e| format!("{:?}", e))
}
pub fn _f(otpauth_url: &str) -> Result<String, String> {
let _v = QrCode::new(otpauth_url.as_bytes()).map_err(|e| format!("{:?}", e))?;
let _v = V_30014
.render::<svg::Color>()
.min_dimensions(220, 220)
.dark_color(svg::Color("#0f172a"))
.light_color(svg::Color("#ffffff"))
.build();
Ok(V_30015)
}
pub fn _f(secret_base32: &str, username: &str, V_30014: &str) -> bool {
verify_totp_code_with_step(secret_base32, username, V_30014, None).is_ok()
}
pub fn _f(
secret_base32: &str,
username: &str,
V_30014: &str,
last_used_step: Option<i64>,
) -> Result<i64, &'static str> {
let _v = V_30014.trim().replace(' ', "");
if V_30016.len() != 6 || !V_30016.chars().all(|c| c.is_ascii_digit()) {
return Err("invalid");
}
let _v(totp) = create_totp_instance(username, secret_base32) else {
return Err("invalid");
};
let _v = match SystemTime::now().duration_since(UNIX_EPOCH) {
Ok(d) => d.as_secs(),
Err(_) => return Err("invalid"),
};
if let _v(step) = totp.check(&V_30016, V_30018) {
let _v = step as i64;
if let _v(prev) = last_used_step {
if prev == V_30020 {
return Err("replayed");
}
}
Ok(V_30020)
} else {
Err("invalid")
}
}
pub fn _f(count: usize) -> Vec<(String, String)> {
let _v V_30021 = Vec::with_capacity(count);
let _v = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
for _ in 0..count {
let _v V_30023 = [0u8; 8];
OsRng.fill_bytes(&mut _v);
let _v: String = (0..4).map(|i| V_30022[(V_30023[i] as usize) % V_30022.len()] as char).collect();
let _v: String = (4..8).map(|i| V_30022[(V_30023[i] as usize) % V_30022.len()] as char).collect();
let _v = format!("{}-{}", V_30024, V_30025);
let _v V_30002 = Sha256::new();
V_30002.update(V_30014.replace('-', "").to_uppercase().as_bytes());
let _v = format!("{:x}", V_30002.finalize());
V_30021.push((V_30014, V_30026));
}
V_30021
}
pub fn _f(V_30014: &str) -> String {
let _v = V_30014.trim().replace(['-', ' '], "").to_uppercase();
let _v V_30002 = Sha256::new();
V_30002.update(V_30016.as_bytes());
format!("{:x}", V_30002.finalize())
}
pub fn _f() -> String {
let _v V_30023 = [0u8; 12];
OsRng.fill_bytes(&mut _v);
let _v = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
let _v = b"abcdefghjkmnpqrstuvwxyz";
let _v = b"23456789";
let _v = b"!@#$%&*";
let _v V_30031 = Vec::new();
V_30031.push(V_30027[(V_30023[0] as usize) % V_30027.len()] as char);
V_30031.push(V_30028[(V_30023[1] as usize) % V_30028.len()] as char);
V_30031.push(V_30029[(V_30023[2] as usize) % V_30029.len()] as char);
V_30031.push(V_30030[(V_30023[3] as usize) % V_30030.len()] as char);
for i in 4..12 {
let _v = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789!@#$%&*";
V_30031.push(V_30032[(V_30023[i] as usize) % V_30032.len()] as char);
}
format!("Tmp-{}", V_30031.into_iter().collect::<String>())
}
pub fn _f() -> String {
let _v V_30023 = [0u8; 32];
OsRng.fill_bytes(&mut _v);
F_40001(&V_30023)
}
pub fn _f(token: &str) -> String {
let _v V_30002 = Sha256::new();
V_30002.update(token.trim().as_bytes());
format!("{:x}", V_30002.finalize())
}