import 'package:flutter/cupertino.dart';
import 'package:provider/provider.dart';
import 'core/config/app_config.dart';
import 'core/theme/azzen_theme.dart';
import 'features/auth/providers/auth_provider.dart';
import 'features/auth/screens/login_screen.dart';
import 'features/auth/screens/totp_screen.dart';
import 'features/dashboard/screens/dashboard_screen.dart';
void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await AppConfig.init();
  runApp(const AzzenSpfApp());
}
class AzzenSpfApp extends StatelessWidget {
  const AzzenSpfApp({super.key});
  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => AuthProvider()),
      ],
      child: CupertinoApp(
        title: 'AZZEN SPF Mobile',
        debugShowCheckedModeBanner: false,
        theme: AzzenTheme.lightTheme,
        home: const AuthGate(),
      ),
    );
  }
}
class AuthGate extends StatelessWidget {
  const AuthGate({super.key});
  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthProvider>();
    switch (auth.status) {
      case AuthStatus.authenticated:
        return const DashboardScreen();
      case AuthStatus.totpRequired:
        return const TotpScreen();
      default:
        return const LoginScreen();
    }
  }
}