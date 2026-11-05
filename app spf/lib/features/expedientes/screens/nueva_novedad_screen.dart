import 'package:flutter/cupertino.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_button.dart';
import '../../../shared/widgets/azzen_text_field.dart';
class NuevaNovedadScreen extends StatefulWidget {
  final int expedienteId;
  final String numeroExpediente;
  const NuevaNovedadScreen({
    super.key,
    required this.expedienteId,
    required this.numeroExpediente,
  });
  @override
  State<NuevaNovedadScreen> createState() => clase_2001();
}
class clase_2001 extends State<NuevaNovedadScreen> {
  final v_4001 = TextEditingController();
  final v_4002 = TextEditingController();
  String v_4003 = 'Actuación administrativa';
  bool v_4004 = false;
  final List<String> v_4005 = [
    'Actuación administrativa',
    'Disposición de sumario / información sumaria',
    'Dictamen instructor',
    'Notificación de imputación',
    'Prueba agregada',
    'Resolución conclusiva',
    'Observación de auditoría',
  ];
  @override
  void dispose() {
    v_4001.dispose();
    v_4002.dispose();
    super.dispose();
  }
  void m_3001() async {
    final tit = v_4001.text.trim();
    final desc = v_4002.text.trim();
    if (tit.isEmpty || desc.isEmpty) {
      showCupertinoDialog(
        context: context,
        builder: (ctx) => CupertinoAlertDialog(
          title: const Text('Campos obligatorios'),
          content: const Text('Debe ingresar un título y descripción para la novedad administrativa.'),
          actions: [
            CupertinoDialogAction(
              child: const Text('Aceptar'),
              onPressed: () => Navigator.pop(ctx),
            ),
          ],
        ),
      );
      return;
    }
    setState(() {
      v_4004 = true;
    });
    try {
      await ApiClient.post(
        '/api/mobile/expedientes/${widget.expedienteId}/novedades',
        body: {
          'titulo': tit,
          'descripcion': desc,
          'tipo_novedad': v_4003,
        },
      );
      if (!mounted) return;
      showCupertinoDialog(
        context: context,
        builder: (ctx) => CupertinoAlertDialog(
          title: const Text('Novedad Registrada'),
          content: const Text('La actuación ha sido guardada en el backend central y se ha emitido la notificación correspondiente.'),
          actions: [
            CupertinoDialogAction(
              child: const Text('Aceptar'),
              onPressed: () {
                Navigator.pop(ctx);
                Navigator.pop(context, true);
              },
            ),
          ],
        ),
      );
    } catch (e) {
      if (!mounted) return;
      setState(() {
        v_4004 = false;
      });
      showCupertinoDialog(
        context: context,
        builder: (ctx) => CupertinoAlertDialog(
          title: const Text('Error al registrar'),
          content: Text(e.toString()),
          actions: [
            CupertinoDialogAction(
              child: const Text('Aceptar'),
              onPressed: () => Navigator.pop(ctx),
            ),
          ],
        ),
      );
    }
  }
  @override
  Widget build(BuildContext context) {
    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text('Novedad: Exp. ${widget.numeroExpediente}'),
      ),
      child: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                'Registrar Actuación Administrativa',
                style: AzzenTypography.header2,
              ),
              const SizedBox(height: 6),
              Text(
                'Esta operación generará un registro inmutable en el historial del expediente #${widget.expedienteId} y se auditará con su firma digital.',
                style: AzzenTypography.caption,
              ),
              const SizedBox(height: 24),
              Text('Tipo de Actuación', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy)),
              const SizedBox(height: 6),
              GestureDetector(
                onTap: () {
                  showCupertinoModalPopup(
                    context: context,
                    builder: (ctx) => CupertinoActionSheet(
                      title: const Text('Seleccione tipo de actuación'),
                      actions: v_4005.map((t) {
                        return CupertinoActionSheetAction(
                          child: Text(t),
                          onPressed: () {
                            setState(() {
                              v_4003 = t;
                            });
                            Navigator.pop(ctx);
                          },
                        );
                      }).toList(),
                      cancelButton: CupertinoActionSheetAction(
                        isDefaultAction: true,
                        child: const Text('Cancelar'),
                        onPressed: () => Navigator.pop(ctx),
                      ),
                    ),
                  );
                },
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
                  decoration: BoxDecoration(
                    color: AzzenColors.surfaceSecondary,
                    borderRadius: BorderRadius.circular(10),
                    border: Border.all(color: AzzenColors.border),
                  ),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Expanded(
                        child: Text(
                          v_4003,
                          style: AzzenTypography.body,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      const Icon(CupertinoIcons.chevron_down, size: 16, color: AzzenColors.textMuted),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),
              AzzenTextField(
                label: 'Título de la Novedad',
                placeholder: 'Ej: Notificación formal de cargos',
                controller: v_4001,
              ),
              const SizedBox(height: 16),
              AzzenTextField(
                label: 'Descripción o Relato de la Actuación',
                placeholder: 'Detalle los fundamentos y actuaciones realizadas...',
                controller: v_4002,
                maxLines: 5,
              ),
              const SizedBox(height: 28),
              AzzenButton(
                label: 'Guardar Actuación en Backend',
                icon: CupertinoIcons.cloud_upload_fill,
                isLoading: v_4004,
                onPressed: m_3001,
              ),
            ],
          ),
        ),
      ),
    );
  }
}