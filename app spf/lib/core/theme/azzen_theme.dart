import 'package:flutter/cupertino.dart';
import 'azzen_colors.dart';
import 'azzen_typography.dart';
class AzzenTheme {
  static CupertinoThemeData get lightTheme {
    return const CupertinoThemeData(
      brightness: Brightness.light,
      primaryColor: AzzenColors.primary,
      primaryContrastingColor: CupertinoColors.white,
      scaffoldBackgroundColor: AzzenColors.background,
      barBackgroundColor: AzzenColors.surfaceSecondary,
      textTheme: CupertinoTextThemeData(
        primaryColor: AzzenColors.textPrimary,
        textStyle: AzzenTypography.body,
        navTitleTextStyle: AzzenTypography.header3,
        navLargeTitleTextStyle: AzzenTypography.header1,
      ),
    );
  }
}