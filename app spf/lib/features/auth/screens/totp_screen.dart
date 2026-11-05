import 'package:flutter/cupertino.dart';
import 'package:provider/provider.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_button.dart';
import '../../../shared/widgets/azzen_text_field.dart';
import '../providers/auth_provider.dart';
class TotpScreen extends StatefulWidget {
  const TotpScreen({super.key});
  @override
  State<TotpScreen> createState() => clase_2001();
}
class clase_2001 extends State<TotpScreen> {
  final v_4001 = TextEditingController();
  @override
  void dispose() {
    v_4001.dispose();
    super.dispose();
  }
  void m_3001() async {
    final code = v_4001.text.trim();
    if (code.isEmpty) return;
    final auth = context.read<AuthProvider>();
    final success = await auth.verifyTotp(code);
    if (!mounted) return;
    if (success) {
      if (Navigator.canPop(context)) {
        Navigator.pop(context);
      }
    } else if (auth.errorMessage != null) {
      showCupertinoDialog(
        context: context,
        builder: (ctx) => CupertinoAlertDialog(
          title: const Text('Código Inválido'),
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
      navigationBar: const CupertinoNavigationBar(
        middle: Text('Segundo Factor TOTP'),
      ),
      child: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 400),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Center(
                    child: Container(
                      width: 64,
                      height: 64,
                      decoration: BoxDecoration(
                        color: AzzenColors.primaryLight,
                        borderRadius: BorderRadius.circular(16),
                      ),
                      child: const Icon(
                        CupertinoIcons.device_phone_portrait,
                        size: 36,
                        color: AzzenColors.primary,
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  const Text(
                    'Verificación de Seguridad',
                    textAlign: TextAlign.center,
                    style: AzzenTypography.header2,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Ingrese el código temporal de 6 dígitos generado por su aplicación autenticadora (Google Authenticator) para la credencial ${auth.pendingCredential ?? ""}.',
                    textAlign: TextAlign.center,
                    style: AzzenTypography.caption,
                  ),
                  const SizedBox(height: 32),
                  AzzenTextField(
                    label: 'Código de Verificación (o Código de Respaldo)',
                    placeholder: '000000',
                    controller: v_4001,
                    keyboardType: TextInputType.text,
                    prefixIcon: CupertinoIcons.lock_shield,
                  ),
                  const SizedBox(height: 24),
                  AzzenButton(
                    label: 'Verificar y Acceder',
                    icon: CupertinoIcons.checkmark_shield,
                    isLoading: auth.status == AuthStatus.authenticating,
                    onPressed: m_3001,
                  ),
                  const SizedBox(height: 16),
                  CupertinoButton(
                    child: const Text(
                      'Cancelar e Iniciar con otra cuenta',
                      style: TextStyle(color: AzzenColors.textSecondary, fontSize: 13),
                    ),
                    onPressed: () {
                      auth.cancelTotp();
                      if (Navigator.canPop(context)) {
                        Navigator.pop(context);
                      }
                    },
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}