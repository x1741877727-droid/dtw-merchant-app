# THIS FILE IS AUTO-GENERATED. DO NOT MODIFY!!

# Copyright 2020-2023 Tauri Programme within The Commons Conservancy
# SPDX-License-Identifier: Apache-2.0
# SPDX-License-Identifier: MIT

-keep class cn.duitaofang.merchant.* {
  native <methods>;
}

-keep class cn.duitaofang.merchant.WryActivity {
  public <init>(...);

  void setWebView(cn.duitaofang.merchant.RustWebView);
  java.lang.Class getAppClass(...);
  int getId();
  java.lang.String getVersion();
  int startActivity(...);
}

-keep class cn.duitaofang.merchant.Ipc {
  public <init>(...);

  @android.webkit.JavascriptInterface public <methods>;
}

-keep class cn.duitaofang.merchant.RustWebView {
  public <init>(...);

  void loadUrlMainThread(...);
  void loadHTMLMainThread(...);
  void evalScript(...);
}

-keep class cn.duitaofang.merchant.RustWebChromeClient,cn.duitaofang.merchant.RustWebViewClient {
  public <init>(...);
}
