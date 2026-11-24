package com.taitidelabs.hg.player

import android.content.Context
import android.os.Looper
import android.view.TextureView
import androidx.annotation.OptIn
import androidx.media3.common.MediaItem
import androidx.media3.common.MimeTypes
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.hls.HlsMediaSource

internal class ReusablePool<T>(
    maxCapacity: Int,
    private val create: () -> T,
    private val reset: (T) -> Unit,
    private val dispose: (T) -> Unit,
) {
    val maxCapacity: Int = maxCapacity.coerceAtLeast(0)
    private val idle = ArrayDeque<T>()

    val idleCount: Int
        get() = idle.size

    fun acquire(): T {
        val item = if (idle.isEmpty()) create() else idle.removeLast()
        reset(item)
        return item
    }

    fun release(item: T): Boolean {
        reset(item)
        if (idle.contains(item)) {
            return true
        }
        return if (idle.size < maxCapacity) {
            idle.addLast(item)
            true
        } else {
            dispose(item)
            false
        }
    }

    fun clearAll() {
        while (idle.isNotEmpty()) {
            val item = idle.removeFirst()
            dispose(item)
        }
    }
}

@OptIn(UnstableApi::class)
class HgVideoPlayerPool(
    context: Context,
    maxCapacity: Int = DEFAULT_CAPACITY,
) {
    private val applicationContext: Context = context.applicationContext
    private val httpDataSourceFactory = DefaultHttpDataSource.Factory()
        .setAllowCrossProtocolRedirects(true)
    private val hlsMediaSourceFactory = HlsMediaSource.Factory(httpDataSourceFactory)
        .setAllowChunklessPreparation(true)

    private val store = ReusablePool(
        maxCapacity = maxCapacity,
        create = {
            ExoPlayer.Builder(applicationContext)
                .setMediaSourceFactory(hlsMediaSourceFactory)
                .build()
        },
        reset = { player -> resetPlayer(player, textureView = null) },
        dispose = { player ->
            player.stop()
            player.clearMediaItems()
            player.release()
        },
    )

    val maxCapacity: Int
        get() = store.maxCapacity

    val idleCount: Int
        get() = store.idleCount

    fun acquire(): ExoPlayer {
        requireMainThread()
        return store.acquire()
    }

    fun release(player: ExoPlayer, textureView: TextureView? = null) {
        requireMainThread()
        resetPlayer(player, textureView)
        store.release(player)
    }

    fun createHlsMediaSource(uri: String): HlsMediaSource {
        val mediaItem = MediaItem.Builder()
            .setUri(uri)
            .setMimeType(MimeTypes.APPLICATION_M3U8)
            .build()
        return hlsMediaSourceFactory.createMediaSource(mediaItem)
    }

    fun clearAll() {
        requireMainThread()
        store.clearAll()
    }

    private fun resetPlayer(player: ExoPlayer, textureView: TextureView?) {
        player.playWhenReady = false
        player.stop()
        player.clearMediaItems()
        if (textureView != null) {
            player.clearVideoTextureView(textureView)
        } else {
            player.clearVideoSurface()
        }
        player.volume = HgVideoPlayerProtocol.DEFAULT_VOLUME.toFloat()
        player.setPlaybackSpeed(HgVideoPlayerProtocol.DEFAULT_RATE.toFloat())
    }

    private fun requireMainThread() {
        check(Looper.myLooper() == Looper.getMainLooper()) {
            "HgVideoPlayerPool must be used on the Android main thread"
        }
    }

    companion object {
        const val DEFAULT_CAPACITY = 3
    }
}
