package com.taitidelabs.hg.player

import android.graphics.Color
import android.graphics.Matrix
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.TextureView
import android.view.View
import android.widget.FrameLayout
import androidx.annotation.OptIn
import androidx.media3.common.C
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.VideoSize
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import com.longbridge.nsh.NshComponent
import com.longbridge.nsh.NshComponentContext

@OptIn(UnstableApi::class)
class HgVideoPlayerComponent(
    private val context: NshComponentContext,
    private val pool: HgVideoPlayerPool,
) : NshComponent {
    private val container = FrameLayout(context.getContext()).apply {
        setBackgroundColor(Color.BLACK)
        clipChildren = true
        clipToPadding = true
    }

    private val textureView = TextureView(context.getContext())
    private val props = VideoPlayerProps()
    private val mainHandler = Handler(Looper.getMainLooper())
    private val transformMatrix = Matrix()

    private var player: ExoPlayer? = pool.acquire()
    private var videoWidth: Int = 0
    private var videoHeight: Int = 0
    private var pendingSeekMs: Long? = null
    private var endedEmitted: Boolean = false
    private var destroyed: Boolean = false

    private val progressTicker = object : Runnable {
        override fun run() {
            if (destroyed || props.paused || props.src == null || endedEmitted) {
                return
            }
            emitProgressSnapshot()
            mainHandler.postDelayed(this, HgVideoPlayerProtocol.PROGRESS_INTERVAL_MS)
        }
    }

    private val playerListener = object : Player.Listener {
        override fun onPlaybackStateChanged(playbackState: Int) {
            if (destroyed) return
            when (playbackState) {
                Player.STATE_READY -> {
                    emitProgressSnapshot()
                    syncProgressTicker()
                }
                Player.STATE_ENDED -> {
                    stopProgressTicker()
                    if (!endedEmitted && props.src != null) {
                        endedEmitted = true
                        context.emit(HgVideoPlayerProtocol.EVENT_ENDED, null)
                    }
                }
                else -> Unit
            }
        }

        override fun onPlayerError(error: PlaybackException) {
            if (destroyed) return
            stopProgressTicker()
            val message = error.localizedMessage?.takeIf { it.isNotBlank() } ?: error.errorCodeName
            context.emit(
                HgVideoPlayerProtocol.EVENT_ERROR,
                HgVideoPlayerProtocol.errorPayload(message),
            )
        }

        override fun onVideoSizeChanged(videoSize: VideoSize) {
            if (destroyed) return
            videoWidth = videoSize.width
            videoHeight = videoSize.height
            applyAspectTransform()
        }
    }

    private val layoutListener = View.OnLayoutChangeListener { _, left, top, right, bottom, oldLeft, oldTop, oldRight, oldBottom ->
        val sizeChanged = (right - left) != (oldRight - oldLeft) || (bottom - top) != (oldBottom - oldTop)
        if (sizeChanged && !destroyed) {
            applyAspectTransform()
        }
    }

    private val attachStateListener = object : View.OnAttachStateChangeListener {
        override fun onViewAttachedToWindow(v: View) {
            if (!destroyed) {
                player?.playWhenReady = !props.paused
                syncProgressTicker()
            }
        }

        override fun onViewDetachedFromWindow(v: View) {
            stopProgressTicker()
            player?.playWhenReady = false
        }
    }

    init {
        container.addView(
            textureView,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT,
                Gravity.CENTER,
            ),
        )
        container.addOnLayoutChangeListener(layoutListener)
        container.addOnAttachStateChangeListener(attachStateListener)
        player?.let { exoPlayer ->
            exoPlayer.addListener(playerListener)
            exoPlayer.setVideoTextureView(textureView)
            exoPlayer.volume = props.effectiveVolume()
            exoPlayer.setPlaybackSpeed(props.rate.toFloat())
            exoPlayer.playWhenReady = !props.paused
        }
    }

    override fun view(): View = container

    override fun setProp(name: String, value: Any?) {
        if (destroyed) return
        val change = props.applyProp(name, value)
        val exoPlayer = player ?: return

        when (change) {
            PropChange.Unchanged -> Unit
            PropChange.Source -> applySource(exoPlayer)
            PropChange.Paused -> {
                exoPlayer.playWhenReady = !props.paused
                syncProgressTicker()
            }
            PropChange.Rate -> exoPlayer.setPlaybackSpeed(props.rate.toFloat())
            PropChange.Volume, PropChange.Muted -> exoPlayer.volume = props.effectiveVolume()
            is PropChange.Seek -> {
                endedEmitted = false
                val targetMs = change.positionSecs * 1000L
                if (props.src != null) {
                    exoPlayer.seekTo(targetMs)
                    emitProgressSnapshot()
                    syncProgressTicker()
                } else {
                    pendingSeekMs = targetMs
                }
            }
            PropChange.Aspect -> applyAspectTransform()
        }
    }

    override fun destroy() {
        if (destroyed) return
        destroyed = true
        stopProgressTicker()
        container.removeOnAttachStateChangeListener(attachStateListener)
        container.removeOnLayoutChangeListener(layoutListener)
        val activePlayer = player
        player = null
        if (activePlayer != null) {
            activePlayer.removeListener(playerListener)
            pool.release(activePlayer, textureView)
        }
        container.removeAllViews()
    }

    private fun applySource(exoPlayer: ExoPlayer) {
        stopProgressTicker()
        endedEmitted = false
        videoWidth = 0
        videoHeight = 0

        val src = props.src
        if (src == null) {
            pendingSeekMs = null
            exoPlayer.playWhenReady = false
            exoPlayer.stop()
            exoPlayer.clearMediaItems()
            return
        }

        exoPlayer.setMediaSource(pool.createHlsMediaSource(src))
        exoPlayer.volume = props.effectiveVolume()
        exoPlayer.setPlaybackSpeed(props.rate.toFloat())
        exoPlayer.playWhenReady = !props.paused
        val initialSeek = pendingSeekMs
        pendingSeekMs = null
        if (initialSeek != null && initialSeek > 0L) {
            exoPlayer.seekTo(initialSeek)
        }
        exoPlayer.prepare()
        syncProgressTicker()
    }

    private fun applyAspectTransform() {
        val transform = HgVideoPlayerProtocol.computeTransform(
            viewWidth = container.width,
            viewHeight = container.height,
            videoWidth = videoWidth,
            videoHeight = videoHeight,
            aspect = props.aspect,
        )
        transformMatrix.reset()
        transformMatrix.setScale(
            transform.scaleX,
            transform.scaleY,
            transform.pivotX,
            transform.pivotY,
        )
        textureView.setTransform(transformMatrix)
    }

    private fun syncProgressTicker() {
        stopProgressTicker()
        if (!destroyed && !props.paused && props.src != null && !endedEmitted) {
            mainHandler.postDelayed(progressTicker, HgVideoPlayerProtocol.PROGRESS_INTERVAL_MS)
        }
    }

    private fun stopProgressTicker() {
        mainHandler.removeCallbacks(progressTicker)
    }

    private fun emitProgressSnapshot() {
        val exoPlayer = player ?: return
        val positionSecs = (exoPlayer.currentPosition.coerceAtLeast(0L)) / 1000L
        val rawDuration = exoPlayer.duration
        val durationSecs = if (rawDuration == C.TIME_UNSET || rawDuration < 0L) {
            0L
        } else {
            rawDuration / 1000L
        }
        context.emit(
            HgVideoPlayerProtocol.EVENT_PROGRESS,
            HgVideoPlayerProtocol.progressPayload(positionSecs, durationSecs),
        )
    }
}
