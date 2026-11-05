import 'package:flutter/cupertino.dart';
import 'package:provider/provider.dart';
import '../../../core/config/app_config.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../shared/widgets/azzen_button.dart';
import '../../../shared/widgets/azzen_text_field.dart';
import '../../../shared/widgets/connection_status_bar.dart';
import '../providers/auth_provider.dart';
class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});
  @override
  State<LoginScreen> createState() => clase_2001();
}
class clase_2001 extends State<LoginScreen> {
  final v_4001 = TextEditingController();
  final v_4002 = TextEditingController();
  @override
  void dispose() {
    v_4001.dispose();
    v_4002.dispose();
    super.dispose();
  }
  void m_3001() {
    final serverController = TextEditingController(text: AppConfig.baseUrl);
    bool testing = false;
    String? testResult;
    bool? testSuccess;
    showCupertinoModalPopup(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setModalState) => CupertinoActionSheet(
          title: const Text('Configuración del Servidor'),
          message: Column(
            children: [
              const SizedBox(height: 8),
              const Text(
                'Indique la dirección IP y puerto del backend Actix-Web AZZEN SPF:',
                style: TextStyle(fontSize: 13, color: AzzenColors.textSecondary),
              ),
              const SizedBox(height: 12),
              CupertinoTextField(
                controller: serverController,
                placeholder: 'http:
                style: const TextStyle(fontSize: 14),
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                decoration: BoxDecoration(
                  color: CupertinoColors.white,
                  border: Border.all(color: AzzenColors.border),
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
              const SizedBox(height: 8),
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  CupertinoButton(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                    minimumSize: Size.zero,
                    onPressed: () {
                      setModalState(() {
                        serverController.text = AppConfig.lanServerUrl;
                      });
                    },
                    child: const Text('Wi-Fi LAN', style: TextStyle(fontSize: 12)),
                  ),
                  CupertinoButton(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                    minimumSize: Size.zero,
                    onPressed: () {
                      setModalState(() {
                        serverController.text = AppConfig.emulatorServerUrl;
                      });
                    },
                    child: const Text('Emulador', style: TextStyle(fontSize: 12)),
                  ),
                  CupertinoButton(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                    minimumSize: Size.zero,
                    onPressed: () {
                      setModalState(() {
                        serverController.text = AppConfig.loopbackServerUrl;
                      });
                    },
                    child: const Text('Localhost', style: TextStyle(fontSize: 12)),
                  ),
                ],
              ),
              if (testing) ...[
                const SizedBox(height: 12),
                const CupertinoActivityIndicator(),
                const SizedBox(height: 4),
                const Text('Verificando respuesta del servidor...', style: TextStyle(fontSize: 12)),
              ],
              if (testResult != null) ...[
                const SizedBox(height: 10),
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: testSuccess == true ? const Color(0xFFDCFCE7) : const Color(0xFFFEE2E2),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(
                        testSuccess == true ? CupertinoIcons.check_mark_circled_solid : CupertinoIcons.exclamationmark_circle_fill,
                        size: 16,
                        color: testSuccess == true ? AzzenColors.success : AzzenColors.danger,
                      ),
                      const SizedBox(width: 6),
                      Flexible(
                        child: Text(
                          testResult!,
                          style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w600,
                            color: testSuccess == true ? const Color(0xFF166534) : const Color(0xFF991B1B),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ],
          ),
          actions: [
            CupertinoActionSheetAction(
              onPressed: testing
                  ? () {}
                  : () async {
                      setModalState(() {
                        testing = true;
                        testResult = null;
                      });
                      final ok = await ApiClient.testConnection(serverController.text.trim());
                      setModalState(() {
                        testing = false;
                        testSuccess = ok;
                        testResult = ok
                            ? 'Conexión exitosa con Actix-Web'
                            : 'No se pudo conectar a ${serverController.text.trim()}';
                      });
                    },
              child: const Text('Probar Conexión'),
            ),
            CupertinoActionSheetAction(
              isDefaultAction: true,
              onPressed: () async {
                final newUrl = serverController.text.trim();
                if (newUrl.isNotEmpty) {
                  await AppConfig.setBaseUrl(newUrl);
                  if (mounted) setState(() {});
                }
                if (ctx.mounted) Navigator.pop(ctx);
              },
              child: const Text('Guardar y Aplicar'),
            ),
          ],
          cancelButton: CupertinoActionSheetAction(
            isDestructiveAction: true,
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cerrar'),
          ),
        ),
      ),
    );
  }
  void m_3002() async {
    final cred = v_4001.text.trim();
    final pwd = v_4002.text.trim();
    if (cred.isEmpty || pwd.isEmpty) {
      showCupertinoDialog(
        context: context,
        builder: (ctx) => CupertinoAlertDialog(
          title: const Text('Datos incompletos'),
          content: const Text('Debe ingresar su número de credencial y contraseña institucional.'),
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
    final auth = context.read<AuthProvider>();
    final success = await auth.login(credential: cred, password: pwd);
    if (!mounted) return;
    if (auth.status == AuthStatus.totpRequired) {
      return;
    } else if (auth.status == AuthStatus.devicePending) {
      showCupertinoDialog(
        context: context,
        builder: (ctx) => CupertinoAlertDialog(
          title: const Text('Dispositivo Pendiente'),
          content: Text(auth.errorMessage ?? 'Terminal en espera de autorización del Centro de Seguridad.'),
          actions: [
            CupertinoDialogAction(
              child: const Text('Entendido'),
              onPressed: () => Navigator.pop(ctx),
            ),
          ],
        ),
      );
    } else if (!success && auth.errorMessage != null) {
      showCupertinoDialog(
        context: context,
        builder: (ctx) => CupertinoAlertDialog(
          title: const Text('Acceso Denegado'),
          content: Text(auth.errorMessage!),
          actions: [
            CupertinoDialogAction(
              child: const Text('Reintentar'),
              onPressed: () => Navigator.pop(ctx),
            ),
          ],
        ),
      );
    }
  }
  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthProvider>();
    return CupertinoPageScaffold(
      backgroundColor: AzzenColors.background,
      child: SafeArea(
        child: Column(
          children: [
            const ConnectionStatusBar(),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  CupertinoButton(
                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                    minimumSize: Size.zero,
                    color: AzzenColors.surfaceSecondary,
                    borderRadius: BorderRadius.circular(14),
                    onPressed: m_3001,
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Icon(CupertinoIcons.gear_alt, size: 14, color: AzzenColors.textSecondary),
                        const SizedBox(width: 4),
                        Text(
                          AppConfig.baseUrl.replaceAll('http:
                          style: const TextStyle(fontSize: 11, color: AzzenColors.textSecondary, fontWeight: FontWeight.w600),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            Expanded(
              child: Center(
                child: SingleChildScrollView(
                  padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 400),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Center(
                          child: Container(
                            width: 88,
                            height: 88,
                            decoration: BoxDecoration(
                              borderRadius: BorderRadius.circular(18),
                              boxShadow: const [
                                BoxShadow(
                                  color: Color(0x14000000),
                                  blurRadius: 12,
                                  offset: Offset(0, 4),
                                ),
                              ],
                            ),
                            clipBehavior: Clip.antiAlias,
                            child: Image.asset(
                              'assets/images/azzen_spf_app.png',
                              width: 88,
                              height: 88,
                              fit: BoxFit.contain,
                            ),
                          ),
                        ),
                        const SizedBox(height: 16),
                        const Text(
                          'AZZEN SPF',
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            fontSize: 26,
                            fontWeight: FontWeight.w800,
                            letterSpacing: 1.2,
                            color: AzzenColors.navy,
                          ),
                        ),
                        const SizedBox(height: 4),
                        const Text(
                          'SERVICIO PENITENCIARIO FEDERAL\nSISTEMA INTEGRADO DE GESTIÓN Y CONTROL',
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w600,
                            letterSpacing: 0.5,
                            color: AzzenColors.textSecondary,
                            height: 1.4,
                          ),
                        ),
                        const SizedBox(height: 36),
                        AzzenTextField(
                          label: 'Número de Credencial',
                          placeholder: 'Ej: 99999 o 900001',
                          controller: v_4001,
                          keyboardType: TextInputType.text,
                          prefixIcon: CupertinoIcons.person_badge_minus,
                        ),
                        const SizedBox(height: 16),
                        AzzenTextField(
                          label: 'Contraseña Institucional',
                          placeholder: 'Ingrese su contraseña',
                          controller: v_4002,
                          isPassword: true,
                          prefixIcon: CupertinoIcons.lock,
                        ),
                        const SizedBox(height: 24),
                        AzzenButton(
                          label: 'Ingresar al Sistema',
                          icon: CupertinoIcons.arrow_right_circle,
                          isLoading: auth.status == AuthStatus.authenticating,
                          onPressed: m_3002,
                        ),
                        if (auth.isBiometricSupported && auth.isBiometricEnabled) ...[
                          const SizedBox(height: 12),
                          AzzenButton(
                            label: 'Desbloquear con Biometría',
                            icon: CupertinoIcons.viewfinder,
                            variant: AzzenButtonVariant.secondary,
                            onPressed: () async {
                              final ok = await auth.unlockWithBiometrics();
                              if (!context.mounted) return;
                              if (!ok && auth.errorMessage != null) {
                                showCupertinoDialog(
                                  context: context,
                                  builder: (ctx) => CupertinoAlertDialog(
                                    title: const Text('Biometría'),
                                    content: Text(auth.errorMessage!),
                                    actions: [
                                      CupertinoDialogAction(
                                        child: const Text('Aceptar'),
                                        onPressed: () => Navigator.pop(ctx),
                                      ),
                                    ],
                                  ),
                                );
                              }
                            },
                          ),
                        ],
                        const SizedBox(height: 32),
                        const Text(
                          'Acceso restringido a personal autorizado.\nTodas las operaciones y consultas son auditadas conforme a normativa institucional.',
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            fontSize: 11,
                            color: AzzenColors.textMuted,
                            height: 1.4,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}