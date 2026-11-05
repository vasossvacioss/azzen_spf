function _f(text) {
if (text === null || text === undefined) return '';
return String(text)
.replace(/&/g, '&amp;')
.replace(/</g, '&lt;')
.replace(/>/g, '&gt;')
.replace(/"/g, '&quot;')
.replace(/'/g, '&#039;');
}
document.addEventListener('DOMContentLoaded', () => {
initNavbarSearch();
initKeyboardShortcuts();
});
function _f() {
const _c= document.getElementById('navbar-search-input');
const _c= document.getElementById('navbar-search-dropdown');
if (!input || !dropdown) return;
let _l= null;
let _l= -1;
input.addEventListener('input', () => {
const _c= input.value.trim();
clearTimeout(debounceTimer);
selectedIndex = -1;
if (query.length < 2) {
dropdown.style.display = 'none';
dropdown.innerHTML = '';
return;
}
debounceTimer = setTimeout(() => {
fetchGlobalSearchResults(query, dropdown);
}, 220);
});
input.addEventListener('keydown', (e) => {
const _c= dropdown.querySelectorAll('.search-result-item');
if (dropdown.style.display !== 'none' && items.length > 0) {
if (e.key === 'ArrowDown') {
e.preventDefault();
selectedIndex = (selectedIndex + 1) % items.length;
updateSelection(items, selectedIndex);
return;
} else if (e.key === 'ArrowUp') {
e.preventDefault();
selectedIndex = (selectedIndex - 1 + items.length) % items.length;
updateSelection(items, selectedIndex);
return;
} else if (e.key === 'Enter') {
if (selectedIndex >= 0 && items[selectedIndex]) {
e.preventDefault();
items[selectedIndex].click();
return;
} else if (input.value.trim()) {
e.preventDefault();
window.location.href = '/busqueda-global?credencial=' + encodeURIComponent(input.value.trim());
return;
}
} else if (e.key === 'Escape') {
dropdown.style.display = 'none';
return;
}
} else if (e.key === 'Enter' && input.value.trim()) {
e.preventDefault();
window.location.href = '/busqueda-global?credencial=' + encodeURIComponent(input.value.trim());
}
});
document.addEventListener('click', (e) => {
if (!input.contains(e.target) && !dropdown.contains(e.target)) {
dropdown.style.display = 'none';
}
});
input.addEventListener('focus', () => {
if (input.value.trim().length >= 2 && dropdown.children.length > 0) {
dropdown.style.display = 'block';
}
});
}
function _f(items, index) {
items.forEach((el, i) => {
if (i === index) {
el.classList.add('selected');
el.scrollIntoView({ block: 'nearest' });
} else {
el.classList.remove('selected');
}
});
}
async function _f(q, dropdown) {
dropdown.innerHTML = '<div class="search-loading">Buscando coincidencias...</div>';
dropdown.style.display = 'block';
try {
const _c= await fetch('/api/global-search?q=' + encodeURIComponent(q));
if (!res.ok) throw new Error('Error en búsqueda');
const _c= await res.json();
renderDropdown(data, q, dropdown);
} catch (err) {
dropdown.innerHTML = '<div class="search-empty">Error al consultar el servidor</div>';
}
}
function _f(data, query, dropdown) {
const _c= data.expedientes && data.expedientes.length > 0;
const _c= data.agentes && data.agentes.length > 0;
const _c= data.suspensiones && data.suspensiones.length > 0;
if (!hasExps && !hasAgentes && !hasSusp) {
dropdown.innerHTML = `
<div class="search-empty">
No se encontraron expedientes, agentes ni suspensiones para <strong>"${escapeHtml(query)}"</strong>
</div>
<a href="/busqueda-global?credencial=${encodeURIComponent(query)}" class="search-footer-link">
Probar en Búsqueda Global completa →
</a>
`;
return;
}
let _l= '';
if (hasExps) {
html += '<div class="search-group-title">📁 Expedientes y Actuaciones</div>';
data.expedientes.forEach(exp => {
html += `
<a href="${exp.url}" class="search-result-item">
<div class="search-item-primary">
<span class="badge badge-info">${escapeHtml(exp.estado)}</span>
<strong>${escapeHtml(exp.numero)}</strong>
</div>
<div class="search-item-secondary">${escapeHtml(exp.caratula)}</div>
</a>
`;
});
}
if (hasAgentes) {
html += '<div class="search-group-title">👥 Agentes y Personal</div>';
data.agentes.forEach(ag => {
html += `
<a href="${ag.url}" class="search-result-item">
<div class="search-item-primary">
<span class="badge badge-secondary">Cred. ${escapeHtml(ag.credencial)}</span>
<strong>${escapeHtml(ag.nombre)}</strong>
</div>
<div class="search-item-secondary">Historial disciplinario centralizado</div>
</a>
`;
});
}
if (hasSusp) {
html += '<div class="search-group-title">🛡️ Suspensiones Preventivas</div>';
data.suspensiones.forEach(s => {
const _c= s.estado === 'Vigente' ? 'badge-danger' : 'badge-success';
html += `
<a href="${s.url}" class="search-result-item">
<div class="search-item-primary">
<span class="badge ${badgeCls}">${escapeHtml(s.estado)}</span>
<strong>${escapeHtml(s.nombre)} (Cred. ${escapeHtml(s.credencial)})</strong>
</div>
<div class="search-item-secondary">Encuadre Art. 419: ${escapeHtml(s.articulo || 'Sin datos')}</div>
</a>
`;
});
}
html += `
<a href="/busqueda-global?credencial=${encodeURIComponent(query)}" class="search-footer-link">
🔍 Ver todos los resultados en Búsqueda Global →
</a>
`;
dropdown.innerHTML = html;
}
function _f(str) {
if (!str) return '';
return String(str)
.replace(/&/g, '&amp;')
.replace(/</g, '&lt;')
.replace(/>/g, '&gt;')
.replace(/"/g, '&quot;');
}
function _f() {
document.addEventListener('keydown', (e) => {
if ((e.key === '/' || (e.ctrlKey && e.key.toLowerCase() === 'k')) && !['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement.tagName)) {
const _c= document.getElementById('navbar-search-input');
if (input) {
e.preventDefault();
input.focus();
input.select();
}
}
});
}
function _f(btn) {
const _c= btn.nextElementSibling;
if (!menu) return;
const _c= menu.classList.contains('show');
document.querySelectorAll('.dropdown-content.show').forEach(el => el.classList.remove('show'));
if (!wasShown) {
menu.classList.add('show');
}
}
document.addEventListener('click', (e) => {
if (!e.target.closest('.actions-dropdown')) {
document.querySelectorAll('.dropdown-content.show').forEach(el => el.classList.remove('show'));
}
});
document.addEventListener('keydown', (e) => {
if (e.key === 'Escape') {
closeInvestigarExpModal();
closeInvestigarSuspModal();
closeInvestigarSheetModal();
closeEditarFilaNovedadModal();
}
});
function _f(trigger) {
const _c= trigger.closest('tr');
if (!tr) return;
const _c= tr.getAttribute('data-id') || '';
const _c= tr.getAttribute('data-numero') || '';
const _c= tr.getAttribute('data-tipo') || 'Sumario';
const _c= tr.getAttribute('data-reparticion') || 'Sin datos';
const _c= tr.getAttribute('data-estado') || 'Abierto';
const _c= tr.getAttribute('data-fecha') || '-';
const _c= tr.getAttribute('data-agentes') || '0';
const _c= tr.querySelector('.raw-data-motivo');
const _c= motivoEl ? motivoEl.innerText.trim() : (tr.querySelector('.cell-motivo')?.innerText.trim() || 'Sin motivo registrado.');
const _c= tr.querySelector('.raw-data-instructor');
const _c= instructorEl ? instructorEl.innerText.trim() : (tr.querySelector('.cell-instructor')?.innerText.trim() || 'Sin instructor asignado');
const _c= document.getElementById('modal-investigar-expediente');
if (!modal) return;
const _c= (id, val) => {
const _c= document.getElementById(id);
if (el) el.textContent = val;
};
setTxt('inv-exp-numero', numero);
setTxt('inv-exp-tipo', tipo);
setTxt('inv-exp-reparticion', reparticion);
setTxt('inv-exp-fecha', fecha);
setTxt('inv-exp-agentes', agentes + (agentes === '1' ? ' agente vinculado' : ' agentes vinculados'));
setTxt('inv-exp-instructor', instructor);
setTxt('inv-exp-motivo', motivo);
const _c= document.getElementById('inv-exp-estado');
if (estadoBadge) {
estadoBadge.textContent = estado;
estadoBadge.className = 'mc-badge ' + (estado === 'Abierto' || estado === 'Inicio' ? 'estado-pendiente' : (estado === 'Reconstrucción' || estado === 'Reapertura' ? 'estado-rechazada' : 'estado-aprobada'));
}
const _c= document.getElementById('inv-exp-btn-ver');
if (btnVer && id) btnVer.href = '/expedientes/' + id;
const _c= document.getElementById('inv-exp-btn-novedad');
if (btnNovedad && id) btnNovedad.href = '/expedientes/' + id + '?tab=tab-novedades';
if (modal.parentElement !== document.body) {
document.body.appendChild(modal);
}
modal.classList.add('active');
modal.classList.add('show');
modal.style.setProperty('display', 'flex', 'important');
modal.style.zIndex = '999999';
modal.style.visibility = 'visible';
modal.style.opacity = '1';
}
function _f() {
const _c= document.getElementById('modal-investigar-expediente');
if (modal) {
modal.classList.remove('active');
modal.classList.remove('show');
modal.style.setProperty('display', 'none', 'important');
}
}
function _f(trigger) {
const _c= trigger.closest('tr');
if (!tr) return;
const _c= tr.getAttribute('data-id') || '';
const _c= tr.getAttribute('data-credencial') || '';
const _c= tr.getAttribute('data-nombre') || '';
const _c= tr.getAttribute('data-expediente') || 'Sin vincular';
const _c= tr.getAttribute('data-exp-id') || '';
const _c= tr.getAttribute('data-fecha-inicio') || '-';
const _c= tr.getAttribute('data-articulo') || 'a)';
const _c= tr.getAttribute('data-estado') || 'Vigente';
const _c= tr.getAttribute('data-fecha-fin') || '—';
const _c= tr.querySelector('.raw-data-motivo-fin');
const _c= motivoFinEl ? motivoFinEl.innerText.trim() : '';
const _c= document.getElementById('modal-investigar-suspension');
if (!modal) return;
const _c= (id, val) => {
const _c= document.getElementById(id);
if (el) el.textContent = val;
};
setTxt('inv-susp-nombre', nombre);
setTxt('inv-susp-credencial', 'Credencial ' + credencial);
setTxt('inv-susp-expediente', expediente);
setTxt('inv-susp-fecha-inicio', fechaInicio);
setTxt('inv-susp-articulo', 'Artículo 419 Inciso ' + articulo);
setTxt('inv-susp-fecha-fin', fechaFin);
setTxt('inv-susp-motivo-fin', motivoFin || 'Actualmente en curso sin fecha de finalización.');
const _c= document.getElementById('inv-susp-estado');
if (badge) {
badge.textContent = estado;
badge.className = 'mc-badge ' + (estado === 'Vigente' ? 'estado-rechazada' : 'estado-aprobada');
}
const _c= document.getElementById('inv-susp-btn-agente');
if (btnAgente && credencial) btnAgente.href = '/agentes/' + credencial + '/historial';
const _c= document.getElementById('inv-susp-btn-exp');
if (btnExp) {
if (expId && expId !== '0') {
btnExp.href = '/expedientes/' + expId;
btnExp.style.display = 'inline-flex';
} else {
btnExp.style.display = 'none';
}
}
if (modal.parentElement !== document.body) {
document.body.appendChild(modal);
}
modal.classList.add('active');
modal.classList.add('show');
modal.style.setProperty('display', 'flex', 'important');
modal.style.zIndex = '999999';
modal.style.visibility = 'visible';
modal.style.opacity = '1';
}
function _f() {
const _c= document.getElementById('modal-investigar-suspension');
if (modal) {
modal.classList.remove('active');
modal.classList.remove('show');
modal.style.setProperty('display', 'none', 'important');
}
}
function _f(trigger) {
const _c= trigger.closest('tr');
if (!tr) return;
const _c= tr.getAttribute('data-row-num') || '';
const _c= tr.getAttribute('data-sheet-name') || '';
const _c= tr.getAttribute('data-file-name') || '';
let _l= document.getElementById('modal-investigar-sheet');
if (!modal) {
modal = document.createElement('div');
modal.id = 'modal-investigar-sheet';
modal.className = 'modal-overlay';
modal.style.display = 'none';
modal.onclick = (e) => { if (e.target === modal) closeInvestigarSheetModal(); };
modal.innerHTML = `
<div class="modal-investigar-box" style="width: 860px; max-width: 95vw;">
<div class="modal-investigar-header">
<div>
<div style="display:flex; align-items:center; gap:8px; margin-bottom:4px;">
<span id="inv-sheet-badge" class="mc-badge estado-entregado">Fila Fuente</span>
<span class="muted" style="font-size:12px;">${escapeHtml(fileName)} · ${escapeHtml(sheetName)}</span>
</div>
<h2 id="inv-sheet-title" style="margin:0; font-size:18px; color:var(--azzen-dark);">Contenido Completo de la Fila</h2>
</div>
<button type="button" class="btn-close" onclick="closeInvestigarSheetModal()">✕</button>
</div>
<div class="modal-investigar-body" id="inv-sheet-body" style="max-height: 65vh; overflow-y: auto;"></div>
<div class="modal-investigar-footer" style="display:flex; justify-content:space-between; align-items:center;">
<div id="inv-sheet-quick-actions" style="display:flex; gap:8px;"></div>
<button type="button" class="button is-dark is-outlined btn-sm" onclick="closeInvestigarSheetModal()">Cerrar</button>
</div>
</div>
`;
document.body.appendChild(modal);
}
const _c= document.getElementById('inv-sheet-body');
const _c= document.getElementById('inv-sheet-title');
const _c= document.getElementById('inv-sheet-badge');
const _c= document.getElementById('inv-sheet-quick-actions');
if (badgeEl && rowNum) badgeEl.textContent = 'Fila #' + rowNum;
if (titleEl && rowNum) titleEl.textContent = sheetName ? `Fila #${rowNum} · ${sheetName}` : `Fila #${rowNum}`;
let _l= [];
const _c= tr.querySelector('.raw-sheet-json') || tr.querySelector('.raw-sheet-json-script');
if (elHolder) {
try {
let _l= (elHolder.value !== undefined && elHolder.value !== '') ? elHolder.value : elHolder.textContent;
rawText = rawText.trim();
if (rawText.includes('&#34;') || rawText.includes('&quot;')) {
const _c= document.createElement('textarea');
decoder.innerHTML = rawText;
rawText = decoder.value;
}
fields = JSON.parse(rawText);
} catch (e) {
console.error('Error parsing raw-sheet-json', e);
}
}
const _c= tr.querySelector('.raw-sheet-json-script');
if ((!fields || fields.length === 0) && scriptEl) {
try {
let _l= scriptEl.textContent.trim();
if (scrText.includes('&#34;') || scrText.includes('&quot;')) {
const _c= document.createElement('textarea');
decoder.innerHTML = scrText;
scrText = decoder.value;
}
fields = JSON.parse(scrText);
} catch (e) {
console.error('Error parsing raw-sheet-json-script', e);
}
}
if (!fields || fields.length === 0) {
const _c= tr.closest('table');
if (table) {
const _c= Array.from(table.querySelectorAll('thead th')).map(th => th.innerText.trim());
const _c= Array.from(tr.querySelectorAll('td')).map(td => td.innerText.trim());
headers.forEach((h, idx) => {
if (h && h !== 'ACCIONES' && idx < cells.length) {
fields.push({ name: h, value: cells[idx] || '—' });
}
});
}
}
const _c= [];
const _c= [];
let _l= '';
let _l= '';
fields.forEach(f => {
const _c= String(f.value || '—').trim();
const _c= f.name.toLowerCase();
if (nameLow.includes('credencial') && !detectedCredencial && val !== '—') {
detectedCredencial = val;
}
if ((nameLow.includes('expediente') || nameLow.includes('sumario')) && !detectedExpediente && val !== '—') {
detectedExpediente = val;
}
const _c= val.length > 50 || nameLow.includes('motivo') || nameLow.includes('observac') || nameLow.includes('resoluc') || nameLow.includes('encuadre') || nameLow.includes('fundamento') || nameLow.includes('caratula');
if (isLongText) {
textFields.push(f);
} else {
standardFields.push(f);
}
});
let _l= '';
if (standardFields.length > 0) {
html += '<div class="investigar-grid-fields" style="grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); margin-bottom: 14px;">';
standardFields.forEach(f => {
html += `
<div class="investigar-field-item">
<span class="investigar-field-label">${escapeHtml(f.name)}</span>
<span class="investigar-field-val">${escapeHtml(f.value)}</span>
</div>
`;
});
html += '</div>';
}
if (textFields.length > 0) {
textFields.forEach((f, idx) => {
const _c= 'inv-sheet-text-' + idx;
html += `
<div class="investigar-text-card" style="margin-bottom: 12px;">
<div class="investigar-text-title">
<span>📝 ${escapeHtml(f.name)}</span>
<button type="button" class="button is-dark is-outlined btn-sm" onclick="copiarTextoDeElemento('${cardId}', this)" style="padding: 2px 8px; font-size: 11px;">
📋 Copiar texto
</button>
</div>
<div id="${cardId}" class="investigar-text-content" style="white-space: pre-wrap; max-height: 200px; overflow-y: auto;">${escapeHtml(f.value)}</div>
</div>
`;
});
}
if (bodyEl) bodyEl.innerHTML = html;
window._lastInvestigatedTr = tr;
if (actionsEl) {
let _l= `<button type="button" class="btn-editar-fila" onclick="closeInvestigarSheetModal(); if(window._lastInvestigatedTr) openEditarFilaNovedadModal(window._lastInvestigatedTr);"><span>✏️</span> Editar / Agregar Novedad</button>`;
if (detectedCredencial && detectedCredencial !== '—') {
actionsHtml += `<a href="/agentes/${encodeURIComponent(detectedCredencial)}/historial" class="button is-azzen btn-sm" target="_blank">👤 Legajo de Agente (${escapeHtml(detectedCredencial)}) ↗</a>`;
}
if (detectedExpediente && detectedExpediente !== '—') {
actionsHtml += `<a href="/busqueda-global?expediente=${encodeURIComponent(detectedExpediente)}" class="button is-dark is-outlined btn-sm" target="_blank">📁 Buscar Expediente ↗</a>`;
}
actionsEl.innerHTML = actionsHtml;
}
if (modal.parentElement !== document.body) {
document.body.appendChild(modal);
}
modal.classList.add('active');
modal.classList.add('show');
modal.style.setProperty('display', 'flex', 'important');
modal.style.zIndex = '999999';
modal.style.visibility = 'visible';
modal.style.opacity = '1';
}
function _f() {
const _c= document.getElementById('modal-investigar-sheet');
if (modal) {
modal.classList.remove('active');
modal.classList.remove('show');
modal.style.setProperty('display', 'none', 'important');
}
}
function _f(elementId, btn) {
const _c= document.getElementById(elementId);
if (!el) return;
const _c= el.innerText || el.textContent;
navigator.clipboard.writeText(text).then(() => {
if (btn) {
const _c= btn.innerHTML;
btn.innerHTML = '✓ ¡Copiado!';
setTimeout(() => { btn.innerHTML = origText; }, 1800);
}
});
}
let _l= false;
function _f(e) {
if (e) e.stopPropagation();
const _c= document.getElementById('notif-dropdown');
if (!dropdown) return;
notifDropdownOpen = !notifDropdownOpen;
dropdown.style.display = notifDropdownOpen ? 'flex' : 'none';
if (notifDropdownOpen) {
cargarNotificacionesNavbar();
}
}
document.addEventListener('click', (e) => {
if (!e.target.closest('#navbar-notif-container') && !e.target.closest('.notif-wrapper')) {
const _c= document.getElementById('notif-dropdown');
if (dropdown) dropdown.style.display = 'none';
notifDropdownOpen = false;
}
});
async function _f() {
try {
const _c= await fetch('/api/notificaciones');
if (!res.ok) return;
const _c= await res.json();
const _c= document.getElementById('notif-badge');
if (badge) {
if (data.unread_count > 0) {
badge.textContent = data.unread_count > 99 ? '99+' : data.unread_count;
badge.style.display = 'inline-block';
} else {
badge.style.display = 'none';
}
}
const _c= document.getElementById('notif-items-list') || document.getElementById('notif-list');
if (!listEl) return;
const _c= data.notificaciones || data.items || [];
if (list.length === 0) {
listEl.innerHTML = '<div class="notif-empty" style="padding:16px; text-align:center; color:#777; font-size:13px;">No hay notificaciones recientes.</div>';
return;
}
let _l= '';
list.forEach(n => {
const _c= n.tipo_evento.includes('suspension') ? '🛡️' : (n.tipo_evento.includes('estado') ? '⚖️' : (n.tipo_evento.includes('documento') ? '📄' : '📁'));
const _c= n.leido ? '' : 'unread';
html += `
<a href="${escapeHtml(n.url_destino)}" class="notif-item ${unreadClass}" onclick="marcarNotificacionLeida(${n.id}, event, '${escapeHtml(n.url_destino)}')">
<div class="notif-item-title">
<span>${icon} ${escapeHtml(n.titulo)}</span>
${!n.leido ? '<span style="width:8px; height:8px; border-radius:50%; background:var(--azzen-orange); display:inline-block;"></span>' : ''}
</div>
<div class="notif-item-msg">${escapeHtml(n.mensaje)}</div>
<div class="notif-item-time">${escapeHtml(n.created_at)}</div>
</a>
`;
});
listEl.innerHTML = html;
} catch (err) {
console.error('Error cargando notificaciones', err);
}
}
function _f() {
marcarTodasNotificacionesLeidas();
}
async function _f(id, e, destUrl) {
if (e && destUrl) e.preventDefault();
try {
await fetch('/api/notificaciones/' + id + '/leer', { method: 'POST' });
} catch (err) {}
if (destUrl) {
window.location.href = destUrl;
} else {
cargarNotificacionesNavbar();
if (window.location.pathname.startsWith('/notificaciones')) {
window.location.reload();
}
}
}
async function _f() {
try {
await fetch('/api/notificaciones/marcar-todas', { method: 'POST' });
cargarNotificacionesNavbar();
if (window.location.pathname.startsWith('/notificaciones')) {
window.location.reload();
}
} catch (err) {
console.error('Error marcando todas leídas', err);
}
}
document.addEventListener('DOMContentLoaded', () => {
cargarNotificacionesNavbar();
setInterval(cargarNotificacionesNavbar, 30000);
AzzenValidationEngine.initAllForms();
});
let _l= null;
function _f() {
let _l= document.getElementById('modal-editar-fila-novedad');
if (modal) return modal;
modal = document.createElement('div');
modal.id = 'modal-editar-fila-novedad';
modal.className = 'modal-overlay';
modal.style.display = 'none';
modal.onclick = (e) => { if (e.target === modal) closeEditarFilaNovedadModal(); };
const _c= new Date().toISOString().split('T')[0];
modal.innerHTML = `
<div class="modal-investigar-box" style="width: 840px; max-width: 96vw;">
<div class="modal-investigar-header">
<div>
<div style="display:flex; align-items:center; gap:8px; margin-bottom:4px; flex-wrap:wrap;">
<span id="edit-fila-badge" class="mc-badge estado-entregado" style="font-weight:700;">✏️ Fila</span>
<span id="edit-fila-ver-badge" class="mc-badge estado-aprobada" style="font-size:11px; font-weight:700;" title="Versión actual">v1</span>
<span id="edit-fila-submeta" class="muted" style="font-size:12px;"></span>
</div>
<h2 id="edit-fila-title" style="margin:0; font-size:18px; color:var(--azzen-dark);">Agregar Novedad sobre la Fila</h2>
</div>
<button type="button" class="btn-close" onclick="closeEditarFilaNovedadModal()" aria-label="Cerrar">✕</button>
</div>
<div class="modal-investigar-body" style="max-height: 72vh; overflow-y: auto;">
<div id="edit-fila-context-cards" class="investigar-grid-fields" style="grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); margin-bottom: 12px;"></div>
<div style="background:#eff6ff; border:1px solid #bfdbfe; border-left:4px solid #2563eb; border-radius:6px; padding:10px 14px; margin-bottom:14px; font-size:12.5px; color:#1e40af; display:flex; align-items:flex-start; gap:10px;">
<span style="font-size:18px; line-height:1;">ℹ️</span>
<div>
<strong>Control de Versiones y Trazabilidad:</strong>
El botón de edición está destinado a <strong>agregar novedades sobre esta fila</strong>. Al guardar el cambio, el sistema registrará la novedad y <strong>generará automáticamente una nueva versión (vX+1) de la fila</strong> con auditoría completa.
</div>
</div>
<div id="edit-fila-alert-box" style="display:none; margin-bottom: 12px;"></div>
<form id="form-editar-fila-novedad" onsubmit="submitEditarFilaNovedad(event)" novalidate>
<div style="background:#fffaf7; border:1px solid #f2d5c4; border-radius:6px; padding:14px; margin-bottom:14px;">
<div style="font-weight:700; color:#8a2e00; font-size:13.5px; margin-bottom:10px; display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:6px;">
<span>📝 Datos de la Novedad</span>
<span id="edit-fila-exp-sync-badge" class="mc-badge estado-aprobada" style="display:none; font-size:11px;">🔗 Sincronizado con Expediente</span>
</div>
<div class="form-grid-2" style="display:grid; grid-template-columns: 1fr 1fr; gap:10px; margin-bottom:10px;">
<div class="form-field" style="margin:0;">
<label for="ef-tipo-novedad" style="font-size:12px; font-weight:600;">Tipo de novedad *</label>
<select id="ef-tipo-novedad" name="tipo_novedad" required>
<option value="Observación de fila">Observación sobre la fila</option>
<option value="Novedad procesal">Novedad procesal / Actuación</option>
<option value="Nota sobre suspensión">Nota sobre suspensión preventiva</option>
<option value="Actualización de datos">Actualización / Rectificación de dato</option>
<option value="Vinculación documental">Referencia / Vinculación documental</option>
<option value="Otro">Otro</option>
</select>
</div>
<div class="form-field" style="margin:0;">
<label for="ef-fecha-evento" style="font-size:12px; font-weight:600;">Fecha de la novedad *</label>
<input id="ef-fecha-evento" name="fecha_evento" type="date" value="${todayIso}" required />
</div>
</div>
<div class="form-field" style="margin-bottom:10px;">
<label for="ef-titulo" style="font-size:12px; font-weight:600;">Título / Resumen corto de la novedad *</label>
<input id="ef-titulo" name="titulo" type="text" placeholder="Ej: Recepción de nota DSG / Actualización de situación procesal..." required oninput="validateEditarFilaField('titulo')" onblur="validateEditarFilaField('titulo')" />
<div id="ef-err-titulo" class="azzen-field-feedback"></div>
</div>
<div class="form-field" style="margin-bottom:10px;">
<label for="ef-descripcion" style="font-size:12px; font-weight:600;">Descripción detallada de la novedad *</label>
<textarea id="ef-descripcion" name="descripcion" rows="3" placeholder="Ingrese el detalle completo de la novedad u observación sobre esta fila..." required oninput="validateEditarFilaField('descripcion')" onblur="validateEditarFilaField('descripcion')"></textarea>
<div id="ef-err-descripcion" class="azzen-field-feedback"></div>
</div>
<div class="form-field" style="margin-bottom:0;">
<label for="ef-observaciones" style="font-size:12px; font-weight:600;">Observaciones adicionales / Referencia documental (Opcional)</label>
<input id="ef-observaciones" name="observaciones" type="text" placeholder="Ej: Disposición / Nota DSG / Expediente asociado..." />
</div>
</div>
<div style="display:flex; justify-content:flex-end; gap:8px; margin-bottom:16px;">
<button type="button" class="button is-dark is-outlined btn-sm" onclick="closeEditarFilaNovedadModal()">Cancelar</button>
<button type="submit" id="ef-btn-submit" class="button is-azzen btn-sm" style="padding: 6px 20px; font-weight:700;">
💾 Guardar Novedad y Generar Nueva Versión
</button>
</div>
</form>
<div style="border-top: 1px solid #e2e8f0; padding-top: 14px;">
<div style="font-weight:700; font-size:13.5px; color:var(--azzen-dark); margin-bottom:10px; display:flex; align-items:center; justify-content:space-between;">
<span>🕒 Historial de Novedades y Versiones de la Fila</span>
<span id="ef-hist-count" class="mc-badge estado-entregado">0</span>
</div>
<div id="ef-historial-list" style="display:flex; flex-direction:column; gap:8px;">
<div class="muted" style="font-size:12.5px;">Cargando historial...</div>
</div>
</div>
</div>
</div>
`;
<span>🕒 Historial de Novedades y Ediciones de la Fila</span>
<span id="ef-hist-count" class="mc-badge estado-entregado">0</span>
</div>
<div id="ef-historial-list" style="display:flex; flex-direction:column; gap:8px;">
<div class="muted" style="font-size:12.5px;">Cargando novedades...</div>
</div>
</div>
</div>
</div>
`;
document.body.appendChild(modal);
return modal;
}
function _f(field) {
const _c= document.getElementById('ef-' + field);
const _c= document.getElementById('ef-err-' + field);
if (!input || !errBox) return true;
const _c= (input.value || '').trim();
errBox.innerHTML = '';
input.classList.remove('azzen-input-error', 'azzen-input-valid');
if (field === 'titulo') {
if (val.length < 3) {
input.classList.add('azzen-input-error');
errBox.innerHTML = '<div class="azzen-feedback-item is-error" role="alert"><span>❌</span><span><strong>Error:</strong> El título o resumen debe tener al menos 3 caracteres.</span></div>';
return false;
}
} else if (field === 'descripcion') {
if (val.length < 5) {
input.classList.add('azzen-input-error');
errBox.innerHTML = '<div class="azzen-feedback-item is-error" role="alert"><span>❌</span><span><strong>Error:</strong> La descripción detallada debe tener al menos 5 caracteres.</span></div>';
return false;
}
}
input.classList.add('azzen-input-valid');
return true;
}
async function _f(triggerOrTr) {
const _c= triggerOrTr.tagName === 'TR' ? triggerOrTr : triggerOrTr.closest('tr');
if (!tr) return;
const _c= ensureEditarFilaModalExists();
const _c= tr.getAttribute('data-origen-tipo')
|| (tr.hasAttribute('data-row-num') ? 'hoja' : (tr.hasAttribute('data-articulo') ? 'suspension' : (tr.hasAttribute('data-numero') ? 'expediente' : 'registro')));
const _c= parseInt(tr.getAttribute('data-file-id') || '0', 10) || null;
const _c= parseInt(tr.getAttribute('data-sheet-id') || '0', 10) || null;
const _c= parseInt(tr.getAttribute('data-row-num') || '0', 10) || null;
const _c= parseInt(tr.getAttribute('data-id') || '0', 10) || null;
const _c= parseInt(tr.getAttribute('data-exp-id') || '0', 10) || (origenTipo === 'expediente' ? recordId : null);
const _c= origenTipo === 'suspension' ? recordId : null;
let _l= tr.getAttribute('data-credencial') || '';
let _l= tr.getAttribute('data-nombre') || '';
let _l= tr.getAttribute('data-numero') || tr.getAttribute('data-expediente') || '';
const _c= tr.getAttribute('data-sheet-name') || '';
const _c= tr.getAttribute('data-file-name') || '';
if (!credencial || !nombre || !numeroExp) {
const _c= tr.querySelector('.raw-sheet-json') || tr.querySelector('.raw-sheet-json-script');
if (elHolder) {
try {
let _l= (elHolder.value !== undefined && elHolder.value !== '') ? elHolder.value : elHolder.textContent;
const _c= JSON.parse(rawText.trim());
parsed.forEach(f => {
const _c= (f.name || '').toLowerCase();
const _c= String(f.value || '').trim();
if (v && v !== '—') {
if (nl.includes('credencial') && !credencial) credencial = v;
if ((nl.includes('apellido') || nl.includes('nombre') || nl.includes('causante')) && !nombre) nombre = v;
if ((nl.includes('expediente') || nl.includes('sumario') || nl.includes('resoluc') || nl.includes('orden interna')) && !numeroExp) numeroExp = v;
}
});
} catch (e) {}
}
}
const _c= parseInt(tr.getAttribute('data-version') || '1', 10);
const _c= parseInt(tr.getAttribute('data-novedades') || '0', 10);
_activeEditFilaContext = {
tr,
origen_tipo: origenTipo,
file_id: fileId,
sheet_id: sheetId,
row_number: rowNum,
expediente_id: expIdAttr,
suspension_id: suspIdAttr,
current_version: currentVersion,
credencial: credencial && credencial !== '—' ? credencial : null,
nombre_agente: nombre && nombre !== '—' ? nombre : null,
numero_expediente: numeroExp && numeroExp !== '—' && numeroExp !== 'Sin vincular' ? numeroExp : null,
};
const _c= document.getElementById('form-editar-fila-novedad');
if (form) form.reset();
const _c= new Date().toISOString().split('T')[0];
const _c= document.getElementById('ef-fecha-evento');
if (dateEl) dateEl.value = todayIso;
['titulo', 'descripcion'].forEach(f => {
const _c= document.getElementById('ef-' + f);
const _c= document.getElementById('ef-err-' + f);
if (inp) inp.classList.remove('azzen-input-error', 'azzen-input-valid');
if (err) err.innerHTML = '';
});
const _c= document.getElementById('edit-fila-alert-box');
if (alertBox) {
alertBox.style.display = 'none';
alertBox.innerHTML = '';
}
const _c= document.getElementById('edit-fila-badge');
const _c= document.getElementById('edit-fila-ver-badge');
const _c= document.getElementById('edit-fila-submeta');
const _c= document.getElementById('edit-fila-title');
const _c= document.getElementById('edit-fila-context-cards');
if (badgeEl) {
badgeEl.textContent = rowNum ? `✏️ Fila #${rowNum}` : (origenTipo === 'expediente' ? '✏️ Actuación' : '✏️ Fila');
}
if (verBadgeEl) {
verBadgeEl.textContent = `v${currentVersion}`;
}
if (submetaEl) {
submetaEl.textContent = fileName || sheetName ? `${fileName} · ${sheetName}` : `Módulo: ${origenTipo.toUpperCase()}`;
}
if (titleEl) {
titleEl.textContent = rowNum
? `Agregar Novedad sobre la Fila #${rowNum}`
: (numeroExp && numeroExp !== '—' ? `Agregar Novedad · ${numeroExp}` : 'Agregar Novedad sobre la Fila');
}
if (cardsEl) {
cardsEl.innerHTML = `
<div class="investigar-field-item">
<span class="investigar-field-label">Credencial</span>
<span class="investigar-field-val">${escapeHtml(credencial || '—')}</span>
</div>
<div class="investigar-field-item">
<span class="investigar-field-label">Agente / Causante</span>
<span class="investigar-field-val">${escapeHtml(nombre || '—')}</span>
</div>
<div class="investigar-field-item">
<span class="investigar-field-label">Expediente / Ref.</span>
<span class="investigar-field-val">${escapeHtml(numeroExp || '—')}</span>
</div>
<div class="investigar-field-item">
<span class="investigar-field-label">Versión Actual</span>
<span class="investigar-field-val"><strong style="color:var(--azzen-dark);">v${currentVersion}</strong> (Novedades: ${currentNovedades})</span>
</div>
`;
}
modal.classList.add('active', 'show');
modal.style.setProperty('display', 'flex', 'important');
modal.style.zIndex = '999999';
modal.style.visibility = 'visible';
modal.style.opacity = '1';
await cargarHistorialNovedadesFila();
}
function _f() {
const _c= document.getElementById('modal-editar-fila-novedad');
if (modal) {
modal.classList.remove('active', 'show');
modal.style.setProperty('display', 'none', 'important');
}
}
async function _f() {
if (!_activeEditFilaContext) return;
const _c= document.getElementById('ef-historial-list');
const _c= document.getElementById('ef-hist-count');
const _c= document.getElementById('edit-fila-exp-sync-badge');
const _c= new URLSearchParams();
if (_activeEditFilaContext.origen_tipo) params.set('origen_tipo', _activeEditFilaContext.origen_tipo);
if (_activeEditFilaContext.file_id) params.set('file_id', String(_activeEditFilaContext.file_id));
if (_activeEditFilaContext.sheet_id) params.set('sheet_id', String(_activeEditFilaContext.sheet_id));
if (_activeEditFilaContext.row_number) params.set('row_number', String(_activeEditFilaContext.row_number));
if (_activeEditFilaContext.expediente_id) params.set('expediente_id', String(_activeEditFilaContext.expediente_id));
if (_activeEditFilaContext.suspension_id) params.set('suspension_id', String(_activeEditFilaContext.suspension_id));
if (_activeEditFilaContext.credencial) params.set('credencial', _activeEditFilaContext.credencial);
if (_activeEditFilaContext.numero_expediente) params.set('numero_expediente', _activeEditFilaContext.numero_expediente);
try {
const _c= await fetch('/api/filas/novedades?' + params.toString(), {
headers: { 'Accept': 'application/json' }
});
if (!res.ok) throw new Error('No se pudo obtener el historial de la fila');
const _c= await res.json();
if (data.expediente_id_vinculado) {
_activeEditFilaContext.expediente_id = data.expediente_id_vinculado;
if (syncBadge) {
syncBadge.style.display = 'inline-flex';
syncBadge.textContent = '🔗 Sincronizado con Expediente #' + data.expediente_id_vinculado;
}
} else if (syncBadge) {
syncBadge.style.display = 'none';
}
const _c= data.novedades || [];
const _c= data.versiones || [];
if (countEl) countEl.textContent = `${items.length} novedades · ${versiones.length} versiones`;
if (!listEl) return;
if (items.length === 0 && versiones.length === 0) {
listEl.innerHTML = '<div class="muted" style="padding:10px; background:#f8fafc; border:1px dashed #cbd5e1; border-radius:5px; font-size:12.5px; text-align:center;">Aún no se registraron novedades sobre esta fila. Versión inicial base (v1).</div>';
return;
}
let _l= '';
if (versiones.length > 0) {
html += '<div style="margin-bottom:8px;"><strong style="font-size:12px; color:#475569; text-transform:uppercase;">Historial de Versiones Generadas:</strong></div>';
versiones.forEach(v => {
html += `
<div style="background:#f8fafc; border:1px solid #cbd5e1; border-left:3px solid #2563eb; border-radius:5px; padding:7px 10px; margin-bottom:6px;">
<div style="display:flex; justify-content:space-between; align-items:center; gap:8px;">
<div>
<span class="mc-badge estado-aprobada" style="font-size:11px; font-weight:700;">v${v.numero_version}</span>
<strong style="font-size:12.5px; color:var(--azzen-dark); margin-left:6px;">${escapeHtml(v.titulo)}</strong>
</div>
<span class="muted" style="font-size:11px;">👤 ${escapeHtml(v.usuario)} · ${escapeHtml(v.created_at)}</span>
</div>
<div style="font-size:12px; color:#334155; margin-top:2px;">${escapeHtml(v.descripcion)}</div>
</div>
`;
});
}
if (items.length > 0) {
html += '<div style="margin-top:10px; margin-bottom:8px;"><strong style="font-size:12px; color:#8a2e00; text-transform:uppercase;">Detalle Cronológico de Novedades:</strong></div>';
items.forEach(n => {
html += `
<div style="background:#ffffff; border:1px solid #e2e8f0; border-left:3px solid #E95420; border-radius:5px; padding:9px 12px; margin-bottom:6px;">
<div style="display:flex; justify-content:space-between; align-items:center; gap:8px; margin-bottom:4px; flex-wrap:wrap;">
<div>
<span class="mc-badge estado-entregado" style="font-size:10.5px;">${escapeHtml(n.tipo_novedad)}</span>
<strong style="font-size:13px; color:var(--azzen-dark); margin-left:4px;">${escapeHtml(n.titulo)}</strong>
</div>
<span class="muted" style="font-size:11px;">📅 ${escapeHtml(n.fecha_evento)} · 👤 ${escapeHtml(n.usuario)}</span>
</div>
<div style="font-size:12.5px; color:#334155; white-space:pre-wrap;">${escapeHtml(n.descripcion)}</div>
${n.observaciones ? `<div class="muted" style="font-size:11.5px; margin-top:3px;"><strong>Obs:</strong> ${escapeHtml(n.observaciones)}</div>` : ''}
</div>
`;
});
}
listEl.innerHTML = html;
} catch (err) {
if (listEl) listEl.innerHTML = `<div class="muted" style="font-size:12px;">No se pudieron cargar las novedades previas.</div>`;
}
}
async function _f(e) {
if (e) e.preventDefault();
if (!_activeEditFilaContext) return;
const _c= validateEditarFilaField('titulo');
const _c= validateEditarFilaField('descripcion');
if (!okTitulo || !okDesc) return;
const _c= document.getElementById('ef-btn-submit');
const _c= document.getElementById('edit-fila-alert-box');
if (btn) {
btn.disabled = true;
btn.textContent = '⏳ Guardando novedad y versionando...';
}
const _c= {
origen_tipo: _activeEditFilaContext.origen_tipo,
file_id: _activeEditFilaContext.file_id,
sheet_id: _activeEditFilaContext.sheet_id,
row_number: _activeEditFilaContext.row_number,
expediente_id: _activeEditFilaContext.expediente_id,
suspension_id: _activeEditFilaContext.suspension_id,
credencial: _activeEditFilaContext.credencial,
nombre_agente: _activeEditFilaContext.nombre_agente,
numero_expediente: _activeEditFilaContext.numero_expediente,
tipo_novedad: document.getElementById('ef-tipo-novedad')?.value || 'Observación de fila',
fecha_evento: document.getElementById('ef-fecha-evento')?.value || '',
titulo: (document.getElementById('ef-titulo')?.value || '').trim(),
descripcion: (document.getElementById('ef-descripcion')?.value || '').trim(),
observaciones: (document.getElementById('ef-observaciones')?.value || '').trim(),
};
try {
const _c= await fetch('/api/filas/novedad', {
method: 'POST',
headers: {
'Content-Type': 'application/json',
'Accept': 'application/json'
},
body: JSON.stringify(payload)
});
const _c= await res.json();
if (!res.ok || !data.ok) {
throw new Error(data.error || 'Error al registrar la novedad en la fila.');
}
const _c= data.nueva_version || (_activeEditFilaContext.current_version + 1);
_activeEditFilaContext.current_version = nuevaVersion;
if (alertBox) {
alertBox.style.display = 'block';
alertBox.className = 'alert-success';
alertBox.innerHTML = `<strong>✓ ${escapeHtml(data.mensaje || 'Novedad registrada exitosamente. Se generó la versión v' + nuevaVersion + ' de la fila.')}</strong>`;
}
const _c= document.getElementById('ef-titulo');
const _c= document.getElementById('ef-descripcion');
const _c= document.getElementById('ef-observaciones');
if (tEl) { tEl.value = ''; tEl.classList.remove('azzen-input-valid'); }
if (dEl) { dEl.value = ''; dEl.classList.remove('azzen-input-valid'); }
if (oEl) { oEl.value = ''; }
const _c= document.getElementById('edit-fila-ver-badge');
if (modalVerBadge) modalVerBadge.textContent = 'v' + nuevaVersion;
if (_activeEditFilaContext.tr) {
const _c= _activeEditFilaContext.tr;
tr.setAttribute('data-version', String(nuevaVersion));
const _c= _activeEditFilaContext.row_number;
const _c= document.getElementById('row-ver-badge-' + rowNum);
if (verBadge) {
verBadge.textContent = 'v' + nuevaVersion;
verBadge.className = 'mc-badge estado-aprobada';
}
const _c= parseInt(tr.getAttribute('data-novedades') || '0', 10);
const _c= prevNovCount + 1;
tr.setAttribute('data-novedades', String(newNovCount));
let _l= document.getElementById('row-nov-count-' + rowNum);
if (!novCountBadge && verBadge && verBadge.parentElement) {
novCountBadge = document.createElement('span');
novCountBadge.id = 'row-nov-count-' + rowNum;
novCountBadge.className = 'mc-badge';
novCountBadge.style.cssText = 'background:#E95420; color:#fff; font-size:9.5px; padding:1px 4px; font-weight:700;';
novCountBadge.title = 'Novedades registradas';
verBadge.parentElement.appendChild(novCountBadge);
}
if (novCountBadge) {
novCountBadge.textContent = '📝 ' + newNovCount;
}
}
await cargarHistorialNovedadesFila();
} catch (err) {
if (alertBox) {
alertBox.style.display = 'block';
alertBox.className = 'alert-error';
alertBox.innerHTML = `<strong>❌ Error:</strong> ${escapeHtml(err.message)}`;
}
} finally {
if (btn) {
btn.disabled = false;
btn.textContent = '💾 Guardar Novedad y Generar Nueva Versión';
}
}
}
const _c= {
debounceTimers: new WeakMap(),
initAllForms() {
const _c= document.querySelector('form[action="/expedientes/nuevo"]');
if (newExpForm) {
this.attachToForm(newExpForm, {
entidad: 'expediente',
excludeId: null,
fields: [
'tipo_identificador',
'numero_expediente',
'reparticion',
'reparticion_otro',
'motivo',
'instructor_credencial',
'instructor_actual',
'fecha_inicio',
'fecha_hecho',
'finalizacion',
'fecha_finalizacion'
]
});
}
const _c= document.querySelector('#tab-general form[action^="/expedientes/"]');
if (editExpForm) {
const _c= editExpForm.getAttribute('action').match(/\/expedientes\/(\d+)/);
const _c= m ? parseInt(m[1], 10) : null;
this.attachToForm(editExpForm, {
entidad: 'expediente',
excludeId: excludeId,
fields: [
'tipo_identificador',
'numero_expediente',
'reparticion',
'reparticion_otro',
'motivo',
'instructor_credencial',
'instructor_actual',
'fecha_inicio',
'fecha_hecho',
'finalizacion',
'fecha_finalizacion'
]
});
}
const _c= document.querySelector('#tab-agentes form[action$="/agentes"]');
if (addAgentForm) {
const _c= addAgentForm.getAttribute('action').match(/\/expedientes\/(\d+)\/agentes/);
const _c= m ? parseInt(m[1], 10) : null;
this.attachToForm(addAgentForm, {
entidad: 'agente',
expedienteId: expId,
fields: ['credencial', 'nombre_completo']
});
}
},
attachToForm(form, config) {
if (form.dataset.azzenValidationAttached === 'true') return;
form.dataset.azzenValidationAttached = 'true';
if (config.entidad === 'expediente' && !form.querySelector('input[name="modo_no_bloqueante"]')) {
const _c= document.createElement('input');
hiddenNonBlocking.type = 'hidden';
hiddenNonBlocking.name = 'modo_no_bloqueante';
hiddenNonBlocking.value = 'true';
form.appendChild(hiddenNonBlocking);
}
let _l= form.querySelector('.azzen-validation-summary-panel');
if (!summaryPanel) {
summaryPanel = document.createElement('div');
summaryPanel.className = 'azzen-validation-summary-panel';
summaryPanel.style.display = 'none';
summaryPanel.setAttribute('aria-live', 'polite');
const _c= form.querySelector('button[type="submit"]')?.parentElement;
if (submitRow && submitRow.parentElement === form) {
form.insertBefore(summaryPanel, submitRow);
} else {
form.prepend(summaryPanel);
}
}
const _c= new Set();
let _l= { valido: true, errores: [], advertencias: [], informacion: [] };
config.fields.forEach(fieldName => {
const _c= form.querySelector(`[name="${fieldName}"]`);
if (!input) return;
let _l= input.parentElement.querySelector(`.azzen-field-feedback[data-for="${fieldName}"]`);
if (!feedbackEl) {
feedbackEl = document.createElement('div');
feedbackEl.className = 'azzen-field-feedback';
feedbackEl.setAttribute('data-for', fieldName);
input.parentElement.appendChild(feedbackEl);
}
const _c= (immediate = false) => {
touchedFields.add(fieldName);
if (fieldName === 'tipo_identificador') touchedFields.add('numero_expediente');
if (fieldName === 'fecha_inicio' || fieldName === 'fecha_hecho' || fieldName === 'fecha_finalizacion') {
touchedFields.add('fecha_inicio');
touchedFields.add('fecha_hecho');
touchedFields.add('fecha_finalizacion');
}
const _c= this.debounceTimers.get(form);
if (prevTimer) clearTimeout(prevTimer);
if (immediate) {
this.runServerValidation(form, config, touchedFields, summaryPanel, state => { latestState = state; });
} else {
this.showFieldLoading(feedbackEl);
const _c= setTimeout(() => {
this.runServerValidation(form, config, touchedFields, summaryPanel, state => { latestState = state; });
}, 260);
this.debounceTimers.set(form, timer);
}
};
input.addEventListener('input', () => triggerValidation(false));
input.addEventListener('change', () => triggerValidation(true));
input.addEventListener('blur', () => {
if ((input.value || '').trim() !== '' || touchedFields.has(fieldName)) {
triggerValidation(true);
}
});
});
form.addEventListener('submit', async (e) => {
config.fields.forEach(f => touchedFields.add(f));
const _c= await this.runServerValidation(form, config, touchedFields, summaryPanel, s => { latestState = s; });
if (state && state.errores && state.errores.length > 0) {
e.preventDefault();
const _c= state.errores[0].campo;
const _c= form.querySelector(`[name="${firstErrField}"]`);
if (firstInput) firstInput.focus();
summaryPanel.scrollIntoView({ behavior: 'smooth', block: 'center' });
}
});
},
showFieldLoading(feedbackEl) {
if (!feedbackEl) return;
if (!feedbackEl.querySelector('.azzen-feedback-loading')) {
const _c= document.createElement('div');
loader.className = 'azzen-feedback-loading';
loader.innerHTML = '<span>⏳</span><span>Verificando disponibilidad y vinculaciones...</span>';
feedbackEl.appendChild(loader);
}
},
async runServerValidation(form, config, touchedFields, summaryPanel, onUpdate) {
const _c= new URLSearchParams();
params.set('entidad', config.entidad || 'expediente');
params.set('campo', 'todos');
if (config.excludeId) params.set('exclude_id', String(config.excludeId));
if (config.expedienteId) params.set('expediente_id', String(config.expedienteId));
config.fields.forEach(fieldName => {
const _c= form.querySelector(`[name="${fieldName}"]`);
if (el) {
params.set(fieldName, el.value || '');
}
});
try {
const _c= await fetch('/api/validaciones/verificar?' + params.toString(), {
headers: { 'Accept': 'application/json' }
});
if (!res.ok) return null;
const _c= await res.json();
if (onUpdate) onUpdate(data);
this.renderFormFeedback(form, config, touchedFields, summaryPanel, data);
return data;
} catch (err) {
return null;
}
},
renderFormFeedback(form, config, touchedFields, summaryPanel, data) {
const _c= (data.errores || []).filter(item => touchedFields.has(item.campo));
const _c= (data.advertencias || []).filter(item => touchedFields.has(item.campo));
const _c= (data.informacion || []).filter(item => touchedFields.has(item.campo));
config.fields.forEach(fieldName => {
const _c= form.querySelector(`[name="${fieldName}"]`);
const _c= input?.parentElement?.querySelector(`.azzen-field-feedback[data-for="${fieldName}"]`);
if (!input || !feedbackEl) return;
input.classList.remove('azzen-input-error', 'azzen-input-warning', 'azzen-input-info', 'azzen-input-valid');
feedbackEl.innerHTML = '';
if (!touchedFields.has(fieldName)) return;
const _c= errores.filter(x => x.campo === fieldName);
const _c= advertencias.filter(x => x.campo === fieldName);
const _c= informacion.filter(x => x.campo === fieldName);
if (fieldErrors.length > 0) {
input.classList.add('azzen-input-error');
fieldErrors.forEach(err => {
const _c= document.createElement('div');
div.className = 'azzen-feedback-item is-error';
div.setAttribute('role', 'alert');
div.innerHTML = `
<span>❌</span>
<div>
<strong>Error (Bloqueante):</strong> ${escapeHtml(err.mensaje)}
${err.url ? ` <a href="${escapeHtml(err.url)}" target="_blank" style="color:#991b1b; font-weight:700; text-decoration:underline; margin-left:4px;">Ver registro existente ↗</a>` : ''}
</div>
`;
feedbackEl.appendChild(div);
});
} else if (fieldWarns.length > 0) {
input.classList.add('azzen-input-warning');
fieldWarns.forEach(w => {
const _c= document.createElement('div');
div.className = 'azzen-feedback-item is-warning';
div.innerHTML = `
<span>⚠️</span>
<div>
<strong>Advertencia — Posible duplicado (No bloquea el guardado):</strong> ${escapeHtml(w.mensaje)}
<div style="font-size:11px; opacity:0.9; margin-top:2px;">Motivo de detección: ${escapeHtml(w.motivo_coincidencia || '')}</div>
${w.url ? `<div style="margin-top:3px;"><a href="${escapeHtml(w.url)}" target="_blank" style="color:#92400e; font-weight:700; text-decoration:underline;">🔍 Consultar registro existente ↗</a></div>` : ''}
</div>
`;
feedbackEl.appendChild(div);
});
} else if (fieldInfos.length > 0) {
input.classList.add('azzen-input-info');
fieldInfos.slice(0, 2).forEach(inf => {
const _c= document.createElement('div');
div.className = 'azzen-feedback-item is-info';
let _l= '';
if (inf.subtipo === 'padron_persona' && inf.nombre_detectado) {
const _c= fieldName === 'instructor_credencial' ? 'instructor_actual' : 'nombre_completo';
autoFillBtn = ` <button type="button" class="button is-dark is-outlined btn-xs" style="padding:1px 6px; font-size:10.5px; margin-left:6px;" onclick="AzzenValidationEngine.fillNameFromPadron(this, '${targetNameField}', '${escapeHtml(inf.nombre_detectado)}')">Usar nombre del Padrón</button>`;
}
div.innerHTML = `
<span>ℹ️</span>
<div>
<strong>Información / Vinculación sugerida:</strong> ${escapeHtml(inf.mensaje)}
${autoFillBtn}
${inf.url ? ` <a href="${escapeHtml(inf.url)}" target="_blank" style="color:#075985; font-weight:700; text-decoration:underline; margin-left:4px;">Ver antecedente ↗</a>` : ''}
</div>
`;
feedbackEl.appendChild(div);
});
} else if ((input.value || '').trim() !== '') {
input.classList.add('azzen-input-valid');
}
});
if (!summaryPanel) return;
if (errores.length === 0 && advertencias.length === 0 && informacion.length === 0) {
summaryPanel.style.display = 'none';
summaryPanel.innerHTML = '';
return;
}
let _l= '';
if (errores.length > 0) {
panelHtml += `
<div class="azzen-val-group group-error" role="alert">
<div class="azzen-val-group-title" style="color:#991b1b;">
<span>❌ Errores de Integridad o Formato (${errores.length}) — Requieren corrección antes de guardar</span>
<span class="mc-badge estado-rechazada">Bloqueante</span>
</div>
<ul class="azzen-val-list">
${errores.map(e => `
<li>
<span>• <strong>[${escapeHtml(e.campo)}]</strong> ${escapeHtml(e.mensaje)}</span>
${e.url ? `<a href="${escapeHtml(e.url)}" target="_blank" class="button is-dark is-outlined btn-sm" style="font-size:11px; padding:2px 8px;">Abrir registro existente ↗</a>` : ''}
</li>
`).join('')}
</ul>
</div>
`;
}
if (advertencias.length > 0) {
panelHtml += `
<div class="azzen-val-group group-warning">
<div class="azzen-val-group-title" style="color:#92400e;">
<span>⚠️ Posibles Duplicados Detectados (${advertencias.length}) — Informativo, no bloquea el guardado</span>
<span class="mc-badge estado-pendiente">Advertencia No Bloqueante</span>
</div>
<ul class="azzen-val-list">
${advertencias.map(w => `
<li>
<div>
<div>• ${escapeHtml(w.mensaje)}</div>
<div class="muted" style="font-size:11.5px;">Motivo de coincidencia: ${escapeHtml(w.motivo_coincidencia || '')}</div>
</div>
${w.url ? `<a href="${escapeHtml(w.url)}" target="_blank" class="button is-dark is-outlined btn-sm" style="font-size:11px; padding:2px 8px;">🔍 Inspeccionar registro ↗</a>` : ''}
</li>
`).join('')}
</ul>
</div>
`;
}
if (informacion.length > 0) {
panelHtml += `
<div class="azzen-val-group group-info">
<div class="azzen-val-group-title" style="color:#075985;">
<span>ℹ️ Relaciones Potenciales y Sugerencias de Vinculación (${informacion.length})</span>
<span class="mc-badge estado-entregado">Información / Sugerencia</span>
</div>
<ul class="azzen-val-list">
${informacion.map(inf => {
let _l= '';
if (inf.subtipo === 'vinculacion_expediente_agente' && inf.registro_id && typeof sugerirVinculacion === 'function' && !inf.ya_vinculado) {
actionBtn = `<button type="button" class="button is-azzen btn-sm" style="font-size:11px; padding:2px 8px;" onclick="sugerirVinculacion('${inf.registro_id}', '${escapeHtml(inf.identificador || '')}')">🔗 Pre-cargar vinculación</button>`;
}
return `
<li>
<div>
<div>• ${escapeHtml(inf.mensaje)}</div>
<div class="muted" style="font-size:11.5px;">Motivo de coincidencia: ${escapeHtml(inf.motivo_coincidencia || '')}</div>
</div>
<div style="display:flex; gap:6px; align-items:center;">
${actionBtn}
${inf.url ? `<a href="${escapeHtml(inf.url)}" target="_blank" class="button is-dark is-outlined btn-sm" style="font-size:11px; padding:2px 8px;">Ver antecedente ↗</a>` : ''}
</div>
</li>
`;
}).join('')}
</ul>
</div>
`;
}
summaryPanel.innerHTML = panelHtml;
summaryPanel.style.display = 'block';
},
fillNameFromPadron(btn, targetFieldName, detectedName) {
const _c= btn.closest('form');
if (!form) return;
const _c= form.querySelector(`[name="${targetFieldName}"]`);
if (targetInput) {
targetInput.value = detectedName;
targetInput.dispatchEvent(new Event('change', { bubbles: true }));
}
}
};
window.currentTargetPrefix = 'involucrado';
window.setupPredictivoAgente = function(credInputId, apelInputId, nomInputId, escInputId, grdInputId, dropdownId, alertaId, targetPrefix) {
const _c= document.getElementById(credInputId);
const _c= document.getElementById(apelInputId);
const _c= document.getElementById(nomInputId);
const _c= document.getElementById(escInputId);
const _c= document.getElementById(grdInputId);
const _c= document.getElementById(dropdownId);
const _c= document.getElementById(alertaId);
if (!credInput || !dropdown) return;
let _l= null;
credInput.addEventListener('input', function() {
const _c= credInput.value.trim();
clearTimeout(debounceTimer);
if (alerta) alerta.style.display = 'none';
if (val.length < 2) {
dropdown.style.display = 'none';
dropdown.innerHTML = '';
return;
}
debounceTimer = setTimeout(async () => {
try {
dropdown.innerHTML = '<div class="autocomplete-loading">🔍 Buscando en padrón central...</div>';
dropdown.style.display = 'block';
const _c= await fetch(`/api/personas/buscar?q=${encodeURIComponent(val)}`);
const _c= await res.json();
const _c= Array.isArray(jsonResp) ? jsonResp : (jsonResp.personas || []);
if (!Array.isArray(personas) || personas.length === 0) {
dropdown.style.display = 'none';
if (alerta) {
alerta.style.display = 'flex';
}
return;
}
dropdown.innerHTML = '';
personas.forEach(p => {
const _c= document.createElement('div');
item.className = 'autocomplete-item';
const _c= [p.escalafon, p.grado].filter(Boolean).join(' - ') || 'Sin escalafón asignado';
item.innerHTML = `<strong>${p.credencial} – ${p.apellido}, ${p.nombre}</strong><span style="font-size:12px; color:#6b7280;">${escGrd}</span>`;
item.addEventListener('click', () => {
credInput.value = p.credencial;
if (apelInput) apelInput.value = p.apellido || '';
if (nomInput) nomInput.value = p.nombre || '';
if (escInput) escInput.value = p.escalafon || '';
if (grdInput) grdInput.value = p.grado || '';
dropdown.style.display = 'none';
if (alerta) alerta.style.display = 'none';
});
dropdown.appendChild(item);
});
} catch (err) {
console.error('Error buscando agente:', err);
dropdown.style.display = 'none';
}
}, 250);
});
document.addEventListener('click', (e) => {
if (!credInput.contains(e.target) && !dropdown.contains(e.target)) {
dropdown.style.display = 'none';
}
});
};
window.abrirModalNuevoAgente = function(prefix) {
window.currentTargetPrefix = prefix;
const _c= document.getElementById('modal-nuevo-agente');
const _c= document.getElementById('modal-agente-error');
if (errBox) errBox.style.display = 'none';
let _l= '';
if (prefix === 'involucrado') {
const _c= document.getElementById('involucrado_credencial');
if (inp) currentVal = inp.value.trim();
} else if (prefix === 'ag') {
const _c= document.getElementById('ag_credencial');
if (inp) currentVal = inp.value.trim();
}
const _c= document.getElementById('modal_credencial');
if (modalCred) modalCred.value = currentVal;
const _c= document.getElementById('modal_apellido');
if (modalApe) modalApe.value = '';
const _c= document.getElementById('modal_nombre');
if (modalNom) modalNom.value = '';
const _c= document.getElementById('modal_escalafon');
if (modalEsc) modalEsc.value = '';
const _c= document.getElementById('modal_grado');
if (modalGrd) modalGrd.value = '';
if (modal) modal.style.display = 'flex';
if (modalCred) modalCred.focus();
};
window.cerrarModalNuevoAgente = function() {
const _c= document.getElementById('modal-nuevo-agente');
if (modal) modal.style.display = 'none';
};
window.guardarNuevoAgenteExpress = async function(e) {
e.preventDefault();
const _c= document.getElementById('btn-modal-guardar-agente');
const _c= document.getElementById('modal-agente-error');
if (errBox) errBox.style.display = 'none';
const _c= document.getElementById('modal_credencial').value.trim();
const _c= document.getElementById('modal_apellido').value.trim();
const _c= document.getElementById('modal_nombre').value.trim();
const _c= document.getElementById('modal_escalafon').value.trim();
const _c= document.getElementById('modal_grado').value.trim();
if (!credencial || !apellido || !nombre) {
if (errBox) {
errBox.textContent = 'Credencial, apellido y nombre son obligatorios.';
errBox.style.display = 'block';
}
return;
}
try {
if (btn) {
btn.disabled = true;
btn.textContent = 'Guardando...';
}
const _c= await fetch('/api/personas/express', {
method: 'POST',
headers: { 'Content-Type': 'application/json' },
body: JSON.stringify({ credencial, apellido, nombre, escalafon, grado })
});
const _c= await resp.json();
if (!data.success) {
throw new Error(data.message || 'Error registrando agente');
}
if (window.currentTargetPrefix === 'involucrado') {
const _c= document.getElementById('involucrado_credencial');
const _c= document.getElementById('involucrado_apellido');
const _c= document.getElementById('involucrado_nombre');
const _c= document.getElementById('involucrado_escalafon');
const _c= document.getElementById('involucrado_grado');
if (c) c.value = credencial;
if (a) a.value = apellido;
if (n) n.value = nombre;
if (es) es.value = escalafon;
if (g) g.value = grado;
const _c= document.getElementById('alerta-involucrado-no-encontrado');
if (alerta) alerta.style.display = 'none';
} else if (window.currentTargetPrefix === 'ag') {
const _c= document.getElementById('ag_credencial');
const _c= document.getElementById('ag_apellido');
const _c= document.getElementById('ag_nombre');
const _c= document.getElementById('ag_escalafon');
const _c= document.getElementById('ag_grado');
if (c) c.value = credencial;
if (a) a.value = apellido;
if (n) n.value = nombre;
if (es) es.value = escalafon;
if (g) g.value = grado;
const _c= document.getElementById('alerta-ag-no-encontrado');
if (alerta) alerta.style.display = 'none';
}
window.cerrarModalNuevoAgente();
} catch (err) {
if (errBox) {
errBox.textContent = err.message || 'Ocurrió un error al registrar el agente.';
errBox.style.display = 'block';
}
} finally {
if (btn) {
btn.disabled = false;
btn.textContent = 'Guardar e Incorporar Agente';
}
}
};