package com.taitidelabs.hg.player

import android.content.Context
import android.os.Handler
import android.os.Looper
import com.longbridge.nsh.NshComponent
import com.longbridge.nsh.NshView

object HgVideoPlayerRegistration {
    @Volatile
    private var sharedPool: HgVideoPlayerPool? = null

    @JvmStatic
    @JvmOverloads
    fun register(
        nshView: NshView,
        poolCapacity: Int = HgVideoPlayerPool.DEFAULT_CAPACITY,
    ) {
        val pool = getOrCreatePool(nshView.context.applicationContext, poolCapacity)
        nshView.registerComponent(
            HgVideoPlayerProtocol.COMPONENT_NAME,
            NshComponent.Factory { componentContext ->
                HgVideoPlayerComponent(componentContext, pool)
            },
        )
    }

    @JvmStatic
    fun releasePool() {
        val pool = sharedPool ?: return
        sharedPool = null
        if (Looper.myLooper() == Looper.getMainLooper()) {
            pool.clearAll()
        } else {
            Handler(Looper.getMainLooper()).post {
                pool.clearAll()
            }
        }
    }

    private fun getOrCreatePool(appContext: Context, poolCapacity: Int): HgVideoPlayerPool {
        val existing = sharedPool
        if (existing != null) {
            return existing
        }
        val created = HgVideoPlayerPool(appContext, poolCapacity)
        sharedPool = created
        return created
    }
}
