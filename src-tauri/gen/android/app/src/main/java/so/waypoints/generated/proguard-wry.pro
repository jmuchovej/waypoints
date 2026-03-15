# THIS FILE IS AUTO-GENERATED. DO NOT MODIFY!!

# Copyright 2020-2023 Tauri Programme within The Commons Conservancy
# SPDX-License-Identifier: Apache-2.0
# SPDX-License-Identifier: MIT

-keep class so.waypoints.* {
  native <methods>;
}

-keep class so.waypoints.WryActivity {
  public <init>(...);

  void setWebView(so.waypoints.RustWebView);
  java.lang.Class getAppClass(...);
  java.lang.String getVersion();
}

-keep class so.waypoints.Ipc {
  public <init>(...);

  @android.webkit.JavascriptInterface public <methods>;
}

-keep class so.waypoints.RustWebView {
  public <init>(...);

  void loadUrlMainThread(...);
  void loadHTMLMainThread(...);
  void evalScript(...);
}

-keep class so.waypoints.RustWebChromeClient,so.waypoints.RustWebViewClient {
  public <init>(...);
}
