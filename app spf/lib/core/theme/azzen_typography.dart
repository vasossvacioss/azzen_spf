import 'package:flutter/cupertino.dart';
import 'azzen_colors.dart';
class AzzenTypography {
  static const TextStyle header1 = TextStyle(
    fontSize: 24,
    fontWeight: FontWeight.w700,
    color: AzzenColors.textPrimary,
    letterSpacing: -0.5,
  );
  static const TextStyle header2 = TextStyle(
    fontSize: 20,
    fontWeight: FontWeight.w600,
    color: AzzenColors.textPrimary,
    letterSpacing: -0.3,
  );
  static const TextStyle header3 = TextStyle(
    fontSize: 17,
    fontWeight: FontWeight.w600,
    color: AzzenColors.textPrimary,
  );
  static const TextStyle body = TextStyle(
    fontSize: 15,
    fontWeight: FontWeight.w400,
    color: AzzenColors.textPrimary,
    height: 1.4,
  );
  static const TextStyle bodyBold = TextStyle(
    fontSize: 15,
    fontWeight: FontWeight.w600,
    color: AzzenColors.textPrimary,
  );
  static const TextStyle caption = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w400,
    color: AzzenColors.textSecondary,
  );
  static const TextStyle captionBold = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w600,
    color: AzzenColors.textSecondary,
  );
  static const TextStyle badge = TextStyle(
    fontSize: 12,
    fontWeight: FontWeight.w600,
    letterSpacing: 0.2,
  );
}