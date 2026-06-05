package cn.duitaofang.merchant

import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.activity.enableEdgeToEdge
import cn.jpush.android.api.JPushInterface
import org.json.JSONObject
import java.io.File

class MainActivity : TauriActivity() {
  private val handler = Handler(Looper.getMainLooper())
  private var lastAlias = ""
  private var seq = 1

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // 极光推送初始化（自有通道 + 厂商通道走系统级，app 关了也能从顶部弹）
    JPushInterface.setDebugMode(true)
    JPushInterface.init(this)

    // 安卓 13+ 需要运行时通知权限，否则不弹
    if (Build.VERSION.SDK_INT >= 33 &&
      checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) {
      requestPermissions(arrayOf(android.Manifest.permission.POST_NOTIFICATIONS), 1001)
    }

    scheduleAliasSync()
  }

  // 周期性读 webview(经 Rust set_push_creds)写下的 push_creds.json 拿当前商户 mid → setAlias("m"+mid)。
  // 后端有新排队/新消息时按 alias "m"+mid 推送，这台设备就收到顶部通知。
  private fun scheduleAliasSync() {
    handler.postDelayed({
      syncAlias()
      scheduleAliasSync()
    }, 5000)
  }

  private fun syncAlias() {
    try {
      val mid = readMid() ?: return
      val alias = "m$mid"
      if (alias != lastAlias) {
        JPushInterface.setAlias(applicationContext, seq++, alias)
        lastAlias = alias
      }
    } catch (e: Exception) {
    }
  }

  private fun readMid(): String? {
    val dirs = listOf(filesDir, cacheDir, File(dataDir, "files"), dataDir)
    for (d in dirs) {
      val f = File(d, "push_creds.json")
      if (f.exists()) {
        try {
          val m = JSONObject(f.readText()).optString("mid")
          if (m.isNotEmpty()) return m
        } catch (e: Exception) {
        }
      }
    }
    return null
  }
}
