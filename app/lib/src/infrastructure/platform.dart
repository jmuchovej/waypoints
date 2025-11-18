import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:universal_html/html.dart' show window;
import 'package:universal_platform/universal_platform.dart';

class CurrentPlatform {
  /// Whether or not we are running on an Apple device (MacOS or iOS).
  static bool get isApple =>
      defaultTargetPlatform == TargetPlatform.macOS ||
      defaultTargetPlatform == TargetPlatform.iOS;

  static bool get isMobile =>
      defaultTargetPlatform == TargetPlatform.android ||
      defaultTargetPlatform == TargetPlatform.iOS;

  /// Whether or not we are running on web.
  ///
  /// By default this is the same as [kIsWeb].
  ///
  /// [debugIsWebOverride] may be used to override the natural value of [isWeb].
  static bool get isWeb =>
      debugIsWebOverride ==
          null //
      ? kIsWeb
      : debugIsWebOverride == WebPlatformOverride.web;
}

/// Overrides the value of [CurrentPlatform.isWeb].
///
/// This is intended to be used in tests.
///
/// Set it to `null` to use the default value of [CurrentPlatform.isWeb].
///
/// Set it to [WebPlatformOverride.web] to configure to run as if we are on web.
///
/// Set it to [WebPlatformOverride.native] to configure to run as if we are NOT on web.
@visibleForTesting
WebPlatformOverride? debugIsWebOverride;

@visibleForTesting
enum WebPlatformOverride {
  /// Configuration to run the app as if we are a native app.
  native,

  /// Configuration to run the app as if we are on web.
  web,
}

extension PlatformExtension on Platform {
  static String get _webPlatform =>
      window.navigator.platform?.toLowerCase() ?? '';

  /// Returns true if the operating system is macOS and not running on Web platform.
  static bool get isMacOS => UniversalPlatform.isMacOS;

  /// Returns true if the operating system is Windows and not running on Web platform.
  static bool get isWindows => UniversalPlatform.isWindows;

  /// Returns true if the operating system is Linux and not running on Web platform.
  static bool get isLinux => UniversalPlatform.isLinux;

  /// Returns true if the operating system is iOS and not running on Web platform.
  static bool get isIOS => UniversalPlatform.isIOS;

  /// Returns true if the operating system is Android and not running on Web platform.
  static bool get isAndroid => UniversalPlatform.isAndroid;

  /// Returns true if the operating system is macOS and running on Web platform.
  static bool get isWebOnMacOS {
    if (!kIsWeb) {
      return false;
    }
    return _webPlatform.contains('mac') == true;
  }

  /// Returns true if the operating system is Windows and running on Web platform.
  static bool get isWebOnWindows {
    if (!kIsWeb) {
      return false;
    }
    return _webPlatform.contains('windows') == true;
  }

  /// Returns true if the operating system is Linux and running on Web platform.
  static bool get isWebOnLinux {
    if (!kIsWeb) {
      return false;
    }
    return _webPlatform.contains('linux') == true;
  }

  static bool get isDesktopOrWeb =>
      UniversalPlatform.isWeb || UniversalPlatform.isDesktop;

  static bool get isDesktop => UniversalPlatform.isDesktop;

  static bool get isMobile => UniversalPlatform.isMobile;

  static bool get isNotMobile => !isMobile;
}
