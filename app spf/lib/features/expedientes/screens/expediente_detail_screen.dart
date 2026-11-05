import 'package:flutter/cupertino.dart';
import '../../../core/models/expediente.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_badge.dart';
import '../../../shared/widgets/azzen_button.dart';
import '../../../shared/widgets/azzen_card.dart';
import '../../../shared/widgets/azzen_divider.dart';
import '../../personas/screens/persona_detail_screen.dart';
import '../../suspensiones/screens/suspension_detail_screen.dart';
import 'nueva_novedad_screen.dart';
class ExpedienteDetailScreen extends StatefulWidget {
  final int expedienteId;
  const ExpedienteDetailScreen({super.key, required this.expedienteId});
  @override
  State<ExpedienteDetailScreen> createState() => clase_2001();
}
class clase_2001 extends State<ExpedienteDetailScreen> {
  ExpedienteDetalle? v_4001;
  bool v_4002 = true;
  String? v_4003;
  int v_4004 = 0;
  @override
  void initState() {
    super.initState();
    m_3001();
  }
  Future<void> m_3001() async {
    setState(() {
      v_4002 = true;
      v_4003 = null;
    });
    try {
      final data = await ApiClient.get('/api/mobile/expedientes/${widget.expedienteId}');
      if (mounted) {
        setState(() {
          v_4001 = ExpedienteDetalle.fromJson(data);
          v_4002 = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          v_4003 = e.toString();
          v_4002 = false;
        });
      }
    }
  }
  void m_3002() {
    if (v_4001 == null) return;
    final estados = ['Abierto', 'En trámite', 'Dictamen', 'Resolución', 'Archivado'];
    String nuevoEstado = estados.firstWhere((e) => e != v_4001!.estado, orElse: () => 'En trámite');
    final motivoCtrl = TextEditingController();
    showCupertinoDialog(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (context, setDialogState) => CupertinoAlertDialog(
          title: const Text('Transición de Estado'),
          content: Column(
            children: [
              const SizedBox(height: 8),
              Text(
                'Cambiar estado del expediente ${v_4001!.numeroExpediente} (Actual: ${v_4001!.estado}).',
                style: const TextStyle(fontSize: 12),
              ),
              const SizedBox(height: 12),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                decoration: BoxDecoration(
                  color: AzzenColors.surfaceSecondary,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: AzzenColors.border),
                ),
                child: CupertinoButton(
                  padding: EdgeInsets.zero,
                  child: Text('Nuevo estado: $nuevoEstado', style: AzzenTypography.bodyBold),
                  onPressed: () {
                    showCupertinoModalPopup(
                      context: context,
                      builder: (sheetCtx) => CupertinoActionSheet(
                        title: const Text('Seleccione nuevo estado'),
                        actions: estados.map((e) {
                          return CupertinoActionSheetAction(
                            child: Text(e),
                            onPressed: () {
                              setDialogState(() {
                                nuevoEstado = e;
                              });
                              Navigator.pop(sheetCtx);
                            },
                          );
                        }).toList(),
                      ),
                    );
                  },
                ),
              ),
              const SizedBox(height: 10),
              CupertinoTextField(
                controller: motivoCtrl,
                placeholder: 'Motivo y justificación del cambio...',
                maxLines: 3,
                style: const TextStyle(fontSize: 13),
              ),
            ],
          ),
          actions: [
            CupertinoDialogAction(
              child: const Text('Cancelar'),
              onPressed: () => Navigator.pop(ctx),
            ),
            CupertinoDialogAction(
              isDestructiveAction: nuevoEstado == 'Archivado',
              child: const Text('Confirmar Transición'),
              onPressed: () async {
                final mot = motivoCtrl.text.trim();
                if (mot.isEmpty) return;
                Navigator.pop(ctx);
                try {
                  await ApiClient.post(
                    '/api/mobile/expedientes/${widget.expedienteId}/transicion',
                    body: {
                      'nuevo_estado': nuevoEstado,
                      'motivo': mot,
                    },
                  );
                  m_3001();
                } catch (e) {
                  if (!context.mounted) return;
                  showCupertinoDialog(
                    context: context,
                    builder: (errCtx) => CupertinoAlertDialog(
                      title: const Text('Error de Transición'),
                      content: Text(e.toString()),
                      actions: [
                        CupertinoDialogAction(
                          child: const Text('Aceptar'),
                          onPressed: () => Navigator.pop(errCtx),
                        ),
                      ],
                    ),
                  );
                }
              },
            ),
          ],
        ),
      ),
    );
  }
  @override
  Widget build(BuildContext context) {
    if (v_4002) {
      return const CupertinoPageScaffold(
        navigationBar: CupertinoNavigationBar(middle: Text('Cargando expediente...')),
        child: Center(child: CupertinoActivityIndicator(radius: 14)),
      );
    }
    if (v_4003 != null || v_4001 == null) {
      return CupertinoPageScaffold(
        navigationBar: const CupertinoNavigationBar(middle: Text('Expediente')),
        child: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(CupertinoIcons.exclamationmark_triangle, size: 40, color: AzzenColors.danger),
                const SizedBox(height: 8),
                Text(v_4003 ?? 'No encontrado', textAlign: TextAlign.center, style: AzzenTypography.caption),
                const SizedBox(height: 12),
                CupertinoButton(onPressed: m_3001, child: const Text('Reintentar')),
              ],
            ),
          ),
        ),
      );
    }
    final d = v_4001!;
    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text('Exp. ${d.numeroExpediente}'),
        trailing: CupertinoButton(
          padding: EdgeInsets.zero,
          child: const Icon(CupertinoIcons.plus_circle, size: 22),
          onPressed: () async {
            final res = await Navigator.push(
              context,
              CupertinoPageRoute(
                builder: (_) => NuevaNovedadScreen(
                  expedienteId: d.id,
                  numeroExpediente: d.numeroExpediente,
                ),
              ),
            );
            if (res == true) {
              m_3001();
            }
          },
        ),
      ),
      child: SafeArea(
        child: Column(
          children: [
            Container(
              color: AzzenColors.surfaceSecondary,
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Expanded(
                        child: Text(
                          d.numeroExpediente,
                          style: AzzenTypography.header2,
                        ),
                      ),
                      AzzenBadge.fromStatus(d.estado),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(
                    '${d.reparticion} · Tipo: ${d.tipoIdentificador.toUpperCase()} · Versión v${d.versionActual}',
                    style: AzzenTypography.caption,
                  ),
                  const SizedBox(height: 12),
                  SizedBox(
                    width: double.infinity,
                    child: CupertinoSlidingSegmentedControl<int>(
                      groupValue: v_4004,
                      children: {
                        0: const Padding(padding: EdgeInsets.symmetric(horizontal: 4), child: Text('Carátula')),
                        1: Padding(padding: const EdgeInsets.symmetric(horizontal: 4), child: Text('Agentes (${d.agentes.length})')),
                        2: Padding(padding: const EdgeInsets.symmetric(horizontal: 4), child: Text('Susp. (${d.suspensiones.length})')),
                        3: Padding(padding: const EdgeInsets.symmetric(horizontal: 4), child: Text('Novedades (${d.novedades.length})')),
                        4: Padding(padding: const EdgeInsets.symmetric(horizontal: 4), child: Text('Docs (${d.documentos.length})')),
                      },
                      onValueChanged: (val) {
                        if (val != null) {
                          setState(() {
                            v_4004 = val;
                          });
                        }
                      },
                    ),
                  ),
                ],
              ),
            ),
            Expanded(
              child: m_3003(d),
            ),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: const BoxDecoration(
                color: AzzenColors.background,
                border: Border(top: BorderSide(color: AzzenColors.border)),
              ),
              child: Row(
                children: [
                  Expanded(
                    child: AzzenButton(
                      label: 'Transición Estado',
                      icon: CupertinoIcons.arrow_2_squarepath,
                      variant: AzzenButtonVariant.secondary,
                      height: 42,
                      onPressed: m_3002,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: AzzenButton(
                      label: 'Nueva Novedad',
                      icon: CupertinoIcons.plus,
                      height: 42,
                      onPressed: () async {
                        final res = await Navigator.push(
                          context,
                          CupertinoPageRoute(
                            builder: (_) => NuevaNovedadScreen(
                              expedienteId: d.id,
                              numeroExpediente: d.numeroExpediente,
                            ),
                          ),
                        );
                        if (res == true) {
                          m_3001();
                        }
                      },
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
  Widget m_3003(ExpedienteDetalle d) {
    switch (v_4004) {
      case 0:
        return m_3004(d);
      case 1:
        return m_3005(d);
      case 2:
        return m_3006(d);
      case 3:
        return m_3007(d);
      case 4:
        return m_3008(d);
      default:
        return const SizedBox.shrink();
    }
  }
  Widget m_3004(ExpedienteDetalle d) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        AzzenCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Motivo / Carátula Oficial', style: AzzenTypography.captionBold),
              const SizedBox(height: 6),
              Text(d.motivo ?? 'Sin carátula registrada', style: AzzenTypography.body),
              if (d.relatoHechos != null && d.relatoHechos!.isNotEmpty) ...[
                const SizedBox(height: 12),
                const Text('Relato de los Hechos', style: AzzenTypography.captionBold),
                const SizedBox(height: 4),
                Text(d.relatoHechos!, style: AzzenTypography.body),
              ],
            ],
          ),
        ),
        AzzenCard(
          child: Column(
            children: [
              m_3009('Instructor actual', d.instructorActual ?? 'Sin instructor asignado'),
              const AzzenDivider(),
              m_3009('Fecha de inicio', d.fechaInicio ?? '—'),
              const AzzenDivider(),
              m_3009('Fecha del hecho', d.fechaHecho ?? '—'),
              const AzzenDivider(),
              m_3009('Autoridad ordenante', d.autoridadOrdenante ?? '—'),
              const AzzenDivider(),
              m_3009('Dependencia', d.autoridadDependencia ?? '—'),
              if (d.articulo != null) ...[
                const AzzenDivider(),
                m_3009('Artículo / Inciso', 'Art. ${d.articulo} ${d.inciso ?? ""}'),
              ],
            ],
          ),
        ),
        if (d.versiones.isNotEmpty) ...[
          Padding(
            padding: const EdgeInsets.only(top: 8, bottom: 6),
            child: Text('HISTORIAL DE VERSIONES AUDITADAS', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy)),
          ),
          ...d.versiones.map((v) => AzzenCard(
            margin: const EdgeInsets.only(bottom: 8),
            child: Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: AzzenColors.primaryLight,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text('v${v.numeroVersion}', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.primary)),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(v.motivo.isNotEmpty ? v.motivo : v.tipoOperacion, style: AzzenTypography.bodyBold),
                      Text('Por ${v.usuario} · ${v.fecha}', style: AzzenTypography.caption),
                    ],
                  ),
                ),
              ],
            ),
          )),
        ],
      ],
    );
  }
  Widget m_3005(ExpedienteDetalle d) {
    if (d.agentes.isEmpty) {
      return const Center(child: Text('No hay agentes vinculados a este expediente.', style: TextStyle(color: AzzenColors.textMuted)));
    }
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: d.agentes.length,
      itemBuilder: (ctx, i) {
        final a = d.agentes[i];
        return AzzenCard(
          onTap: () {
            Navigator.push(
              context,
              CupertinoPageRoute(
                builder: (_) => PersonaDetailScreen(credencial: a.credencial),
              ),
            );
          },
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text('Cred. ${a.credencial}', style: AzzenTypography.bodyBold),
                  AzzenBadge.fromStatus(a.rol),
                ],
              ),
              const SizedBox(height: 6),
              Text(a.nombreCompleto, style: AzzenTypography.body),
              const SizedBox(height: 4),
              Text(
                'Situación: ${a.situacionProcesal}${a.escalafon != null ? " · ${a.escalafon}" : ""}',
                style: AzzenTypography.caption,
              ),
              if (a.suspensionPreventiva) ...[
                const SizedBox(height: 6),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  decoration: BoxDecoration(
                    color: AzzenColors.dangerBackground,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'Posee medida de Suspensión Preventiva',
                    style: AzzenTypography.captionBold.copyWith(color: AzzenColors.dangerText, fontSize: 11),
                  ),
                ),
              ],
            ],
          ),
        );
      },
    );
  }
  Widget m_3006(ExpedienteDetalle d) {
    if (d.suspensiones.isEmpty) {
      return const Center(child: Text('No hay suspensiones registradas en este expediente.', style: TextStyle(color: AzzenColors.textMuted)));
    }
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: d.suspensiones.length,
      itemBuilder: (ctx, i) {
        final s = d.suspensiones[i];
        return AzzenCard(
          onTap: () {
            Navigator.push(
              context,
              CupertinoPageRoute(
                builder: (_) => SuspensionDetailScreen(suspensionId: s.id),
              ),
            );
          },
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Text('Cred. ${s.credencial}', style: AzzenTypography.bodyBold),
                        const SizedBox(width: 8),
                        AzzenBadge.fromStatus(s.estado),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(s.nombreCompleto, style: AzzenTypography.body),
                    if (s.articulo419Incisos.isNotEmpty) ...[
                      const SizedBox(height: 2),
                      Text('Art. 419: ${s.articulo419Incisos}', style: AzzenTypography.caption),
                    ],
                    if (s.fechaInicio != null)
                      Text('Inicio: ${s.fechaInicio}${s.fechaFin != null ? " · Cese: ${s.fechaFin}" : ""}', style: AzzenTypography.caption),
                  ],
                ),
              ),
              const Icon(CupertinoIcons.chevron_forward, size: 16, color: AzzenColors.textMuted),
            ],
          ),
        );
      },
    );
  }
  Widget m_3007(ExpedienteDetalle d) {
    if (d.novedades.isEmpty) {
      return const Center(child: Text('No hay novedades registradas en este expediente.', style: TextStyle(color: AzzenColors.textMuted)));
    }
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: d.novedades.length,
      itemBuilder: (ctx, i) {
        final n = d.novedades[i];
        return AzzenCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Expanded(child: Text(n.titulo, style: AzzenTypography.bodyBold)),
                  Text(n.fecha, style: AzzenTypography.caption.copyWith(fontSize: 11)),
                ],
              ),
              const SizedBox(height: 2),
              Text(n.tipoNovedad, style: AzzenTypography.captionBold.copyWith(color: AzzenColors.primary)),
              const SizedBox(height: 6),
              Text(n.descripcion, style: AzzenTypography.body),
              const SizedBox(height: 6),
              Text('Registrado por: ${n.usuario}', style: AzzenTypography.caption.copyWith(color: AzzenColors.textMuted)),
            ],
          ),
        );
      },
    );
  }
  Widget m_3008(ExpedienteDetalle d) {
    if (d.documentos.isEmpty) {
      return const Center(child: Text('No hay documentos adjuntos.', style: TextStyle(color: AzzenColors.textMuted)));
    }
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: d.documentos.length,
      itemBuilder: (ctx, i) {
        final doc = d.documentos[i];
        return AzzenCard(
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: AzzenColors.primaryLight,
                  borderRadius: BorderRadius.circular(6),
                ),
                child: const Icon(CupertinoIcons.doc_fill, size: 22, color: AzzenColors.primary),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(doc.nombreOriginal, style: AzzenTypography.bodyBold, maxLines: 1, overflow: TextOverflow.ellipsis),
                    const SizedBox(height: 2),
                    Text('${doc.tipoDocumento} · v${doc.version}', style: AzzenTypography.caption),
                    Text('Subido: ${doc.fecha}', style: AzzenTypography.caption.copyWith(color: AzzenColors.textMuted)),
                  ],
                ),
              ),
              const Icon(CupertinoIcons.arrow_down_doc, size: 18, color: AzzenColors.navy),
            ],
          ),
        );
      },
    );
  }
  Widget m_3009(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(width: 130, child: Text(label, style: AzzenTypography.captionBold)),
          Expanded(child: Text(value, style: AzzenTypography.body)),
        ],
      ),
    );
  }
}