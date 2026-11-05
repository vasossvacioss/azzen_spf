import 'package:flutter/cupertino.dart';
import '../../core/networking/api_client.dart';
import '../../core/theme/azzen_colors.dart';
import '../../core/theme/azzen_typography.dart';
class ConnectionStatusBar extends StatelessWidget {
  const ConnectionStatusBar({super.key});
  @override
  Widget build(BuildContext context) {
    if (ApiClient.connectionStatus == ConnectionStateStatus.connected) {
      return const SizedBox.shrink();
    }
    final isDisconnected = ApiClient.connectionStatus == ConnectionStateStatus.disconnected;
    return Container(
      width: double.infinity,
      color: isDisconnected ? AzzenColors.danger : AzzenColors.warning,
      padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 16),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            isDisconnected ? CupertinoIcons.wifi_slash : CupertinoIcons.exclamationmark_triangle,
            color: CupertinoColors.white,
            size: 16,
          ),
          const SizedBox(width: 8),
          Text(
            isDisconnected
                ? 'Sin conexión con el backend central AZZEN SPF'
                : 'Conexión inestable con el servidor',
            style: AzzenTypography.captionBold.copyWith(color: CupertinoColors.white),
          ),
        ],
      ),
    );
  }
}